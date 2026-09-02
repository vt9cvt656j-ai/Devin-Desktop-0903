// 能力缺口：没有专用工具时，模型该被指向组合路径，而不是被推回去空搜。
//
// 用户的原话是「用户让你做东西 ide 没有的、你能做的…那些都要会，不然就会很呆」。
// 查下来「呆」有很具体的来源：能力缺口在每一处都被**误诊**成别的问题——
//   · search_tools 没命中 → 说成"检索失败，换个查询词再搜"
//   · 二进制/不认识的格式 → 说成"找不到文件，先确认真实路径"，还会累加失败计数越试越死
//   · 文档解析失败 → 直接当成文件内容返回，这次读取被记为**成功**并进缓存
// 三条都指向同一件事：模型手上有 http_request / run_cmd / write_file，够把事情做成，
// 只是没有任何一句话告诉它可以这么做。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
// 正向源码断言必须跑在**剥掉注释**的源码上。注释不是代码：把一条契约从代码里删掉、
// 只在注释里留一句，assert.match 照样绿——本仓库已经这样漏过一整组模型可见的工具契约。
// 所以 `SRC` 绑定的是 CODE（注释整段置空，行号与偏移和原文一字不差）；
// 真要匹配注释本身的断言显式用 RAW_SRC，并在那一行写清为什么。
import { CODE as SRC, SRC as RAW_SRC } from "./helpers/source.mjs";
const FILES_RS = readFileSync(join(HERE, "..", "src-tauri", "src", "files.rs"), "utf8");

// 组合路径的四条出路，缺一条模型就少一种解法
const ROUTES = [
  ["http_request", "外部服务/API 要能自己按文档打通"],
  ["run_cmd", "本地格式与批处理要能自己写脚本跑"],
  ["sqlite", "工作区里的 .db 文件路径本身就是连接串，不该回头问用户"],
  ["SKILL.md", "值得复用的流程要能存下来，下一轮直接用"],
  // 2026-08-19：这两条不再只是"路"，而是真有工具了——接 MCP 和存技能都能自己做。
  ["mcp_server", "外部服务有官方 MCP 时要能自己接进来，而不是回一句「没有这个能力」"],
  ["save_skill", "存技能要有明确的工具，不能靠模型自己猜路径去 write_file"],
];

// 四条出路现在是一份共用常量。原来它被手抄了两份，而另外两个出口（编了个不存在的工具名、
// 抓页面失败）压根没有——所以钉两样：常量本身一条不缺，且每个出口都真的接上了它。
test("能力缺口的换路清单只有一份，四条出路一条不缺", () => {
  const at = RAW_SRC.indexOf("const _CAPABILITY_ROUTES =");
  assert.ok(at > 0, "换路清单被改名或删了");
  const decl = SRC.slice(at, RAW_SRC.indexOf(";\n", at));
  for (const [needle, why] of ROUTES) {
    assert.ok(decl.includes(needle), `缺 ${needle}：${why}`);
  }
  assert.ok(decl.includes("run_in_terminal"), "要跑起来看的服务/TUI 也是一条出路");
});

test("search_tools 精确名没命中时给的是出路，不是「换个词再搜」", () => {
  const at = RAW_SRC.indexOf("当前注册表没有名为");
  assert.ok(at > 0, "找不到精确名 miss 的文案");
  const line = SRC.slice(at, RAW_SRC.indexOf("`;", at));
  assert.match(line, /\$\{_CAPABILITY_ROUTES\}/, "没接上换路清单，模型只会被推回去再搜一次");
  assert.match(line, /不是能力边界|起手包/, "要点明注册表不等于能力上限");
});

test("语义调度也没命中时同样给出路", () => {
  const at = RAW_SRC.indexOf("语义工具调度本次不可用");
  assert.ok(at > 0, "找不到语义兜底文案");
  const line = SRC.slice(at, at + 700);
  assert.match(line, /_CAPABILITY_ROUTES/, "没接上换路清单");
  // 旧文案把能力缺口框成检索问题，会让模型继续空转
  assert.ok(!line.includes("换更具体的能力描述"),
    "不能只让它换查询词——那是把能力缺口当成搜索技巧问题");
});

test("模型编了个不存在的工具名时，也要给出路", () => {
  // 这一步恰恰是「它认为该有这个能力、而注册表里没有」的时刻，最需要换路指引。
  // 原来这里只说「请用 search_tools 描述当前所需能力，或按已注册工具名重试」——把它推回
  // 它刚刚证明了没有它要的东西的那张表。
  const at = RAW_SRC.indexOf("没有通过词形、关键词或相似度猜测替代工具");
  assert.ok(at > 0, "未知工具的兜底文案找不到了");
  const line = SRC.slice(at, at + 400);
  assert.match(line, /_CAPABILITY_ROUTES/, "只把模型推回注册表，等于告诉它这事做不了");
  assert.match(line, /不是能力边界|起手包/);
});

test("抓页面/联网搜索失败要给换路，不能只说检查网络", () => {
  // agent_core 第 5 条把「web_search + web_fetch 读官方文档」定为造能力的第一步。
  // 这一步倒下时，原来唯一的指示是「检查 URL、检查网络、换个关键词试试」。
  const fetchAt = RAW_SRC.indexOf("[ERROR] 网页抓取失败");
  assert.ok(fetchAt > 0);
  const fetchLine = SRC.slice(fetchAt, fetchAt + 500);
  assert.match(fetchLine, /http_request/, "反爬/要登录/要渲染时的换路没给");
  assert.match(fetchLine, /browser|curl/, "换真实浏览器或 curl 这条路没给");

  const searchAt = RAW_SRC.indexOf("[ERROR] 联网搜索失败");
  assert.ok(searchAt > 0);
  const searchLine = SRC.slice(searchAt, searchAt + 500);
  assert.match(searchLine, /web_fetch|http_request/, "已知站点直接打官网这条路没给");
  assert.ok(!/换个关键词试试/.test(searchLine), "只让它换关键词就是让它原地空转");
});

test("文档解析失败不再被当成文件内容返回", () => {
  // 以前是 `catch (e) { return "[文档无法解析] …" }`：错误变成了正文，readFailed 是 false，
  // 这次读取被记为成功并进读缓存，模型拿着一句错误文案当文档往下推。
  const at = RAW_SRC.indexOf("async function _readFileOrDoc(");
  assert.ok(at > 0);
  const fn = SRC.slice(at, RAW_SRC.indexOf("\n}\n", at));
  assert.doesNotMatch(fn, /catch \(e\) \{ return `\[文档无法解析\]/,
    "解析失败不能当作内容返回——那会把失败记成成功");
  assert.match(fn, /throw new Error\(/, "解析失败要真的抛出去");
  assert.match(fn, /run_cmd/, "要顺带说清真正的出路");
});

test("Excel 抽取不再只拿共享字符串，而且会声明自己是有损的", () => {
  // 只抽 sharedStrings 是去重后的字符串池：没有行列、没有 sheet 名，数值单元格根本不在
  // 这个文件里。纯数字表抽出来是空的，而模型会拿这堆字符串编出行列关系。
  assert.match(FILES_RS, /xl\/worksheets\/sheet/,
    "必须连 worksheet 一起抽，否则数值单元格一个都拿不到");
  assert.match(FILES_RS, /文本层抽取/, "抽取结果必须自报有损，别让模型当成结构化表格");
  assert.match(FILES_RS, /openpyxl|pandas/, "要指明按行列读真实数据的办法");
});
