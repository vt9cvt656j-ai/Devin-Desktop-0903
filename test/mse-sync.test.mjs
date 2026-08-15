// 三份 MSE 客户端副本必须和唯一的源逐字节一致。
//
// 官网（ide/website）、用户后台、管理后台各有一份 src/lib/mse.ts，都是
// server/web-shared/mse.ts 的生成物。三份手抄的密码学代码里有一份改漏了，症状是
// **那一个前端偶尔解不开**：另外两个好好的，服务端日志里只有一句「解密失败」，
// 是这套系统里最难查的一类故障。
//
// 判定交给 sync-mse-client.mjs --check 本身跑，而不是在这里再比一次文件：比对逻辑
// （banner 长什么样、目标有哪几个）只该有一处定义，测试里再写一份就成了第四份副本。
//
// 源和脚本都在父仓库里。ide/ 可以被单独 clone 出来（见 repo_sync.rs 顶部），那时候
// 跳过 —— 缺文件不是漂移。
import { existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const PARENT = join(HERE, "..", "..");
const SCRIPT = join(PARENT, "server", "scripts", "sync-mse-client.mjs");
const SOURCE = join(PARENT, "server", "web-shared", "mse.ts");

const missing = [SCRIPT, SOURCE].filter((p) => !existsSync(p));
const SKIP =
  missing.length > 0
    ? `父仓库不在（${missing.map((p) => p.replace(PARENT, "..")).join(", ")}）—— 缺文件不是漂移`
    : false;

test("三个前端的 src/lib/mse.ts 都还是 web-shared/mse.ts 的副本", { skip: SKIP }, () => {
  const r = spawnSync(process.execPath, [SCRIPT, "--check"], {
    cwd: PARENT,
    encoding: "utf8",
  });

  assert.equal(r.error, undefined, `跑不起来 sync-mse-client.mjs：${r.error?.message}`);
  assert.equal(
    r.status,
    0,
    `MSE 客户端副本和 server/web-shared/mse.ts 不一致：\n${r.stdout}${r.stderr}\n` +
      "副本是生成物，不要手改。改源文件之后重新生成：\n" +
      "  node server/scripts/sync-mse-client.mjs",
  );
  // 脚本一个目标都没检查却退出 0，是「测试变绿了但什么都没守」的那种失败。
  assert.match(r.stdout, /ide\/website\/src\/lib\/mse\.ts/, "检查里没有官网那一份");
});
