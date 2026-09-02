#!/usr/bin/env node

import { createHash, randomUUID } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const TIER_LIMITS = Object.freeze({
  "1m": 1_000_000,
  "2m": 2_000_000,
  "5m": 5_000_000,
});
const BODY_LIMIT_BYTES = 12 * 1024 * 1024;
const SAFE_BODY_BYTES = 11 * 1024 * 1024;
const DEFAULT_BASE_URL = "https://code.mrday.one";
const DEFAULT_MODEL = "gpt-5.4-mini";
const DEFAULT_BATCH_TOKENS = 480_000;
const DEFAULT_BLOCK_TOKENS = 20_000;

function usage() {
  return `Michael effective-context evaluator

Usage:
  node scripts/context-eval.mjs [options]

Safe by default: without --execute it only builds fixtures and reports request sizes.

Options:
  --tier <1m|2m|5m>       Evaluate one tier (repeatable)
  --tiers <csv>            Evaluate several tiers (default: 1m,2m,5m)
  --model <id>             Target model (default: ${DEFAULT_MODEL})
  --base-url <url>         Michael gateway (default: ${DEFAULT_BASE_URL})
  --batch-tokens <n>       Approximate source tokens per upload (default: ${DEFAULT_BATCH_TOKENS})
  --target-ratio <0..1>    Portion of tier limit to fill (default: 0.975)
  --run-id <id>            Reuse deterministic fixtures after an interrupted run
  --resume-covered <n>     Message count covered by MICHAEL_EVAL_RESUME_PREFIX
  --resume-at-checkpoint <n> First checkpoint to send; batch count + 1 runs recall only
  --execute                Send paid production requests
  --verify-pg-recovery     Evict only this run's Redis context keys before recall
  --output <path>          JSON report path
  --warm-timeout-ms <n>    Maximum total warming wait per request (default: 1200000)
  --transient-timeout-ms <n> Maximum 502/503/504 retry time (default: 1200000)
  --request-timeout-ms <n> Timeout for one HTTP attempt (default: 360000)
  --retry-ms <n>           Base warming retry interval (default: 2500)
  --help                    Show this help

Required for --execute:
  MICHAEL_EVAL_API_KEY     Dedicated Michael evaluation API key

Required for resume:
  MICHAEL_EVAL_RESUME_PREFIX  Durable mcp_ prefix token (never written to reports)

Required for --verify-pg-recovery:
  MICHAEL_EVAL_SSH_TARGET  Example: root@154.44.13.133
  MICHAEL_EVAL_SSH_KEY     SSH private key path
  MICHAEL_EVAL_COMPOSE_DIR Production server compose directory
`;
}

function parsePositiveInt(raw, name) {
  const value = Number(raw);
  if (!Number.isSafeInteger(value) || value <= 0) throw new Error(`${name} must be a positive integer`);
  return value;
}

function parseArgs(argv) {
  const options = {
    tiers: [],
    model: process.env.MICHAEL_EVAL_MODEL || DEFAULT_MODEL,
    baseUrl: process.env.MICHAEL_EVAL_BASE_URL || DEFAULT_BASE_URL,
    batchTokens: DEFAULT_BATCH_TOKENS,
    blockTokens: DEFAULT_BLOCK_TOKENS,
    targetRatio: 0.975,
    runId: process.env.MICHAEL_EVAL_RUN_ID || "",
    resumePrefix: process.env.MICHAEL_EVAL_RESUME_PREFIX || "",
    resumeCovered: 0,
    resumeAtCheckpoint: 0,
    execute: false,
    verifyPgRecovery: false,
    output: "",
    warmTimeoutMs: 1_200_000,
    transientTimeoutMs: 1_200_000,
    requestTimeoutMs: 360_000,
    retryMs: 2_500,
    help: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = () => {
      const value = argv[++index];
      if (!value || value.startsWith("--")) throw new Error(`${arg} requires a value`);
      return value;
    };
    if (arg === "--tier") options.tiers.push(next().toLowerCase());
    else if (arg === "--tiers") options.tiers.push(...next().toLowerCase().split(",").filter(Boolean));
    else if (arg === "--model") options.model = next();
    else if (arg === "--base-url") options.baseUrl = next();
    else if (arg === "--batch-tokens") options.batchTokens = parsePositiveInt(next(), arg);
    else if (arg === "--target-ratio") options.targetRatio = Number(next());
    else if (arg === "--run-id") options.runId = next();
    else if (arg === "--resume-covered") options.resumeCovered = parsePositiveInt(next(), arg);
    else if (arg === "--resume-at-checkpoint") options.resumeAtCheckpoint = parsePositiveInt(next(), arg);
    else if (arg === "--output") options.output = next();
    else if (arg === "--warm-timeout-ms") options.warmTimeoutMs = parsePositiveInt(next(), arg);
    else if (arg === "--transient-timeout-ms") options.transientTimeoutMs = parsePositiveInt(next(), arg);
    else if (arg === "--request-timeout-ms") options.requestTimeoutMs = parsePositiveInt(next(), arg);
    else if (arg === "--retry-ms") options.retryMs = parsePositiveInt(next(), arg);
    else if (arg === "--execute") options.execute = true;
    else if (arg === "--verify-pg-recovery") options.verifyPgRecovery = true;
    else if (arg === "--help" || arg === "-h") options.help = true;
    else throw new Error(`unknown option: ${arg}`);
  }
  options.tiers = [...new Set(options.tiers.length ? options.tiers : Object.keys(TIER_LIMITS))];
  for (const tier of options.tiers) {
    if (!TIER_LIMITS[tier]) throw new Error(`unsupported tier: ${tier}`);
  }
  if (!(options.targetRatio > 0 && options.targetRatio < 1)) {
    throw new Error("--target-ratio must be greater than 0 and less than 1");
  }
  if (options.batchTokens >= TIER_LIMITS["1m"]) {
    throw new Error("--batch-tokens must stay below 1,000,000");
  }
  if (options.runId && !/^[a-zA-Z0-9_-]{6,80}$/.test(options.runId)) {
    throw new Error("--run-id may contain only letters, digits, underscores, and hyphens");
  }
  if (options.runId && options.tiers.length !== 1) {
    throw new Error("--run-id can only be used with one tier");
  }
  const resumeValues = [options.resumePrefix, options.resumeCovered, options.resumeAtCheckpoint];
  if (resumeValues.some(Boolean) && !resumeValues.every(Boolean)) {
    throw new Error("resume requires MICHAEL_EVAL_RESUME_PREFIX, --resume-covered, and --resume-at-checkpoint together");
  }
  if (options.resumePrefix && !/^mcp_[a-f0-9]{32}$/i.test(options.resumePrefix)) {
    throw new Error("MICHAEL_EVAL_RESUME_PREFIX is not a valid prefix token");
  }
  if (options.resumePrefix && !options.runId) {
    throw new Error("resume requires --run-id so the exact fixture can be reconstructed");
  }
  options.baseUrl = options.baseUrl.replace(/\/+$/, "");
  return options;
}

// This mirrors server/src/compression.rs. It is a planning estimate, never a billing claim.
function estimateTokens(text) {
  let cjk = 0;
  let other = 0;
  for (const ch of String(text || "")) {
    const code = ch.codePointAt(0);
    if ((code >= 0x4e00 && code <= 0x9fff)
      || (code >= 0x3040 && code <= 0x30ff)
      || (code >= 0xac00 && code <= 0xd7af)
      || (code >= 0x3400 && code <= 0x4dbf)) cjk += 1;
    else other += 1;
  }
  return cjk + Math.ceil(other / 4);
}

function shortHash(value, length = 12) {
  return createHash("sha256").update(String(value)).digest("hex").slice(0, length);
}

function padToTokens(prefix, targetTokens, blockIndex) {
  const used = estimateTokens(prefix);
  if (used > targetTokens) throw new Error(`fixture header exceeds block ${blockIndex} token budget`);
  const alphabet = "上下文有效检索归档稳定前缀真实数据性能验证";
  const remaining = targetTokens - used;
  const repeats = Math.ceil(remaining / alphabet.length);
  return prefix + alphabet.repeat(repeats).slice(0, remaining);
}

function targetTokensForTier(tier, targetRatio) {
  return Math.min(
    TIER_LIMITS[tier] - 5_000,
    Math.floor(TIER_LIMITS[tier] * targetRatio),
  );
}

function makeFacts(runId, tier, targetTokens) {
  const tierLabel = tier.toUpperCase();
  const value = (label) => `${label}-${shortHash(`${runId}:${label}`, 18)}`;
  return [
    { ratio: 0.05, key: "needle_05", anchor: `MC_${runId}_${tierLabel}_NEEDLE_05`, value: value(`${tierLabel}-N05`) },
    { ratio: 0.50, key: "needle_50", anchor: `MC_${runId}_${tierLabel}_NEEDLE_50`, value: value(`${tierLabel}-N50`) },
    { ratio: 0.95, key: "needle_95", anchor: `MC_${runId}_${tierLabel}_NEEDLE_95`, value: value(`${tierLabel}-N95`) },
    { ratio: 0.12, key: "hop_origin", anchor: `MC_${runId}_${tierLabel}_HOP_ORIGIN`, value: value(`${tierLabel}-HOP-A`) },
    { ratio: 0.47, key: "hop_bridge", anchor: `MC_${runId}_${tierLabel}_HOP_BRIDGE`, value: value(`${tierLabel}-HOP-B`) },
    { ratio: 0.82, key: "hop_terminal", anchor: `MC_${runId}_${tierLabel}_HOP_TERMINAL`, value: value(`${tierLabel}-HOP-C`) },
  ].map((fact) => ({
    ...fact,
    tokenPosition: Math.floor(targetTokens * fact.ratio),
  }));
}

function makeFixture(tier, targetTokens, runId, blockTokens, targetRatio) {
  const facts = makeFacts(runId, tier, targetTokens);
  // Embed the same matrix facts at absolute offsets in every tier. With one run ID, the 2M
  // fixture is byte-identical to the start of 5M and exercises content-addressed cache reuse.
  const embeddedFacts = Object.keys(TIER_LIMITS)
    .flatMap((matrixTier) => makeFacts(
      runId,
      matrixTier,
      targetTokensForTier(matrixTier, targetRatio),
    ))
    .filter((fact) => fact.tokenPosition < targetTokens);
  const blockCount = Math.ceil(targetTokens / blockTokens);
  const placements = new Map();
  for (const fact of embeddedFacts) {
    const blockIndex = Math.min(blockCount - 1, Math.floor(fact.tokenPosition / blockTokens));
    fact.blockIndex = blockIndex;
    const list = placements.get(blockIndex) || [];
    list.push(fact);
    placements.set(blockIndex, list);
  }

  const messages = [];
  let generatedTokens = 0;
  for (let blockIndex = 0; blockIndex < blockCount; blockIndex += 1) {
    const remaining = targetTokens - generatedTokens;
    const tokens = Math.min(blockTokens, remaining);
    const embedded = placements.get(blockIndex) || [];
    const factText = embedded
      .map((fact) => `FACT ${fact.anchor} exact_value=${fact.value}. Preserve this exact pair.`)
      .join("\n");
    const header = `CONTEXT_EVAL run=${runId} block=${blockIndex + 1}.\n${factText}\nFILLER:\n`;
    const content = padToTokens(header, tokens, blockIndex);
    messages.push({ role: blockIndex % 2 === 0 ? "user" : "assistant", content });
    generatedTokens += estimateTokens(content);
  }
  if (generatedTokens !== targetTokens) {
    throw new Error(`fixture token mismatch: expected ${targetTokens}, built ${generatedTokens}`);
  }
  return { messages, facts, generatedTokens };
}

function batchMessages(messages, batchTokens) {
  const batches = [];
  let batch = [];
  let tokens = 0;
  for (const message of messages) {
    const messageTokens = estimateTokens(message.content);
    if (batch.length && tokens + messageTokens > batchTokens) {
      batches.push({ messages: batch, tokens });
      batch = [];
      tokens = 0;
    }
    batch.push(message);
    tokens += messageTokens;
  }
  if (batch.length) batches.push({ messages: batch, tokens });
  return batches;
}

function reusablePrefixDigest(fixture, blockTokens, targetRatio) {
  const reusableBlocks = Math.min(
    fixture.messages.length,
    Math.floor(targetTokensForTier("2m", targetRatio) / blockTokens),
  );
  return shortHash(JSON.stringify(fixture.messages.slice(0, reusableBlocks)), 24);
}

function systemMessage(runId) {
  return {
    role: "system",
    content: `You are running controlled context evaluation ${runId}. For ingestion checkpoints, reply only MC_EVAL_ACK. For the final recall audit, obey its JSON-only format and reproduce exact values without guessing.`,
  };
}

function recallMessage(facts) {
  const byKey = Object.fromEntries(facts.map((fact) => [fact.key, fact]));
  const anchors = facts.map((fact) => fact.anchor).join(", ");
  return {
    role: "user",
    content: `FINAL RECALL AUDIT. Search exact archived history for these anchors: ${anchors}. Return only one compact JSON object with string fields needle_05, needle_50, needle_95, hop_origin, hop_bridge, hop_terminal, and hop_chain. Each first six value must be the exact_value paired with its anchor. hop_chain must equal hop_origin + "::" + hop_bridge + "::" + hop_terminal. Do not infer or abbreviate. Expected field labels are fixed; anchor for needle_05 is ${byKey.needle_05.anchor}.`,
  };
}

function requestBody(options, messages, prefix) {
  const body = {
    model: options.model,
    stream: true,
    max_tokens: 512,
    michael_compression: options.tier,
    messages,
  };
  if (prefix) {
    body.mc_prefix = prefix.token;
    body.mc_prefix_covered = prefix.covered;
  }
  return body;
}

function contentDelta(data) {
  const content = data?.choices?.[0]?.delta?.content;
  if (typeof content === "string") return content;
  if (!Array.isArray(content)) return "";
  return content.map((part) => typeof part === "string" ? part : (part?.text || "")).join("");
}

async function readResponse(response, startedAt) {
  const type = response.headers.get("content-type") || "";
  if (!type.includes("text/event-stream") || !response.body) {
    const text = await response.text();
    return {
      text,
      usage: null,
      firstByteMs: Math.round(performance.now() - startedAt),
      totalMs: Math.round(performance.now() - startedAt),
    };
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  let text = "";
  let usage = null;
  let firstByteMs = null;
  const consumeLine = (line) => {
    if (!line.startsWith("data:")) return;
    const raw = line.slice(5).trim();
    if (!raw || raw === "[DONE]") return;
    try {
      const data = JSON.parse(raw);
      text += contentDelta(data);
      if (data.usage) usage = data.usage;
      if (data.error && !text) text = JSON.stringify(data.error);
    } catch { /* ignore non-JSON SSE comments from compatible providers */ }
  };

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    if (firstByteMs === null) firstByteMs = Math.round(performance.now() - startedAt);
    buffer += decoder.decode(value, { stream: true });
    let newline;
    while ((newline = buffer.indexOf("\n")) >= 0) {
      consumeLine(buffer.slice(0, newline).replace(/\r$/, ""));
      buffer = buffer.slice(newline + 1);
    }
  }
  buffer += decoder.decode();
  if (buffer) consumeLine(buffer.replace(/\r$/, ""));
  return {
    text,
    usage,
    firstByteMs: firstByteMs ?? Math.round(performance.now() - startedAt),
    totalMs: Math.round(performance.now() - startedAt),
  };
}

function responseErrorText(raw) {
  try {
    const parsed = JSON.parse(raw);
    return String(parsed?.error?.message || parsed?.error || parsed?.message || raw);
  } catch {
    return String(raw || "");
  }
}

function publicPrefix(prefix) {
  if (!prefix) return null;
  return { fingerprint: shortHash(prefix.token), covered: prefix.covered };
}

async function sendWithWarming(options, body, apiKey) {
  const encoded = JSON.stringify(body);
  const requestBytes = Buffer.byteLength(encoded);
  if (requestBytes > SAFE_BODY_BYTES) {
    throw new Error(`request is ${requestBytes} bytes; refusing to approach the ${BODY_LIMIT_BYTES}-byte server cap`);
  }
  const attempts = [];
  const warmingStarted = performance.now();
  let warmingRetries = 0;
  let transientRetries = 0;
  while (true) {
    const startedAt = performance.now();
    let response;
    try {
      response = await fetch(`${options.baseUrl}/v1/chat/completions`, {
        method: "POST",
        headers: {
          authorization: `Bearer ${apiKey}`,
          "content-type": "application/json",
          "x-michael-compression": options.tier,
          "x-context-eval-run": options.runId,
        },
        body: encoded,
        signal: AbortSignal.timeout(options.requestTimeoutMs),
      });
    } catch (error) {
      throw new Error(`request transport failed: ${error?.message || error}`);
    }
    const headerMs = Math.round(performance.now() - startedAt);
    const payload = await readResponse(response, startedAt);
    const attempt = {
      status: response.status,
      headerMs,
      firstByteMs: payload.firstByteMs,
      totalMs: payload.totalMs,
    };
    attempts.push(attempt);
    const errorText = response.ok ? "" : responseErrorText(payload.text);
    const warming = response.status === 503 && errorText.includes("michael-compression warming");
    const transient = [502, 503, 504].includes(response.status);
    if (warming || transient) {
      const elapsed = performance.now() - warmingStarted;
      if (warming) warmingRetries += 1;
      else transientRetries += 1;
      const timeoutMs = warming ? options.warmTimeoutMs : options.transientTimeoutMs;
      if (elapsed >= timeoutMs) {
        const kind = warming ? "compression warming" : `transient HTTP ${response.status}`;
        throw new Error(`${kind} exceeded ${timeoutMs}ms after ${warming ? warmingRetries : transientRetries} retries`);
      }
      const retryAfter = Number(response.headers.get("retry-after"));
      const count = warming ? warmingRetries : transientRetries;
      const waitMs = Number.isFinite(retryAfter) && retryAfter > 0
        ? Math.min(30_000, retryAfter * 1_000)
        : Math.min(15_000, options.retryMs * Math.max(1, Math.ceil(count / 4)));
      const label = warming ? "warming" : `transient HTTP ${response.status}`;
      console.error(`[${options.tier}] ${label} retry ${count}; next check in ${waitMs}ms`);
      await new Promise((resolveDelay) => setTimeout(resolveDelay, waitMs));
      continue;
    }
    if (!response.ok) {
      throw new Error(`gateway returned HTTP ${response.status}: ${errorText.slice(0, 800)}`);
    }
    const appliedTier = response.headers.get("x-michael-compression-applied");
    if (appliedTier !== options.tier) {
      throw new Error(`requested ${options.tier}, but gateway applied ${appliedTier || "no compression tier"}`);
    }
    const token = response.headers.get("x-michael-compression-prefix");
    const coveredRaw = response.headers.get("x-michael-compression-covered");
    let issuedPrefix = null;
    if (token || coveredRaw) {
      const covered = Number(coveredRaw);
      if (!/^mcp_[a-f0-9]{32}$/i.test(token || "") || !Number.isSafeInteger(covered) || covered <= 0) {
        throw new Error("gateway returned an invalid compression prefix contract");
      }
      issuedPrefix = { token, covered };
    }
    return {
      requestBytes,
      attempts,
      warmingRetries,
      transientRetries,
      appliedTier,
      issuedPrefix,
      usage: payload.usage,
      text: payload.text,
      headerMs,
      firstByteMs: payload.firstByteMs,
      totalMs: payload.totalMs,
    };
  }
}

function shellQuote(value) {
  return `'${String(value).replace(/'/g, `'"'"'`)}'`;
}

function recoveryConfig() {
  const target = process.env.MICHAEL_EVAL_SSH_TARGET || "";
  const key = process.env.MICHAEL_EVAL_SSH_KEY || "";
  const composeDir = process.env.MICHAEL_EVAL_COMPOSE_DIR || "";
  if (!target || !key || !composeDir) {
    throw new Error("PostgreSQL recovery verification requires MICHAEL_EVAL_SSH_TARGET, MICHAEL_EVAL_SSH_KEY, and MICHAEL_EVAL_COMPOSE_DIR");
  }
  if (!/^[a-zA-Z0-9_.-]+@[a-zA-Z0-9_.:-]+$/.test(target)) throw new Error("invalid MICHAEL_EVAL_SSH_TARGET");
  if (!composeDir.startsWith("/") || /[\r\n]/.test(composeDir)) throw new Error("invalid MICHAEL_EVAL_COMPOSE_DIR");
  const keyPath = resolve(key.startsWith("~/") ? `${homedir()}/${key.slice(2)}` : key);
  return { target, keyPath, composeDir };
}

function sshRun(config, remoteCommand) {
  const result = spawnSync("ssh", [
    "-o", "BatchMode=yes",
    "-o", "ConnectTimeout=15",
    "-i", config.keyPath,
    config.target,
    remoteCommand,
  ], { encoding: "utf8", maxBuffer: 4 * 1024 * 1024 });
  if (result.status !== 0) {
    throw new Error(`scoped Redis eviction failed over SSH: ${(result.stderr || "unknown error").trim().slice(0, 500)}`);
  }
  return result.stdout.trim();
}

function postgresRecord(config, prefixToken) {
  if (!/^mcp_[a-f0-9]{32}$/i.test(prefixToken)) throw new Error("refusing recovery check for invalid prefix token");
  const sql = `SELECT record::text FROM michael_context_prefixes WHERE token = '${prefixToken}' LIMIT 1`;
  const inner = `psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c ${shellQuote(sql)}`;
  const command = `cd ${shellQuote(config.composeDir)} && docker compose exec -T postgres sh -lc ${shellQuote(inner)}`;
  const output = sshRun(config, command);
  const line = output.split("\n").filter(Boolean).at(-1);
  if (!line) throw new Error("evaluation prefix was not found in PostgreSQL");
  const record = JSON.parse(line);
  if (!Array.isArray(record.segment_keys) || !Array.isArray(record.raw_segment_keys)) {
    throw new Error("PostgreSQL prefix record is incomplete");
  }
  return record;
}

function evictEvaluationContext(prefix) {
  const config = recoveryConfig();
  const record = postgresRecord(config, prefix.token);
  const keys = new Set([`mc:prefix:${prefix.token}`]);
  for (const key of record.segment_keys) keys.add(key);
  for (const key of record.raw_segment_keys) {
    keys.add(key);
    keys.add(`mc:search:v1:${String(key).split(":").at(-1)}`);
  }
  for (const key of keys) {
    if (!/^mc:[a-z0-9:_-]+$/i.test(key)) throw new Error("refusing to delete an unexpected Redis key");
  }
  let deleted = 0;
  const allKeys = [...keys];
  for (let index = 0; index < allKeys.length; index += 100) {
    const chunk = allKeys.slice(index, index + 100);
    const command = `cd ${shellQuote(config.composeDir)} && docker compose exec -T redis redis-cli DEL ${chunk.map(shellQuote).join(" ")}`;
    const output = sshRun(config, command);
    const count = Number(output.split("\n").filter(Boolean).at(-1));
    if (!Number.isFinite(count)) throw new Error("Redis did not return a deletion count");
    deleted += count;
  }
  return { requestedKeys: allKeys.length, deletedKeys: deleted, durableSegments: record.raw_segment_keys.length };
}

function parseRecall(text, facts) {
  const expected = Object.fromEntries(facts.map((fact) => [fact.key, fact.value]));
  expected.hop_chain = `${expected.hop_origin}::${expected.hop_bridge}::${expected.hop_terminal}`;
  let parsed = null;
  const match = String(text || "").match(/\{[\s\S]*\}/);
  if (match) {
    try { parsed = JSON.parse(match[0]); } catch { /* exact-value checks below still apply */ }
  }
  const fields = Object.fromEntries(Object.entries(expected).map(([key, value]) => {
    return [key, parsed?.[key] === value || String(text || "").includes(value)];
  }));
  return {
    passed: Object.values(fields).every(Boolean),
    fields,
    validJson: !!parsed,
    response: String(text || "").slice(0, 12_000),
  };
}

function dryRunTier(options, fixture, batches) {
  const system = systemMessage(options.runId);
  const requestSizes = batches.map((batch) => {
    const body = requestBody(options, [system, ...batch.messages], null);
    return Buffer.byteLength(JSON.stringify(body));
  });
  if (requestSizes.some((bytes) => bytes > SAFE_BODY_BYTES)) {
    throw new Error(`${options.tier} dry run produced a request above the safe body cap`);
  }
  return {
    tier: options.tier,
    mode: "dry-run",
    sourceTokenEstimate: fixture.generatedTokens,
    messageCount: fixture.messages.length,
    batchCount: batches.length,
    largestUncompressedBatchBytes: Math.max(...requestSizes),
    serverBodyLimitBytes: BODY_LIMIT_BYTES,
    reusablePrefixDigest: reusablePrefixDigest(fixture, options.blockTokens, options.targetRatio),
    factPositions: fixture.facts.map(({ key, ratio, tokenPosition }) => ({ key, ratio, tokenPosition })),
  };
}

async function executeTier(options, fixture, batches, apiKey) {
  const logical = [];
  const system = systemMessage(options.runId);
  let prefix = options.resumePrefix
    ? { token: options.resumePrefix, covered: options.resumeCovered }
    : null;
  const checkpoints = [];
  let cumulativeTokens = 0;
  const startIndex = options.resumeAtCheckpoint ? options.resumeAtCheckpoint - 1 : 0;

  if (startIndex > batches.length) {
    throw new Error(`--resume-at-checkpoint must be between 1 and ${batches.length + 1}`);
  }
  for (let index = 0; index < startIndex; index += 1) {
    logical.push(...batches[index].messages);
    cumulativeTokens += batches[index].tokens;
  }
  if (prefix && prefix.covered > logical.length) {
    throw new Error(`resume prefix covers ${prefix.covered} messages, but only ${logical.length} exist before checkpoint ${options.resumeAtCheckpoint}`);
  }
  const resumedFrom = prefix ? {
    firstCheckpoint: options.resumeAtCheckpoint,
    cumulativeSourceTokens: cumulativeTokens,
    reconstructedMessages: logical.length,
    prefix: publicPrefix(prefix),
  } : null;
  if (resumedFrom) {
    console.error(`[${options.tier}] resuming at checkpoint ${resumedFrom.firstCheckpoint}/${batches.length}: source=${cumulativeTokens}, reconstructed_messages=${logical.length}, prefix_covered=${prefix.covered}`);
  }

  for (let index = startIndex; index < batches.length; index += 1) {
    const batch = batches[index];
    logical.push(...batch.messages);
    cumulativeTokens += batch.tokens;
    const suffix = prefix ? logical.slice(prefix.covered) : logical;
    const body = requestBody(options, [system, ...suffix], prefix);
    console.error(`[${options.tier}] checkpoint ${index + 1}/${batches.length}: source=${cumulativeTokens}, upload_messages=${suffix.length}`);
    const response = await sendWithWarming(options, body, apiKey);
    if (response.issuedPrefix) prefix = response.issuedPrefix;
    if (!prefix && index < batches.length - 1) {
      throw new Error("gateway did not issue a prefix before the next incremental upload");
    }
    checkpoints.push({
      index: index + 1,
      cumulativeSourceTokens: cumulativeTokens,
      uploadedMessages: suffix.length,
      requestBytes: response.requestBytes,
      warmingRetries: response.warmingRetries,
      transientRetries: response.transientRetries,
      attempts: response.attempts,
      headerMs: response.headerMs,
      firstByteMs: response.firstByteMs,
      totalMs: response.totalMs,
      usage: response.usage,
      appliedTier: response.appliedTier,
      prefix: publicPrefix(prefix),
      ack: response.text.includes("MC_EVAL_ACK"),
    });
  }

  if (!prefix) throw new Error("gateway never issued a compression prefix");
  let recovery = null;
  if (options.verifyPgRecovery) {
    console.error(`[${options.tier}] evicting only this evaluation prefix's Redis context keys`);
    recovery = evictEvaluationContext(prefix);
  }

  logical.push(recallMessage(fixture.facts));
  const suffix = logical.slice(prefix.covered);
  const recallBody = requestBody(options, [system, ...suffix], prefix);
  const response = await sendWithWarming(options, recallBody, apiKey);
  if (response.issuedPrefix) prefix = response.issuedPrefix;
  const recall = parseRecall(response.text, fixture.facts);
  console.error(`[${options.tier}] recall ${recall.passed ? "PASS" : "FAIL"}`);
  return {
    tier: options.tier,
    mode: "execute",
    model: options.model,
    sourceTokenEstimate: fixture.generatedTokens,
    messageCount: fixture.messages.length,
    batchCount: batches.length,
    reusablePrefixDigest: reusablePrefixDigest(fixture, options.blockTokens, options.targetRatio),
    factPositions: fixture.facts.map(({ key, ratio, tokenPosition }) => ({ key, ratio, tokenPosition })),
    resumedFrom,
    checkpoints,
    recovery,
    recall: {
      ...recall,
      requestBytes: response.requestBytes,
      warmingRetries: response.warmingRetries,
      transientRetries: response.transientRetries,
      attempts: response.attempts,
      headerMs: response.headerMs,
      firstByteMs: response.firstByteMs,
      totalMs: response.totalMs,
      usage: response.usage,
      appliedTier: response.appliedTier,
      prefix: publicPrefix(prefix),
    },
  };
}

async function writeReport(path, report) {
  const output = resolve(path);
  await mkdir(dirname(output), { recursive: true });
  await writeFile(output, `${JSON.stringify(report, null, 2)}\n`, { mode: 0o600 });
  return output;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    process.stdout.write(usage());
    return;
  }
  if (options.verifyPgRecovery && !options.execute) {
    throw new Error("--verify-pg-recovery requires --execute");
  }
  if (options.resumePrefix && !options.execute) {
    throw new Error("resume requires --execute");
  }
  const apiKey = process.env.MICHAEL_EVAL_API_KEY || "";
  if (options.execute && !apiKey) {
    throw new Error("--execute requires MICHAEL_EVAL_API_KEY");
  }

  const startedAt = new Date();
  const report = {
    schemaVersion: 1,
    mode: options.execute ? "execute" : "dry-run",
    startedAt: startedAt.toISOString(),
    baseUrl: options.baseUrl,
    model: options.model,
    targetRatio: options.targetRatio,
    batchTokens: options.batchTokens,
    tiers: [],
  };

  for (const tier of options.tiers) {
    const runId = options.runId
      || `${tier.replace("m", "M")}_${shortHash(`${randomUUID()}:${startedAt.toISOString()}`, 10)}`;
    const tierOptions = { ...options, tier, runId };
    const targetTokens = targetTokensForTier(tier, options.targetRatio);
    const fixture = makeFixture(tier, targetTokens, runId, options.blockTokens, options.targetRatio);
    const batches = batchMessages(fixture.messages, options.batchTokens);
    const result = options.execute
      ? await executeTier(tierOptions, fixture, batches, apiKey)
      : dryRunTier(tierOptions, fixture, batches);
    report.tiers.push(result);
  }
  report.completedAt = new Date().toISOString();
  report.passed = options.execute ? report.tiers.every((tier) => tier.recall?.passed) : true;

  const defaultName = `${startedAt.toISOString().replace(/[:.]/g, "-")}-${options.tiers.join("-")}.json`;
  const outputPath = options.output || `context-eval-results/${defaultName}`;
  const written = await writeReport(outputPath, report);
  process.stdout.write(`${JSON.stringify({
    mode: report.mode,
    passed: report.passed,
    tiers: report.tiers.map((tier) => ({
      tier: tier.tier,
      sourceTokenEstimate: tier.sourceTokenEstimate,
      batchCount: tier.batchCount,
      largestUncompressedBatchBytes: tier.largestUncompressedBatchBytes,
      recallPassed: tier.recall?.passed,
    })),
    report: written,
  }, null, 2)}\n`);
  if (!report.passed) process.exitCode = 1;
}

main().catch((error) => {
  process.stderr.write(`context-eval: ${error?.message || error}\n`);
  process.exitCode = 1;
});
