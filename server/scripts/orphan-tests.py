#!/usr/bin/env python3
"""测试模块里「写成测试的样子却没有 #[test]」的函数。

存在的理由是两次真实事故。`drained_pool_makes_the_gate_reachable_again` 和
`the_topup_rate_is_per_plan_not_a_single_exchange_rate` 各自漏了 `#[test]`，于是它们
只是普通私有函数：`cargo test` 全绿、`--list` 里一次都不出现，而前者是「用户没钱还
能用」那个修复的**全部行为证据**。也就是说那个修复上线时是没有守卫的，而没有任何
东西会提示这件事——编译器不会（私有函数被同模块引用得到就不算 dead_code，引用不到
就只是一条 warning，混在几十条 warning 里）。

判据刻意收窄，只认「无参 + 函数体里有 assert + 前面没有 #[test]」这一种形状。
辅助函数（src()、fn_body()、code_of() 这类）几乎都有参数或没有断言，不会被误报；
真漏了属性的测试全都长这个样子。宁可漏报也别误报——误报多了这道闸就会被关掉。

用法：python3 scripts/orphan-tests.py [src 目录]，发现问题时退出码 1。
"""
import re, sys, glob, os

SRC = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), "..", "src")

def scan(path):
    src = open(path, encoding="utf8").read()
    out = []
    for m in re.finditer(r'\n(\s*)(?:pub\s+)?fn\s+([a-z_][a-z0-9_]*)\s*\(\s*\)\s*\{', src):
        # 函数体
        i = m.end(); d = 1
        while i < len(src) and d > 0:
            if src[i] == '{': d += 1
            elif src[i] == '}': d -= 1
            i += 1
        if "assert" not in src[m.end():i]:
            continue
        # 紧邻这个 fn 的属性/注释块里有没有 #[test]
        pre = src[max(0, m.start() - 800):m.start()]
        block = pre.rsplit("\n\n", 1)[-1]
        # **先剥注释再找属性。** 不剥的话这道闸是恒真的：本仓的测试文档里就写着
        # 「少了 `#[test]`，这条从来没跑过」，那行注释本身含有 `#[test]` 这几个字，
        # 于是判据被自己的说明喂饱 —— 实测把真属性删掉，它照样报「没有问题」。
        code = "\n".join(l for l in block.split("\n") if not l.strip().startswith("//"))
        if "#[test]" in code or "#[tokio::test]" in code:
            continue
        out.append((os.path.basename(path), src.count("\n", 0, m.start()) + 1, m.group(2)))
    return out

bad = []
for f in sorted(glob.glob(os.path.join(SRC, "*.rs"))):
    bad += scan(f)

if not bad:
    print("orphan-tests: 没有漏挂 #[test] 的测试函数")
    sys.exit(0)
print("orphan-tests: 下面这些函数有断言、却没有 #[test]，它们一次都不会跑：")
for f, l, n in bad:
    print(f"  {f}:{l}  {n}()")
print("补上 #[test]，或者如果它其实是辅助函数，给它加个参数/改名以示区别。")
sys.exit(1)
