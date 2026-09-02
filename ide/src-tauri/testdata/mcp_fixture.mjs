import readline from "node:readline";
import { appendFileSync, writeFileSync } from "node:fs";

const input = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
const pending = new Map();
let serverRequestId = 9000;
// 服务自己的状态变了（第一次工具调用之后），清单随之多出一个工具。
let listChanged = false;

function send(value) {
  process.stdout.write(JSON.stringify(value) + "\n");
}

function reply(id, result) {
  send({ jsonrpc: "2.0", id, result });
}

function toolPage(request) {
  const configuredCount = Number(process.env.MCP_FIXTURE_TOOL_COUNT) || 0;
  if (configuredCount > 0) {
    return {
      tools: Array.from({ length: configuredCount }, (_, index) => ({
        name: `generated_${index}`,
        description: "Generated fixture tool",
        inputSchema: { type: "object", properties: {} },
      })),
    };
  }
  const configuredSchemaBytes = Number(process.env.MCP_FIXTURE_SCHEMA_BYTES) || 0;
  if (configuredSchemaBytes > 0) {
    return {
      tools: [{
        name: "oversized_schema",
        description: "Oversized schema fixture",
        inputSchema: {
          type: "object",
          properties: {
            payload: { type: "string", description: "x".repeat(configuredSchemaBytes) },
          },
        },
      }],
    };
  }
  const cursor = request.params?.cursor || "";
  if (!cursor) {
    return {
      tools: [{
        name: "echo",
        description: "Return the supplied text",
        inputSchema: {
          type: "object",
          properties: { text: { type: "string" } },
          required: ["text"],
        },
      }],
      nextCursor: "page-2",
    };
  }
  const tools = [{
    name: "resource_echo",
    description: "Return an embedded resource",
    inputSchema: { type: "object", properties: {} },
  }];
  if (listChanged) {
    tools.push({
      name: "late_bloomer",
      description: "Only exists after the server announced a list change",
      inputSchema: { type: "object", properties: {} },
    });
  }
  return { tools };
}

input.on("line", (line) => {
  let message;
  try { message = JSON.parse(line); } catch { return; }

  if (!message.method && pending.has(message.id)) {
    const request = pending.get(message.id);
    pending.delete(message.id);
    reply(request.id, toolPage(request));
    return;
  }

  // 客户端放弃等待之后发来的取消通知：记下 requestId，测试据此断言"服务真的被告知了"。
  if (message.method === "notifications/cancelled") {
    const log = String(process.env.MCP_FIXTURE_CANCEL_LOG || "");
    if (log) appendFileSync(log, `${message.params?.requestId}\n`);
    return;
  }

  if (message.method === "initialize") {
    // 一个字都不回、但进程照常活着：远端服务在等用户去浏览器里授权时就是这个样子。
    if (process.env.MCP_FIXTURE_IGNORE_INITIALIZE === "1") return;
    const capabilities = { tools: {}, resources: {}, prompts: {} };
    if (process.env.MCP_FIXTURE_LOGGING === "1") capabilities.logging = {};
    reply(message.id, {
      // 握手是一次**协商**：服务回的版本不一定是客户端报的那个。让它可控，才验得了
      // 客户端拿到一个不认的版本时是当场停下来说清楚，还是揣着往下走。
      protocolVersion: process.env.MCP_FIXTURE_PROTOCOL_VERSION || "2025-06-18",
      capabilities,
      serverInfo: { name: "michael-ide-test-fixture", version: "1.0.0" },
    });
    return;
  }

  // 无论有没有声明 logging 能力都照单回应，并留下痕迹。测试据此断言客户端**只在服务声明了
  // logging 时**才发这条：没声明的那次，痕迹必须一条都没有。
  if (message.method === "logging/setLevel") {
    send({
      jsonrpc: "2.0",
      method: "notifications/message",
      params: { level: "info", logger: "fixture", data: `level set to ${message.params?.level}` },
    });
    reply(message.id, {});
    return;
  }

  if (message.method === "ping") {
    if (process.env.MCP_FIXTURE_IGNORE_PING === "1") return;
    // ping 在 MCP 里不是必须实现的方法：合规的服务完全可以回 -32601。这条分支就是那种服务。
    if (process.env.MCP_FIXTURE_PING_UNSUPPORTED === "1") {
      send({ jsonrpc: "2.0", id: message.id, error: { code: -32601, message: "Method not found" } });
      return;
    }
    reply(message.id, {});
    return;
  }

  if (message.method === "tools/list") {
    const requestId = serverRequestId++;
    pending.set(requestId, message);
    send({ jsonrpc: "2.0", id: requestId, method: "ping", params: {} });
    return;
  }

  if (message.method === "tools/call") {
    // 不带令牌的进度：客户端**不能**拿它续预算（归不到具体哪条请求上）。这一条一直在发，
    // 正是超时那几条测试还能正常超时的前提。
    send({ jsonrpc: "2.0", method: "notifications/progress", params: { progress: 1 } });
    if (message.params?.name === "exit") {
      process.exit(0);
    }
    if (message.params?.name === "delay_echo") {
      // 上限放宽到 60 秒：要验"一直报进度的长任务不会被静默预算掐掉"，1.5 秒是不够的。
      const delay = Math.max(0, Math.min(60000, Number(message.params?.arguments?.delay_ms) || 0));
      const startedPath = String(message.params?.arguments?.started_path || "");
      if (startedPath) writeFileSync(startedPath, "started");
      // 带令牌的进度：客户端认得出"这条是我那个请求的"，据此把静默预算重新计时。
      const every = Math.max(0, Number(message.params?.arguments?.progress_ms) || 0);
      const token = message.params?._meta?.progressToken;
      let ticker = null;
      if (every > 0 && token !== undefined) {
        let progress = 0;
        ticker = setInterval(() => {
          send({
            jsonrpc: "2.0",
            method: "notifications/progress",
            params: { progressToken: token, progress: ++progress },
          });
        }, every);
      }
      setTimeout(() => {
        if (ticker) clearInterval(ticker);
        reply(message.id, { content: [{ type: "text", text: String(message.params?.arguments?.text || "") }] });
      }, delay);
    } else if (message.params?.name === "echo") {
      // arguments.pid：回自己的进程号，而不是回声。测试靠它证明"这次调用落在了哪个子进程上"
      // ——同名服务在两个 (窗口, 根目录) 下必须是两个进程，各自答各自的。
      // 做成 echo 的一个参数而不是新工具，是为了不改动别处对工具清单的断言。
      const text = message.params?.arguments?.pid === true
        ? String(process.pid)
        : String(message.params?.arguments?.text || "");
      reply(message.id, { content: [{ type: "text", text }] });
    } else if (message.params?.name === "resource_echo") {
      reply(message.id, {
        content: [{
          type: "resource",
          resource: { uri: "fixture://proof", mimeType: "text/plain", text: "resource body" },
        }],
      });
    } else {
      send({
        jsonrpc: "2.0",
        id: message.id,
        error: { code: -32602, message: "unknown tool", data: { tool: message.params?.name } },
      });
    }
    // 这条通知**在回应之后**发，正是它难处理的地方：请求那一层拿到回应就返回了，这条只能
    // 等下一次请求（每轮的 mcp_status ping）顺路读到。
    if (process.env.MCP_FIXTURE_LIST_CHANGED === "1" && !listChanged) {
      listChanged = true;
      send({ jsonrpc: "2.0", method: "notifications/tools/list_changed", params: {} });
    }
    return;
  }

  if (message.method === "resources/list") {
    reply(message.id, {
      resources: [{
        uri: "fixture://proof",
        name: "Fixture proof",
        description: "A readable fixture resource",
        mimeType: "text/plain",
      }],
    });
    return;
  }

  if (message.method === "resources/templates/list") {
    reply(message.id, {
      resourceTemplates: [{
        uriTemplate: "fixture://items/{id}",
        name: "Fixture item",
        description: "A parameterized fixture resource",
        mimeType: "application/json",
      }],
    });
    return;
  }

  if (message.method === "resources/read") {
    reply(message.id, {
      contents: [{ uri: message.params?.uri || "fixture://proof", mimeType: "text/plain", text: "resource body" }],
    });
    return;
  }

  if (message.method === "prompts/list") {
    reply(message.id, {
      prompts: [{
        name: "review",
        description: "Review a target",
        arguments: [{ name: "target", required: true }],
      }],
    });
    return;
  }

  if (message.method === "prompts/get") {
    reply(message.id, {
      description: "Fixture review prompt",
      messages: [{ role: "user", content: { type: "text", text: `Review ${message.params?.arguments?.target || "target"}` } }],
    });
  }
});
