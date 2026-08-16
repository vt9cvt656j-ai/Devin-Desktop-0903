// 发版构建脚本。
//
// 这个脚本的全部意义是：让代码签名的**指定要求**钉在证书上而不是 cdhash 上。钉在 cdhash
// 上时，每发一版用户之前授的隐私权限就当场作废，而系统设置里的开关还照样亮着——用户看到
// 的是「权限明明开着却说没权限」。
//
// 它自己却一度是坏的，而且坏法很典型：**前提过时了，没人发现**。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SH = readFileSync(join(HERE, "..", "scripts", "build-signed.sh"), "utf8");
const CONF = JSON.parse(readFileSync(join(HERE, "..", "src-tauri", "tauri.conf.json"), "utf8"));

test("身份从配置里读，不能只认环境变量", () => {
  // 原来是「没设 APPLE_SIGNING_IDENTITY 就拒绝构建」，理由写着「直接跑 tauri build 会用
  // ad-hoc 签名」——而配置里早就写了真实身份，普通构建一直是正经签名的。于是这个脚本
  // 对着一个配置齐全、能签、也一直在签的项目说「拒绝构建」。
  assert.match(SH, /tauri\.conf\.json/, "脚本没有从配置里取身份");
  assert.match(SH, /signingIdentity/);
  assert.ok(CONF.bundle?.macOS?.signingIdentity, "配置里没有签名身份，脚本的回退取不到东西");
  assert.notEqual(CONF.bundle?.macOS?.signingIdentity, "-", "配置退回 ad-hoc 了");
});

test("查身份不许用 -v——项目用的自签证书永远过不了那一关", () => {
  // 自签证书必然带 CSSMERR_TP_NOT_TRUSTED，被 `-v` 过滤掉。而那个标记只说明「验证这张
  // 证书的链不受信任」，**完全不影响用它签名**。原来脚本让用户用 -v 去查，查不到就去
  // 建证书——等于告诉一个已经拥有正确身份的人「你什么都没有」。
  assert.doesNotMatch(SH, /find-identity -v/, "又用回 -v 了，自签证书永远查不到");
  assert.match(SH, /find-identity -p codesigning/);
  assert.match(SH, /CSSMERR_TP_NOT_TRUSTED/, "没有解释为什么不用 -v，下一个人会改回去");
});

test("构建前先确认身份真能用，别等编译完才失败", () => {
  // 只导入了证书没导入私钥时，codesign 会在构建**最后一步**才炸——几分钟白烧。
  assert.match(SH, /钥匙串里找不到签名身份/, "缺少构建前的身份可用性检查");
});

test("签名校验只认本次产物，不能被改名前的遗留物拖红", () => {
  // 原来遍历 *.app.tar.gz，于是应用改名后目录里几个月前的旧名产物会让校验永远失败——
  // 它的 .sig 当然比它自己旧，但那跟这次构建毫无关系。
  assert.match(SH, /productName/, "校验没有按当前产品名定位本次产物");
  assert.doesNotMatch(SH, /for _tar in "\$_bundle_dir"\/\*\.app\.tar\.gz/, "又变回遍历全部了");
  assert.match(SH, /改名前的遗留产物/, "遗留物应当提醒而不是拦截");
});

test("更新签名私钥要注入，且签名不能比包旧", () => {
  assert.match(SH, /TAURI_SIGNING_PRIVATE_KEY=/, "没注入更新签名私钥");
  assert.match(SH, /-ot "\$_tar"/, "没有校验签名比包新");
});

test("最终必须验证指定要求不再钉在 cdhash 上", () => {
  // 这是整个脚本的验收条件。少了它，前面每一步都可能白做而没人知道。
  assert.match(SH, /designated =>/);
  assert.match(SH, /cdhash\*\)/, "没有对 cdhash 结果报错");
  assert.match(SH, /身份跨构建稳定/);
});

// ── 打 DMG 时的验收对象 ──────────────────────────────────────────────────────
//
// Tauri 打完 DMG 会把 macos/ 下的 .app 清掉（日志里那句 "Cleaning …"）。脚本最后那道
// 签名校验原来把路径钉死在那个 .app 上，于是 `MRDAYONE_BUNDLES=dmg` 这条路：codesign
// 读到空 → exit 1，而 DMG 好好地躺在 bundle/dmg 里。退出码说失败、产物却在——这正是
// 这个脚本别处一直在骂的失败模式，只是发生在它自己身上。
// 后果比"看着吓人"严重：真正发给用户的那条路，签名稳定性校验从来没跑过。

test("DMG 路径下改验 DMG 里那份 app，而不是对着被清掉的路径报失败", () => {
  const tail = SH.slice(SH.indexOf('APP="$TARGET_DIR/bundle/macos'));
  assert.match(tail, /if \[ ! -d "\$APP" \]; then/, "没有处理 .app 被清掉的情况");
  assert.match(tail, /ls -t "\$TARGET_DIR"\/bundle\/dmg\/\*\.dmg/, "没去找刚打出来的 DMG");
  assert.match(tail, /hdiutil attach "\$_dmg"[^\n]*-readonly/, "必须只读挂载，别改动产物");
  assert.match(tail, /APP="\$_mounted\/\$\(basename "\$APP"\)"/, "验收对象没切到 DMG 里那份");
  // 挂载失败要明着报错，不能默默拿着空 APP 往下走——那又变回"读不到就 exit 1"的老样子，
  // 但这次原因完全不同，会把人引到错的方向。
  assert.match(tail, /挂载 \$_dmg 失败/);
});

test("挂载点一定会被卸掉，包括校验失败提前退出的那两条路", () => {
  const tail = SH.slice(SH.indexOf('APP="$TARGET_DIR/bundle/macos'));
  assert.match(SH, /trap _cleanup EXIT/, "得用 trap——下面那个 case 里有两个 exit 1，逐个补 detach 必漏");
  assert.match(SH, /hdiutil detach "\$_mounted" -quiet/);
  // trap 是**覆盖**不是追加：临时文件和挂载点各装各的 trap，必然只剩最后一个。
  assert.equal((SH.match(/^trap /gm) || []).length, 1, "装了不止一个 EXIT trap，前面的会被覆盖掉");
  // trap 必须在第一次可能挂载之前就装好。
  // 只看**校验阶段**那次 attach：构建前的残留预清也用 hdiutil，但它只 detach、不 attach，
  // 而且发生在任何挂载点产生之前，没有需要 trap 守护的东西。
  assert.ok(SH.indexOf("trap _cleanup EXIT") < SH.indexOf('hdiutil attach "$_dmg"'),
    "trap 装晚了，挂载后到装 trap 之间失败就会留下残留挂载");
});

// ── 指定架构构建 ────────────────────────────────────────────────────────────
//
// 指定 target 之后产物落在 target/<triple>/release/，而脚本原来有三处硬编码
// target/release/…。不改的话「构建 Intel 版」会成功、然后脚本对着本机 arm64 的旧产物
// 做校验并给出通过的结论——最坏的一种绿。

test("MRDAYONE_TARGET 指定架构时，每一处产物路径都跟着走", () => {
  assert.match(SH, /TARGET="\$\{MRDAYONE_TARGET:-\}"/);
  assert.match(SH, /TARGET_DIR="target\$\{TARGET:\+\/\$TARGET\}\/release"/);
  assert.match(SH, /\$\{TARGET:\+--target "\$TARGET"\}/, "构建命令没把 --target 传下去");
  // 三处校验路径：更新产物、验收的 .app、兜底找的 .dmg
  assert.match(SH, /_bundle_dir="\$TARGET_DIR\/bundle\/macos"/);
  assert.match(SH, /APP="\$TARGET_DIR\/bundle\/macos\/Mr\. Day One\.app"/);
  assert.match(SH, /ls -t "\$TARGET_DIR"\/bundle\/dmg\/\*\.dmg/);
  // provenance 预清也要清对目录，否则 cargo 在新 target 目录里照样撞硬链接失败
  assert.match(SH, /"\$TARGET_DIR"\/build\/\*\/build_script_build-\*/);
  // 一处 target/release/ 硬编码都不该剩
  // 预清那段是唯一允许写死 target/release/ 的地方：DMG 的临时读写映像只会落在本机
  // 架构的 bundle/macos 下，交叉编译产物不经过那条路径。
  const _noPreclean = SH.split("\n").filter((l) => !/^\s*#/.test(l))
    .filter((l) => !/\brw\b/.test(l)).join("\n");
  assert.doesNotMatch(_noPreclean, /target\/release\//, "还有写死本机架构路径的地方");
});

test("只打 dmg 时不许拿上一轮的更新产物冒充这一轮", () => {
  // Tauri 只打 dmg 时不产出 .app.tar.gz，而上一轮 app 构建留下的那份还在原地，
  // .sig 和 .tar.gz 时间戳都是上次的、谁也不比谁旧 —— 原来的新鲜度校验照样打勾。
  // 对一个靠自动更新推测试版的项目，这是最贵的一种假绿。
  assert.match(SH, /_started="\$\(mktemp\)"/, "没有记录开工时间就没法判断产物是不是这一轮的");
  assert.match(SH, /if \[ ! "\$_tar" -nt "\$_started" \]; then/);
  assert.match(SH, /MRDAYONE_BUNDLES=app,dmg/, "报错要给出可执行的下一步");
  // 时间戳必须在 tauri build **之前**取，否则永远判定为陈旧
  assert.ok(SH.indexOf('_started="$(mktemp)"') < SH.indexOf("npm run tauri build"),
    "开工时间戳取晚了，这道校验会永远红");
});

test("构建前要扫掉上次没收干净的 DMG 中间产物", () => {
  // Tauri 的 bundle_dmg.sh 每次建一个 rw.<pid>.dmg 并挂载；任何一次中断都会让映像
  // 永远挂着。攒够十来个之后 hdiutil attach 失败，而 Tauri 只报一句
  // `failed to run bundle_dmg.sh`——不说是挂载失败，也不说为什么。实测就是这么挂的。
  assert.match(SH, /hdiutil detach "\$_dev" -force/);
  assert.match(SH, /rm -f target\/release\/bundle\/macos\/rw\.\*\.dmg/);
  // 只能动本产品自己的临时映像，不许误伤用户挂着的别的磁盘映像
  assert.match(SH, /target\/release\/bundle\/macos\/rw/,
    "过滤条件必须限定到本产品的中间产物路径");
  assert.match(SH, /只动 target\/ 下\*\*本产品自己\*\*的临时读写映像，不碰任何别的磁盘映像/);
  // 清了要说，否则下次有人排查"为什么我的映像被卸载了"会一头雾水
  assert.match(SH, /预清 DMG 残留：卸载 \$_stale_mounts 个挂载/);
});
