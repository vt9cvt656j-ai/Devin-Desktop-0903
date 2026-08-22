// src-tauri 的测试二进制里，改进程级环境变量的地方只能有一处。
//
// 出过的事：`auth.rs` 的 `mod auth_dir_tests` 里有一把 `static ENV_LOCK`，`mcp.rs` 顶上
// 另有一把。两处的注释都写着「改它们的用例必须排队」，但排的不是同一条队——同一个
// crate、同一个 `cargo test` 进程、默认多线程并行：
//
//   · auth 的用例 `remove_var("HOME")` 后断言 `auth_db_dir()` 走 USERPROFILE，中间 mcp
//     的用例把 HOME 设成临时目录 → 断言看到的是 `/tmp/…/.michael_ide`；
//   · 反向则是 mcp 的 `mcp_user_config()` 读不到自己刚写下的 mcp.json。
//
// 症状是「每次红的不是同一条」，而且两边的局部锁谁都看不见对方。修法是把锁和还原都收进
// `crate::test_env::EnvGuard`，行为本身由 src-tauri/src/test_env.rs 里的 Rust 用例守着。
// 这个文件守的是**结构**：别再长出第二把锁、别再有人绕过守卫直接写环境变量——那是这个
// bug 唯一的复发方式，而它复发时是偶发红，没人会当场发现。
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC_DIR = join(HERE, "..", "src-tauri", "src");
const GUARD_FILE = "test_env.rs";

/**
 * 把 Rust 源码里的注释抹成空格，保留长度和换行（行号不变）。
 *
 * 这里的断言全是**反向**的（"这段文本不许出现"），所以注释不剥的话方向反过来：注释里
 * 引用一句 `std::env::set_var("HOME", …)` 就能把测试喂红，于是下一个人会把断言改松。
 * 字符串、原始字符串、字符字面量一律照原样留着——`'static` 这样的生命周期不能被当成
 * 字符字面量的开头，否则会一路吞掉后面的代码，那才是真的假绿。
 */
export function stripRustComments(source) {
  const out = source.split("");
  const blank = (from, to) => {
    for (let i = from; i < to; i++) if (out[i] !== "\n" && out[i] !== "\r") out[i] = " ";
  };
  let i = 0;
  while (i < source.length) {
    const two = source.slice(i, i + 2);
    if (two === "//") {
      let end = source.indexOf("\n", i);
      if (end === -1) end = source.length;
      blank(i, end);
      i = end;
    } else if (two === "/*") {
      // Rust 的块注释可以嵌套。
      let depth = 1;
      let j = i + 2;
      while (j < source.length && depth > 0) {
        if (source.slice(j, j + 2) === "/*") { depth++; j += 2; }
        else if (source.slice(j, j + 2) === "*/") { depth--; j += 2; }
        else j++;
      }
      blank(i, j);
      i = j;
    } else if (source[i] === "r" && /["#]/.test(source[i + 1] ?? "")) {
      // 原始字符串 r"…" / r#"…"# / r##"…"##
      let hashes = 0;
      let j = i + 1;
      while (source[j] === "#") { hashes++; j++; }
      if (source[j] !== '"') { i++; continue; }
      const close = '"' + "#".repeat(hashes);
      const end = source.indexOf(close, j + 1);
      i = end === -1 ? source.length : end + close.length;
    } else if (source[i] === '"') {
      let j = i + 1;
      while (j < source.length) {
        if (source[j] === "\\") { j += 2; continue; }
        if (source[j] === '"') { j++; break; }
        j++;
      }
      i = j;
    } else if (source[i] === "'") {
      // 字符字面量只有两种形状：'x' 和 '\…'。别的一律是生命周期，原样跳过这一个引号。
      if (source[i + 1] === "\\") {
        let j = i + 2;
        while (j < source.length && source[j] !== "'") j++;
        i = j + 1;
      } else if (source[i + 2] === "'") {
        i += 3;
      } else {
        i += 1;
      }
    } else {
      i++;
    }
  }
  return out.join("");
}

/** src-tauri/src 下每个 .rs 文件的（剥了注释的）源码。 */
const RUST = Object.fromEntries(
  readdirSync(SRC_DIR)
    .filter((name) => name.endsWith(".rs"))
    .map((name) => [name, stripRustComments(readFileSync(join(SRC_DIR, name), "utf8"))]),
);

test("剥注释这一步本身是对的——生命周期不能被当成字符字面量", () => {
  const probe = stripRustComments(
    [
      `struct S<'a> { x: &'a str }`,
      `// 注释里写 std::env::set_var("HOME", "x") 不算数`,
      `let c = 'q'; let esc = '\\n';`,
      `let s = "// 这不是注释"; let r = r#"也 // 不是"#;`,
      `/* 嵌套 /* 的块 */ 注释 */ let real = 1;`,
    ].join("\n"),
  );
  assert.match(probe, /struct S<'a> \{ x: &'a str \}/, "生命周期被吞掉了");
  assert.doesNotMatch(probe, /set_var/, "行注释没剥干净");
  assert.match(probe, /let c = 'q'; let esc = '\\n';/, "字符字面量被改坏了");
  assert.match(probe, /"\/\/ 这不是注释"/, "字符串里的 // 被当成注释剥了");
  assert.match(probe, /r#"也 \/\/ 不是"#/, "原始字符串被当成注释剥了");
  assert.match(probe, /let real = 1;/, "嵌套块注释没配平，把后面的代码一起吞了");
  assert.doesNotMatch(probe, /嵌套/, "块注释没剥");
  assert.equal(probe.split("\n").length, 5, "行数变了，报错行号会对不上");
});

test("全 crate 只有一把环境变量锁，而且在 test_env.rs 里", () => {
  const holders = Object.entries(RUST).filter(([, code]) => /static\s+ENV_LOCK\b/.test(code));
  assert.deepEqual(
    holders.map(([name]) => name),
    [GUARD_FILE],
    "又出现了第二把 ENV_LOCK：两把锁互不相识，跨模块改同一个 HOME 时谁都拦不住谁，" +
      "症状是偶发红且每次红的不是同一条",
  );
  const count = (RUST[GUARD_FILE].match(/static\s+ENV_LOCK\b/g) ?? []).length;
  assert.equal(count, 1, `test_env.rs 里有 ${count} 个 ENV_LOCK`);
});

test("写进程级环境变量只许走 EnvGuard，别处一律不许调 set_var / remove_var", () => {
  const offenders = Object.entries(RUST)
    .filter(([name]) => name !== GUARD_FILE)
    .flatMap(([name, code]) => {
      const lines = code.split("\n");
      return lines
        .map((line, idx) => ({ name, line: line.trim(), no: idx + 1 }))
        .filter(({ line }) => /\benv::(set_var|remove_var)\s*\(/.test(line));
    });
  assert.deepEqual(
    offenders,
    [],
    "这些地方绕过了 EnvGuard 直接改进程级环境变量——改了就没人还原、也没人排队；" +
      "要给子进程设变量用 Command::env，要在用例里改就用 crate::test_env::EnvGuard：\n" +
      offenders.map((o) => `  ${o.name}:${o.no}  ${o.line}`).join("\n"),
  );
});

test("EnvGuard 是「加锁 + 记旧值 + Drop 还原」，不是一把光秃秃的锁", () => {
  const guard = RUST[GUARD_FILE];
  assert.match(guard, /pub\(crate\)\s+struct\s+EnvGuard/, "EnvGuard 不见了");
  assert.match(guard, /MutexGuard<'static, \(\)>/, "守卫没有真的把锁扣在自己身上");
  assert.match(guard, /impl\s+Drop\s+for\s+EnvGuard/, "没有 Drop 就没有还原，panic 出去会留下脏环境");
  // 还原读的必须是记下来的旧值，而不是"清成没设"——auth 的用例改的 HOME 本来是有值的。
  const drop = guard.slice(guard.indexOf("impl Drop for EnvGuard"));
  assert.match(drop, /self\.saved/, "Drop 里没有用记下来的旧值");
  // 排队和改动是同一个对象的事：拿不到锁就写不了变量。
  assert.match(guard, /ENV_LOCK\s*\.lock\(\)/, "守卫没有加锁");
});

test("auth 和 mcp 的环境变量用例都改成走同一个守卫了", () => {
  for (const name of ["auth.rs", "mcp.rs"]) {
    assert.match(
      RUST[name],
      /use\s+crate::test_env::EnvGuard\s*;/,
      `${name} 没有接到共用的守卫上`,
    );
    assert.match(RUST[name], /EnvGuard::(set|serial)\s*\(/, `${name} 里没人真的用守卫`);
  }
  // auth 那条用例是这次撞车的一侧：它读 HOME/USERPROFILE，必须在锁里。
  const auth = RUST["auth.rs"];
  const at = auth.indexOf("fn windows_has_no_home_so_userprofile_must_be_honoured");
  assert.notEqual(at, -1, "auth 的 Windows 家目录用例没了");
  const body = auth.slice(at, auth.indexOf("\n    }\n", at));
  assert.match(body, /EnvGuard::set\s*\(/, "auth 的用例又在裸改 HOME / USERPROFILE 了");
});

test("test_env 模块真的被编进测试二进制", () => {
  const lib = RUST["lib.rs"];
  assert.match(lib, /#\[cfg\(test\)\]\s*\n\s*mod test_env;/, "lib.rs 没有声明 test_env 模块");
});
