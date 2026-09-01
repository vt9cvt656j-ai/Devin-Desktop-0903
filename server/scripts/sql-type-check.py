#!/usr/bin/env python3
"""SQL 的返回类型 和 Rust 的解码类型 对不对得上。

**这一类错编译器一个字都看不见，而且在空表上也看不见。** sqlx 的运行时解码要求列的
PG 类型和 Rust 类型严格匹配，可这两边写在不同的地方：SQL 在字符串字面量里，类型在
`let x: Vec<(f64, ...)>` 上。对不上时唯一的表现是**接口 500**，而且只在**查询真的
返回了行**的时候 —— 数据攒够之前它一直是绿的。

线上抓到的：`/api/admin/plan-health` 的
    EXTRACT(epoch FROM (s.hi - s.lo)) / 3600.0
PostgreSQL 14 起 EXTRACT 回 **numeric**，除以 3600.0 还是 numeric，而 Rust 那边声明的是
`f64` → 解码失败 → 整页 500。它此前一直没暴露，是因为余额探针 2026-08-25 才上线，
在那之前这条查询恒返回 0 行、走 `None` 分支。同一形状的 `sum(bigint)` → numeric 在
总览页上也炸过一次。

做法：把每条查询 `PREPARE` 到真库上（不执行、不写库），从 `pg_prepared_statements`
读回 `result_types`，和 Rust 声明的元组逐列比。需要一个能连的库；CI 里连测试库即可。

用法：
    python3 scripts/sql-type-check.py --emit-prepare > /tmp/prep.sql
    psql ... -f /tmp/prep.sql > /tmp/prep.out
    python3 scripts/sql-type-check.py --compare /tmp/prep.out
"""
import re, os, json, sys, glob

SRC = os.environ.get("SQLCHECK_SRC") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src")

def read_string_literal(s, i):
    """i 指向起始引号。返回 (内容, 结束下标+1)。处理 \\ 续行与转义。"""
    assert s[i] == '"'
    out = []; i += 1
    while i < len(s):
        c = s[i]
        if c == '\\':
            n = s[i+1] if i+1 < len(s) else ''
            if n == '\n':                      # Rust 续行：吃掉换行和后续缩进
                i += 2
                while i < len(s) and s[i] in ' \t': i += 1
                continue
            out.append({'n':'\n','t':'\t','r':'\r','"':'"','\\':'\\','0':'\0'}.get(n, n))
            i += 2; continue
        if c == '"':
            return "".join(out), i+1
        out.append(c); i += 1
    return "".join(out), i

def grab_sql_after(s, i):
    """从 i 起找到第一个字符串字面量（可能是多个用 + 拼接的）。"""
    depth_guard = 0
    while i < len(s) and depth_guard < 4000:
        c = s[i]
        if c == '"':
            lit, j = read_string_literal(s, i)
            # 支持 "a" + "b" 形式的拼接
            k = j
            while True:
                m = re.match(r'\s*\+\s*"', s[k:])
                if not m: break
                more, k2 = read_string_literal(s, k + m.end() - 1)
                lit += more; k = k2
            return lit, k
        if c in ');' and depth_guard > 0:
            return None, i
        if not c.isspace(): depth_guard += 1
        i += 1
    return None, i

def split_tuple(t):
    """拆 (A, B, Option<C>) 顶层逗号。"""
    out=[]; d=0; cur=""
    for ch in t:
        if ch in '<([': d+=1
        elif ch in '>)]': d-=1
        if ch==',' and d==0: out.append(cur.strip()); cur=""
        else: cur+=ch
    if cur.strip(): out.append(cur.strip())
    return out

# 结构体字段类型（用于 query_as::<_, Struct>）
def collect_structs():
    structs={}
    for f in glob.glob(os.path.join(SRC,"*.rs")):
        src=open(f,encoding="utf8").read()
        for m in re.finditer(r'struct\s+(\w+)\s*\{([^}]*)\}', src):
            name=m.group(1); body=m.group(2)
            fields=[]
            for fm in re.finditer(r'(?:pub\s+)?(\w+)\s*:\s*([^,\n]+)', body):
                fields.append((fm.group(1), fm.group(2).strip()))
            if fields: structs.setdefault((os.path.basename(f),name), fields)
    return structs

ALIAS = re.compile(r'type\s+(\w+)\s*=\s*\(([^;]*?)\)\s*;', re.S)

PATTERNS = [
    # let x: Vec<(..)> = sqlx::query_as(
    re.compile(r'let\s+\w+\s*:\s*Vec<\(([^;]*?)\)>\s*=\s*sqlx::query_as\s*\('),
    re.compile(r'let\s+\w+\s*:\s*Option<\(([^;]*?)\)>\s*=\s*sqlx::query_as\s*\('),
]
SCALAR = re.compile(r'let\s+\w+\s*:\s*([A-Za-z0-9_:<>, ]+?)\s*=\s*sqlx::query_scalar\s*\(')
SCALAR_T = re.compile(r'sqlx::query_scalar::<\s*_\s*,\s*([A-Za-z0-9_:<>, ]+?)\s*>\s*\(')
AS_STRUCT = re.compile(r'let\s+\w+\s*:\s*(?:Vec|Option)<\s*([A-Za-z_]\w*)\s*>\s*=\s*sqlx::query_as\s*\(')

def main():
    structs = collect_structs()
    items=[]
    for f in sorted(glob.glob(os.path.join(SRC,"*.rs"))):
        base=os.path.basename(f)
        src=open(f,encoding="utf8").read()
        # 去掉 #[cfg(test)] 之后的内容（测试里有大量假 SQL）
        ti = src.find("\n#[cfg(test)]")
        body = src[:ti] if ti>0 else src
        def line_of(pos): return body.count("\n",0,pos)+1
        for pat in PATTERNS:
            for m in pat.finditer(body):
                types = split_tuple(m.group(1))
                sql,_ = grab_sql_after(body, m.end())
                if sql and 'SELECT' in sql.upper():
                    items.append(dict(file=base, line=line_of(m.start()), kind="tuple", types=types, sql=sql))
        for m in SCALAR.finditer(body):
            t=m.group(1).strip()
            sql,_ = grab_sql_after(body, m.end())
            if sql and 'SELECT' in sql.upper():
                items.append(dict(file=base, line=line_of(m.start()), kind="scalar", types=[t], sql=sql))
        for m in SCALAR_T.finditer(body):
            t=m.group(1).strip()
            sql,_ = grab_sql_after(body, m.end())
            if sql and 'SELECT' in sql.upper():
                items.append(dict(file=base, line=line_of(m.start()), kind="scalar", types=[t], sql=sql))
        aliases={am.group(1): split_tuple(am.group(2)) for am in ALIAS.finditer(body)}
        for m in AS_STRUCT.finditer(body):
            sname=m.group(1)
            if sname in aliases:
                sql,_ = grab_sql_after(body, m.end())
                if sql and 'SELECT' in sql.upper():
                    items.append(dict(file=base, line=line_of(m.start()), kind="alias:"+sname,
                                      types=aliases[sname], sql=sql))
                continue
            if (base,sname) not in structs: continue
            sql,_ = grab_sql_after(body, m.end())
            if sql and 'SELECT' in sql.upper() and 'SELECT *' not in sql.upper():
                items.append(dict(file=base, line=line_of(m.start()), kind="struct:"+sname,
                                  types=[t for _,t in structs[(base,sname)]], sql=sql))
    return items


OK = {
 "i64": {"bigint", "integer", "smallint"}, "i32": {"integer", "smallint"}, "i16": {"smallint"},
 "f64": {"double precision", "real"}, "f32": {"real"}, "bool": {"boolean"},
 "String": {"text", "character varying", "name", "character", "citext"},
 "Uuid": {"uuid"}, "uuid::Uuid": {"uuid"},
 "serde_json::Value": {"json", "jsonb"}, "Value": {"json", "jsonb"},
 "chrono::DateTime<chrono::Utc>": {"timestamp with time zone"},
 "DateTime<Utc>": {"timestamp with time zone"},
 "chrono::DateTime<Utc>": {"timestamp with time zone"},
 "chrono::NaiveDate": {"date"}, "NaiveDate": {"date"},
 "chrono::NaiveDateTime": {"timestamp without time zone"},
 "Vec<String>": {"text[]", "text"},
 "sqlx::types::BigDecimal": {"numeric"}, "rust_decimal::Decimal": {"numeric"},
}


def unwrap(t):
    t = t.strip()
    while t.startswith("Option<") and t.endswith(">"):
        t = t[7:-1].strip()
    return t


def compare(items, out_path):
    res = {}
    # PREPARE 失败的行长这样：`psql:<stdin>:50: ERROR:  column "x" does not exist`
    # 行号对应 emit-prepare 的输出，第 1 行是 \set，所以 PREPARE pN 在第 N+2 行。
    errs = {}
    for line in open(out_path, encoding="utf8", errors="replace"):
        m = re.match(r'\s*(p\d+)\s*\|\s*\{(.*)\}\s*$', line)
        if m:
            res[m.group(1)] = [t.strip().strip('"') for t in m.group(2).split(",")] if m.group(2) else []
            continue
        m = re.match(r'psql:[^:]*:(\d+): ERROR:\s*(.*)$', line)
        if m:
            errs[int(m.group(1)) - 2] = m.group(2).strip()
    bad = []
    cols = 0
    for i, it in enumerate(items):
        key = f"p{i}"
        if key not in res:
            continue
        pg, rs = res[key], it["types"]
        # FromRow 按**列名**匹配，SQL 多给几列是合法的；少给才是错。
        if it["kind"].startswith("struct:"):
            if len(pg) < len(rs):
                bad.append((it, f"列不足：SQL {len(pg)} < 结构体 {len(rs)}"))
            continue
        if it["kind"] == "scalar":
            t = unwrap(rs[0])
            # query_scalar + fetch_all 的 Rust 类型是 Vec<T>，每行仍只有一列。
            rs = [t[4:-1] if t.startswith("Vec<") and t.endswith(">") else t]
            pg = pg[:1]
        if len(pg) != len(rs):
            bad.append((it, f"列数不符：SQL {len(pg)} vs Rust {len(rs)}"))
            continue
        for idx, (p, r) in enumerate(zip(pg, rs)):
            cols += 1
            allowed = OK.get(unwrap(r))
            if allowed is None:
                continue          # 不认识的 Rust 类型，宁可不报
            if p not in allowed:
                bad.append((it, f"第 {idx+1} 列：SQL={p} / Rust={r}"))
    # **PREPARE 没跑通的必须报出来。** 静默跳过的话，一条引用了尚未迁移的列的查询
    # 会被算成「检查过了、没问题」——而它其实一次都没被检查。本仓当场踩到：新加的
    # ref_micro_usd 列还没迁移，那条查询 PREPARE 失败，而工具照样打印「没有对不上的」。
    # 带 `{` 的是运行时拼接的动态 SQL（format! 占位符），那种本来就 PREPARE 不了。
    hard = {
        i: e for i, e in errs.items()
        if 'syntax error at or near "{"' not in e and 0 <= i < len(items)
    }
    print(f"sql-type-check: 核对 {len(res)} 条查询 / {cols} 个列位"
          + (f"，另有 {len(errs) - len(hard)} 条动态 SQL 跳过" if len(errs) > len(hard) else ""))
    if hard:
        print("sql-type-check: 下面这些查询**根本没能被检查**（PREPARE 就失败了）：")
        for i, e in sorted(hard.items()):
            it = items[i]
            print(f"  {it['file']}:{it['line']}  {e}")
        print("常见原因：引用了还没迁移上去的列。先把迁移跑到这个库上，再跑这道检查。")
        return 1
    if not bad:
        print("sql-type-check: 没有对不上的")
        return 0
    print("sql-type-check: 下面这些查询会在**返回行时**解码失败，也就是接口 500：")
    for it, msg in bad:
        print(f"  {it['file']}:{it['line']}  [{it['kind']}] {msg}")
        print(f"      {it['sql'][:160]}")
    print("修法一律是在 SQL 里显式转型（::bigint / ::float8），不要去改 Rust 的类型。")
    return 1


if __name__ == "__main__":
    items = main()
    if "--emit-prepare" in sys.argv:
        print("\\set ON_ERROR_STOP off")
        for i, it in enumerate(items):
            print(f"PREPARE p{i} AS {it['sql'].strip().rstrip(';')};")
        print("SELECT name, result_types FROM pg_prepared_statements ORDER BY length(name), name;")
        sys.exit(0)
    if "--compare" in sys.argv:
        sys.exit(compare(items, sys.argv[sys.argv.index("--compare") + 1]))
    print(f"抽到 {len(items)} 条查询（--emit-prepare 生成 SQL，--compare <psql输出> 比对）")
