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
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
const FILES_RS = readFileSync(join(HERE, "..", "src-tauri", "src", "files.rs"), "utf8");

// 组合路径的四条出路，缺一条模型就少一种解法
const ROUTES = [
  ["http_request", "外部服务/API 要能自己按文档打通"],
  ["run_cmd", "本地格式与批处理要能自己写脚本跑"],
  ["sqlite", "工作区里的 .db 文件路径本身就是连接串，不该回头问用户"],
  ["SKILL.md", "值得复用的流程要能存下来，下一轮直接用"],
];

test("search_tools 精确名没命中时给的是出路，不是「换个词再搜」", () => {
  const at = SRC.indexOf("当前注册表没有名为");
  assert.ok(at > 0, "找不到精确名 miss 的文案");
  const line = SRC.slice(at, SRC.indexOf("`;", at));
  for (const [needle, why] of ROUTES) {
    assert.ok(line.includes(needle), `缺 ${needle}：${why}`);
  }
  assert.match(line, /不是能力边界|起手包/, "要点明注册表不等于能力上限");
});

test("语义调度也没命中时同样给出路", () => {
  const at = SRC.indexOf("语义工具调度本次不可用");
  assert.ok(at > 0, "找不到语义兜底文案");
  const line = SRC.slice(at, SRC.indexOf('";', at));
  for (const [needle, why] of ROUTES) {
    assert.ok(line.includes(needle), `缺 ${needle}：${why}`);
  }
  // 旧文案把能力缺口框成检索问题，会让模型继续空转
  assert.doesNotMatch(
    line.slice(0, line.indexOf("这**不是检索失败") + 1 || line.length),
    /^[^]*换更具体的能力描述[^]*$/,
    "不能只让它换查询词——那是把能力缺口当成搜索技巧问题",
  );
});

test("文档解析失败不再被当成文件内容返回", () => {
  // 以前是 `catch (e) { return "[文档无法解析] …" }`：错误变成了正文，readFailed 是 false，
  // 这次读取被记为成功并进读缓存，模型拿着一句错误文案当文档往下推。
  const at = SRC.indexOf("async function _readFileOrDoc(");
  assert.ok(at > 0);
  const fn = SRC.slice(at, SRC.indexOf("\n}\n", at));
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
