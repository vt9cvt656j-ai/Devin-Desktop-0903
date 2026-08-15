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
