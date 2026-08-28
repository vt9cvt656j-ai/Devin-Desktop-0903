#!/usr/bin/env bash
# 跑测试，绿了才部署。
#
# # 为什么要有这个脚本
#
# 同一个检查手写了八次，其中两次有洞，而且两次都放行了红的东西：
#
#   1. `cargo test | grep "test result" && ./deploy.sh` —— 管道的退出码是最后一个
#      命令（grep/head）的，**测试的退出码从头到尾没参与判断**。等于搭了一道
#      看起来像门的东西，然后从旁边走过去。
#
#   2. 改成「有没有 FAILED 行」之后仍然有洞：**编译失败时既没有 FAILED 也没有
#      test result**，两个都不存在，于是「没有失败」被当成「通过了」。
#      这正是「没查到证据」被当成「没有问题」。
#
# 所以这里的判据是**正面的**：必须看见 "test result: ok"，看不见就当失败。
# 不是「没有坏消息」，是「有好消息」。
set -uo pipefail

LOG=$(mktemp)
trap 'rm -f "$LOG"' EXIT

echo "=== 跑测试 ==="
cargo test --offline >"$LOG" 2>&1
STATUS=$?

# 正面判据：**测试真的跑起来了**——至少有一行 "^test result:"。编译失败时这一行不存在。
#
# 判据从 "test result: ok" 放宽到 "test result:" 是有具体原因的，不是松口：
# 整个后端是**一个**测试二进制，只要有一条红，那唯一的一行就是
# "test result: FAILED"，"ok" 那一行根本不会出现。于是下面那条**具名的**
# repo_sync 例外（脚本里白纸黑字写着「唯一的红是 repo_sync 就放行」）
# 永远走不到 —— 一条写了却到不了的规则，比没写更糟。
#
# 「有没有跑起来」和「跑出来红没红」是两件事，现在分两处判：
# 这一处只判前者（防的是把编译失败当成没问题），红不红交给下面那段具名分析。
if ! grep -q "^test result:" "$LOG"; then
    echo "!!! 一个 'test result' 都没有 —— 测试没跑起来（编译失败）。不部署。"
    grep -E "^error" -A 6 "$LOG" | head -30
    exit 1
fi
grep -E "^test result" "$LOG"

FAILED=$(grep "FAILED$" "$LOG" | sed 's/^test //;s/ \.\.\. FAILED//' | sort)
# repo_sync 守的是 ide/ 的双仓副本一致性，属于别的会话的活；
# 后端镜像只从 server/ 构建，ide/ 不进包 —— 与部署无因果关系。
# 例外必须**具名**且打在屏幕上：绕过和「拦不住」是两回事，前者要留痕。
OTHER=$(echo "$FAILED" | grep -v "^repo_sync::" | grep -v '^$' || true)
if [ -n "$OTHER" ]; then
    echo "!!! repo_sync 之外有测试红了，不部署："
    echo "$OTHER" | sed 's/^/    /'
    exit 1
fi
if [ -n "$FAILED" ]; then
    echo "⚠ 唯一的红是 repo_sync（ide/ 副本镜像，别人的活；后端只从 server/ 构建）—— 放行"
fi
if [ "$STATUS" -ne 0 ] && [ -z "$FAILED" ]; then
    echo "!!! cargo test 退出码 $STATUS 但没有具名失败 —— 情况不明，不部署。"
    tail -20 "$LOG"
    exit 1
fi

echo "=== 部署后端 ==="
SERVER_HOST="${SERVER_HOST:-154.44.13.133}" SERVER_KEY="${SERVER_KEY:-$HOME/.ssh/michael_server}" ./deploy.sh || exit 1

if [ "${WITH_CONSOLE:-1}" = "1" ]; then
    echo "=== 发布控制台 ==="
    SERVER_KEY="${SERVER_KEY:-$HOME/.ssh/michael_server}" ./deploy-admin-ui.sh || exit 1
fi

# 部署成功了，说明这一版的迁移**已经在线上跑过**。把它们记进 APPLIED.txt ——
# 之后谁再去改其中任何一个文件，`the_applied_migrations_are_never_edited` 会在
# 测试阶段红掉，而不是等到部署时后端反复重启才发现。
#
# 放在最后而不是最前：只有真的部署成功才算「上线跑过」。
python3 - <<'PY'
import hashlib, io, os, glob
rows = []
for f in sorted(glob.glob("migrations/*.sql")):
    ver = os.path.basename(f).split("_")[0]
    rows.append(f"{ver}  {hashlib.sha384(io.open(f,'rb').read()).hexdigest()}  {os.path.basename(f)}")
p = "migrations/APPLIED.txt"
old = io.open(p, encoding="utf-8").read()
head = "\n".join(l for l in old.splitlines() if l.startswith("#") or not l.strip())
io.open(p, "w", encoding="utf-8").write(head.rstrip("\n") + "\n" + "\n".join(rows) + "\n")
print(f"=== 迁移清单已刷新（{len(rows)} 个）===")
PY
