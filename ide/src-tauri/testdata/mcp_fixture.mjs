import readline from "node:readline";
import { writeFileSync } from "node:fs";

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
      capabilities: { tools: {}, resources: {}, prompts: {} },
      serverInfo: { name: "michael-ide-test-fixture", version: "1.0.0" },
    });
    return;
  }

  if (message.method === "ping") {
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
    send({ jsonrpc: "2.0", method: "notifications/progress", params: { progress: 1 } });
    if (message.params?.name === "delay_echo") {
      const delay = Math.max(0, Math.min(5000, Number(message.params?.arguments?.delay_ms) || 0));
      const startedPath = String(message.params?.arguments?.started_path || "");
      if (startedPath) writeFileSync(startedPath, "started");
      setTimeout(() => {
        reply(message.id, { content: [{ type: "text", text: String(message.params?.arguments?.text || "") }] });
      }, delay);
    } else if (message.params?.name === "echo") {
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
