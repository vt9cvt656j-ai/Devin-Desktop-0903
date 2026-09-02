// automation 工具那条 browser.* 链路：回执必须描述**真实状态**。
//
// 两个失败模式，都是「把失败说成成功」，而且都只在下一步才以别的形式暴露出来：
//
//   1. 浏览器已经在跑时，browser.start 把 headless 和 profile 两个参数整个丢掉，回执却
//      照着**本次请求**的参数宣布「持久 profile、登录态可用」。模型先用默认 isolated
//      起过浏览器，撞了登录墙，照工具描述带 profile="session" 再 start 一次——收到一句
//      「登录态可用」，而真正在跑的还是那只无 cookie 的隔离实例。
//   2. 用户手关了那个有头窗口（或 Chrome 崩了）之后，实例仍然挂在 Agent 上：每条
//      browser.* 抛底层连接错误，模型调 browser.start 想重启，拿到的是同一句无动作的
//      ok，再调 browser.* 还是连接错误。这个环没有出口。
//   3. browser.close 无论是优雅退出还是超时被强杀都回 {"status":"ok"}——持久 profile 的
//      cookie 没落盘这件事完全不可见，要到下一次运行才以「怎么又要登录」的形式冒出来。
//
// 这里守的是机制：身份跟着实例走、探活是连接级的、回执由结果生成而不是由参数生成、
// 落盘结果不被吞掉。行为本身另有 Rust 单元测试（cargo test --lib）逐条钉着。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const crate = (f) =>
  readFileSync(join(HERE, "..", "automation-framework", "src", f), "utf8");
const AGENT = crate("agent.rs");
const BROWSER = crate("browser.rs");
const RPC = crate("rpc.rs");

// 正向断言不能被注释喂饱：这几个文件的头注里就逐字引用着被修掉的旧写法
// （`if self.browser.is_some() { return Ok(()) }`、旧的那段 note），
// 扫原文的话「旧代码已经删掉」这类断言会一直是红的，而「新机制在」会一直是绿的。
const codeOnly = (text) =>
  text
    .split("\n")
    .filter((l) => !l.trimStart().startsWith("//"))
    .join("\n");

/** 从 `anchor` 起、到 `until` 为止的那段生产代码（注释已剥掉）。 */
function section(source, anchor, until) {
  const at = source.indexOf(anchor);
  assert.notEqual(at, -1, `找不到 ${anchor}`);
  const rel = source.indexOf(until, at + anchor.length);
  assert.notEqual(rel, -1, `${anchor} 之后找不到 ${until}，切不出这一段`);
  return codeOnly(source.slice(at, rel));
}

test("跑着的实例带着自己的身份，回执不再照请求参数编", () => {
  // 身份存在实例上（而不是 Agent 上）：这样它不可能和实例脱节。
  assert.match(BROWSER, /pub struct BrowserIdentity \{/);
  assert.match(codeOnly(BROWSER), /pub fn identity\(&self\) -> BrowserIdentity/);
  // 两条启动路径都必须如实标注有头/无头，否则 headless 那一半照样是编的。
  const code = codeOnly(BROWSER);
  assert.ok(code.includes("BrowserIdentity::new(true, profile)"), "无头路径没标注身份");
  assert.ok(code.includes("BrowserIdentity::new(false, profile)"), "有头路径没标注身份");
});

test("browser.start 命中已有实例时先做连接级探活，死的要重起", () => {
  const body = section(
    AGENT,
    "pub fn browser_start_with_profile(",
    "pub fn browser_goto",
  );
  assert.ok(
    !body.includes("if self.browser.is_some()"),
    "又回到「已经有实例就无条件回 Ok」了——那正是那个没有出口的环",
  );
  assert.ok(body.includes("is_alive()"), "启动路径没做连接级探活");
  assert.ok(
    !body.includes("is_connected()"),
    "拿页面级探活当浏览器存活判据：current_page 为 None 时它恒 true",
  );
  assert.ok(body.includes("decide_start("), "复用/重启没走同一个判据");
  // 三种结局必须是可区分的结构化结果，不能又退回一个裸 Ok(())。
  for (const v of ["Started(", "AlreadyRunning(", "Restarted("]) {
    assert.ok(codeOnly(AGENT).includes(`BrowserStartOutcome::${v}`), `${v} 不见了`);
  }
});

test("探活问的是浏览器自己，不是某个页面", () => {
  const body = section(BROWSER, "pub async fn is_alive(", "\n    }");
  assert.ok(body.includes("self.browser.version()"), "没走 CDP 的 Browser.getVersion");
  assert.ok(!body.includes("current_page"), "又从页面探活了");
  assert.ok(!body.includes("evaluate"), "又从页面探活了");
  // websocket 卡住时 execute 会一直等，探活不能变成挂起。
  assert.ok(body.includes("timeout("), "探活没有超时兜底");
});

test("优雅关闭要回答「有没有真的落盘」，两个结果都算", () => {
  const sig = BROWSER.slice(BROWSER.indexOf("pub async fn close_gracefully("));
  assert.match(sig.split("\n")[0], /Result<bool>/, "close_gracefully 又不回落盘结果了");
  const body = section(BROWSER, "pub async fn close_gracefully(", "\n    }");
  assert.ok(!body.includes("let _ = self.browser.close()"), "又吞掉 close 的结果了");
  // 钉的是**返回值**：写在 warn! 的判据里不算，那句日志不改变任何回执。
  assert.ok(
    body.includes("Ok(close_sent && exited)"),
    "落盘结论不是「两步都成了」——只看其中一个不够，超时被强杀同样是没落盘",
  );

  const closePath = section(AGENT, "fn close_current(", "\n    // ====");
  assert.ok(
    !closePath.includes("let _ = guard.close_gracefully()"),
    "关闭路径又把落盘结果吞掉了",
  );
});

test("两条 RPC 回执都由结果生成，不由请求参数生成", () => {
  const start = section(RPC, '"browser.start" => {', "\n            #[cfg");
  assert.ok(start.includes("browser_start_receipt("), "回执又不是从 outcome 生成的了");
  assert.ok(
    !start.includes("Persistent profile"),
    "又在这条分发里按请求参数编 note 了——这就是「说反」的原始形状",
  );
  const close = section(RPC, '"browser.close" => {', "\n            //");
  assert.ok(close.includes("browser_close_receipt("), "close 又回那句无条件的 ok 了");

  // 模型要能分支的东西必须是结构化字段，不能只藏在一句英文里。
  const code = codeOnly(RPC);
  for (const key of ["already_running", "restarted", "requested_profile", "flushed", "was_running"]) {
    assert.ok(code.includes(`"${key}"`), `回执里没有 ${key} 字段`);
  }
});
