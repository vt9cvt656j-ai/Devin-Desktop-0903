// 用户自己声明的角色。
//
// 角色以前是**三张写死的表**：合法名单、提示词、工具矩阵。加一个角色要改三处代码，
// 而这三处的查表函数早就写成了「按 key 查、查不到返回默认」——缺的从来不是分发，
// 是「行」的来源。
//
// 这里守住三条，每条都对着一种「加了等于没加」的失败：
//   1. 声明的提示词真的到了子智能体手里（否则角色只是个名字）
//   2. 声明的工具矩阵真的到了子智能体手里（否则 design 角色打不开浏览器）
//   3. 模型**知道**这个角色存在（枚举里没有的角色，它永远不会选）
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";
import { normalizeCapabilities } from "../src/agent/capabilities.js";

const HERE = dirname(fileURLToPath(import.meta.url));
// 正向源码断言必须跑在**剥掉注释**的源码上。注释不是代码：把一条契约从代码里删掉、
// 只在注释里留一句，assert.match 照样绿——本仓库已经这样漏过一整组模型可见的工具契约。
// 所以 `SRC` 绑定的是 CODE（注释整段置空，行号与偏移和原文一字不差）；
// 真要匹配注释本身的断言显式用 RAW_SRC，并在那一行写清为什么。
import { CODE as SRC, SRC as RAW_SRC } from "./helpers/source.mjs";

function extractFn(name) {
  const i = RAW_SRC.indexOf(`function ${name}(`);
  assert.ok(i >= 0, `main.js 里找不到 ${name}`);
  let depth = 0;
  let j = RAW_SRC.indexOf("{", RAW_SRC.indexOf(")", i));
  for (; j < RAW_SRC.length; j++) {
    const c = RAW_SRC[j];
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (!depth) break; }
  }
  return RAW_SRC.slice(i, j + 1);
}

const DECL = normalizeCapabilities({
  roles: [{
    name: "data",
    prompt: "你是数据工程专家：先看清表结构和数据量级，再动手。",
    tools: ["db_query", "http_request", "这个工具不存在"],
    types: ["db", "http"],
  }],
}, "项目配置");

/** 用给定声明构建 _userRoleMap（内部要查真实工具目录，所以把它也一并注入）。 */
function roleMapWith(caps) {
  return new Function(
    "_userCapabilities", "_buildAgentToolSchemas",
    `${extractFn("_userRoleMap")}\n;return _userRoleMap;`,
  )(
    () => caps,
    () => [
      { function: { name: "db_query" } },
      { function: { name: "http_request" } },
      { function: { name: "browser" } },
    ],
  )();
}

test("声明的提示词真的进了子智能体的角色段", () => {
  const block = new Function(
    "_userRoleMap", "_AGENT_ROLE_BLOCKS",
    `${extractFn("_agentRoleBlock")}\n;return _agentRoleBlock;`,
  )(() => roleMapWith(DECL), { frontend: "内置前端角色" });
  assert.match(block("data"), /数据工程专家/, "声明的角色提示词没到子智能体手里——那这个角色只是个名字");
  // 内置角色不受影响。
  assert.match(block("frontend"), /内置前端角色/);
  // 谁都不认识的角色仍然返回空，而不是抛错。
  assert.equal(block("不存在的角色"), "");
});

test("声明的工具矩阵真的到了子智能体手里，而写错的工具名被挡掉", () => {
  const caps = new Function(
    "_userRoleMap", "_ROLE_CAPABILITIES",
    `${extractFn("_roleCapabilities")}\n;return _roleCapabilities;`,
  )(() => roleMapWith(DECL), { frontend: { tools: ["browser"], types: ["browser"] } });

  const got = caps("data", true);
  assert.deepEqual(got.tools, ["db_query", "http_request"],
    "不存在的工具名必须被挡掉——放行的代价是子智能体派出去之后才发现手里没这件工具");
  assert.deepEqual(got.types, ["db", "http"]);
  // 只读子智能体照旧什么副作用工具都不给，用户声明也不能突破这条。
  assert.deepEqual(caps("data", false), { tools: [], types: [] });
  // 内置角色不受影响。
  assert.deepEqual(caps("frontend", true).tools, ["browser"]);
});

test("没有任何声明时，角色行为和以前完全一样", () => {
  const empty = normalizeCapabilities({}, "");
  const caps = new Function(
    "_userRoleMap", "_ROLE_CAPABILITIES",
    `${extractFn("_roleCapabilities")}\n;return _roleCapabilities;`,
  )(() => roleMapWith(empty), { frontend: { tools: ["browser"], types: ["browser"] } });
  assert.deepEqual(caps("frontend", true).tools, ["browser"]);
  assert.deepEqual(caps("data", true), { tools: [], types: [] });
});

test("模型要知道这个角色存在——但没声明时提示词必须逐字节不变", () => {
  const suffix = new Function(
    "_userCapabilities",
    `${extractFn("_userRoleEnumSuffix")}\n;return _userRoleEnumSuffix;`,
  );
  // 没声明 → 空串。这段提示词在缓存前缀里，多一个字符就是一次全量未命中。
  assert.equal(suffix(() => normalizeCapabilities({}, ""))(), "",
    "没声明角色时也改了提示词——那会让所有用户白丢一次 prompt cache");
  assert.equal(suffix(() => DECL)(), "/data", "声明了角色，枚举里却没有它，模型永远不会选它");
});

test("模型编出来的角色名仍然挡住", () => {
  // 放宽到用户声明不等于放开：编出来的角色既没提示词也没工具矩阵，派出去是个空壳。
  assert.match(SRC, /_AI_AGENT_ROLES\.has\(item\) \|\| _userRoleMap\(\)\.has\(item\)/,
    "角色过滤要么被删了、要么没接上用户声明");
});
