import readline from "node:readline";

const input = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
const pending = new Map();
let serverRequestId = 9000;

function send(value) {
  process.stdout.write(JSON.stringify(value) + "\n");
}

function reply(id, result) {
  send({ jsonrpc: "2.0", id, result });
}

function toolPage(request) {
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
  return {
    tools: [{
      name: "resource_echo",
      description: "Return an embedded resource",
      inputSchema: { type: "object", properties: {} },
    }],
  };
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

  if (message.method === "initialize") {
    reply(message.id, {
      protocolVersion: "2025-06-18",
      capabilities: { tools: {} },
      serverInfo: { name: "michael-ide-test-fixture", version: "1.0.0" },
    });
    return;
  }

  if (message.method === "tools/list") {
    const requestId = serverRequestId++;
    pending.set(requestId, message);
    send({ jsonrpc: "2.0", id: requestId, method: "ping", params: {} });
    return;
  }

  if (message.method === "tools/call") {
    send({ jsonrpc: "2.0", method: "notifications/progress", params: { progress: 1 } });
    if (message.params?.name === "echo") {
      reply(message.id, { content: [{ type: "text", text: String(message.params?.arguments?.text || "") }] });
    } else if (message.params?.name === "resource_echo") {
      reply(message.id, {
        content: [{
          type: "resource",
          resource: { uri: "fixture://proof", mimeType: "text/plain", text: "resource body" },
        }],
      });
    } else {
      send({ jsonrpc: "2.0", id: message.id, error: { code: -32602, message: "unknown tool" } });
    }
  }
});
