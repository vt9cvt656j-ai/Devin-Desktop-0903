/* eslint-disable */
// ⚠️ 生成文件，不要手改。
//
// 源：server/web-shared/mse.ts
// 重新生成：node server/scripts/sync-mse-client.mjs
//
// 手改这里的后果：test/mse-sync.test.mjs 会红，而且在它红之前，这一个前端和另外两个
// 已经在跑不同版本的密码学代码了。

/**
 * MSE-1 客户端 —— 请求与响应的应用层加密。
 *
 * 协议规范在 `server/docs/MSE.md`，服务端实现在 `server/src/mse.rs`，两边的字节级
 * 一致性由 `server/testdata/mse-vectors.json` 钉死。
 *
 * 这一份是**唯一的源**。官网、用户后台、管理后台里的 `src/lib/mse.ts` 都是它的副本，
 * 由 `server/scripts/sync-mse-client.mjs` 生成，`test/mse-sync.test.mjs` 守着不许漂移
 * —— 三份手抄的密码学代码里有一份改漏了，表现是那一个前端偶尔解不开，最难查。
 *
 * # 用法
 *
 * 调用点只改一个词：`fetch(...)` → `mseFetch(...)`。返回的是一个正常的 `Response`，
 * `res.ok` / `res.status` / `res.json()` 全都照旧 —— 这是让三个应用能低风险迁移的
 * 关键，否则每一个调用点都要重写错误处理。
 *
 * # 密码学
 *
 * ECDH P-384 + HKDF-SHA-384 + AES-256-GCM（CNSA 1.0）。全部走 WebCrypto，所以包里
 * 一行第三方密码学代码都没有 —— 「没有第三方密码学库」本身就是一条安全性质。
 *
 * 私钥用 `extractable: false` 生成：它连本页的 JS 都导不出去，只能拿去做运算。挡不住
 * 用户自己（用户能直接读解密后的数据），但确实挡住了 XSS 把密钥偷走再离线解流量。
 */

export const MSE_SUITE = "MSE1-P384-HKDF-SHA384-AES256GCM";
const FORMAT_VERSION = 1;
const NONCE_LEN = 12;
const SEALED_CT = "application/mse-sealed";
/** 加密流的结束标记。和 mse.rs 里 seal_frame 封的那一串必须逐字节相同。 */
const EOS_FRAME = '{"__mse_eos":true}';

/** 客户端绝不能加密的路径：引导端点自己被加密就成了循环依赖。 */
function clientExempt(path: string): boolean {
  return path.startsWith("/api/crypto/");
}

export type MseMode = "auto" | "require";

export type MseConfig = {
  /** 网关源，`""` 表示同源。官网跨源打 https://code.mrday.one。 */
  base?: string;
  /**
   * 认可的服务端 kid 列表。非空时，对面报了别的 kid 就直接拒绝，没有回退。
   *
   * 这一条是整套东西里最值钱的：不固定密钥，只挡得住被动的中间人（Cloudflare、
   * nginx 日志、代理）；固定了密钥，连装了受信根证书的**主动**中间人也挡住了。
   */
  pin?: string[];
  /** `require` = 永不明文回退；密码学不可用就抛错，而不是把明文发出去。 */
  mode?: MseMode;
  /**
   * `true` = **强制前向保密**：网关没给出可信的临时密钥（eph 被剥掉、或验签失败）就
   * 直接报错，绝不回退到静态 ECDH。
   *
   * 为什么需要它：eph 是可选 JSON 字段，主动中间人**不需要任何密钥**就能把它从
   * pubkey 响应里删掉，逼客户端走无前向保密的静态路径——之后一旦静态私钥泄露，录下来
   * 的这段流量就能被解开，正好是前向保密要消灭的那件事。开了这一档，剥 eph 只会让连接
   * **失败**（可见），而不是**静默降级**（不可见）。代价：网关必须确实在发 eph（我们
   * 自己的网关一直在发），否则连不上。所以登录页刻意不开这一档（可用性优先），SPA 开。
   */
  requireFs?: boolean;
  /** 要封进密文的请求头（小写）。默认把 Bearer 令牌和地区信号封起来。 */
  sealHeaders?: string[];
  fetchImpl?: typeof fetch;
};

const DEFAULT_SEAL_HEADERS = [
  // 会话令牌。封进去之后它不再出现在 Cloudflare、nginx 日志和任何中间人眼里。
  //
  // 注意同源请求上浏览器仍然会自动带 `mide_token` cookie —— nginx 的 auth_request
  // 门禁要读它，删不掉。所以这一条对**官网**（跨源，SameSite=Lax 不发 cookie）是
  // 完整的收益，对同源的后台是部分收益。
  "authorization",
  // 中国区判定的三个信号，见 account-ui 的 regionSignals()。
  "x-ide-language",
  "x-ide-timezone",
  "x-ide-utc-offset-minutes",
];

let cfg: Required<Omit<MseConfig, "fetchImpl">> & { fetchImpl: typeof fetch } = {
  base: "",
  pin: [],
  mode: "auto",
  requireFs: false,
  sealHeaders: DEFAULT_SEAL_HEADERS,
  fetchImpl: (...a: Parameters<typeof fetch>) => globalThis.fetch(...a),
};

export function configureMse(next: MseConfig): void {
  cfg = {
    base: next.base ?? cfg.base,
    pin: next.pin ?? cfg.pin,
    mode: next.mode ?? cfg.mode,
    requireFs: next.requireFs ?? cfg.requireFs,
    sealHeaders: (next.sealHeaders ?? cfg.sealHeaders).map((h) => h.toLowerCase()),
    fetchImpl: next.fetchImpl ?? cfg.fetchImpl,
  };
  // 配置一变，之前引导出来的东西可能指向另一个网关。全部作废重来。
  boot = null;
  session = null;
}

/** 从 Vite 构建参数里取默认配置。三个前端各自 `configureMse(mseEnvConfig())` 一次。 */
export function mseEnvConfig(): MseConfig {
  let env: Record<string, string | undefined> = {};
  try {
    env = ((import.meta as unknown as { env?: Record<string, string> }).env ?? {}) as Record<
      string,
      string | undefined
    >;
  } catch {
    /* 非 Vite 环境（node 测试）下没有 import.meta.env，用默认值 */
  }
  const pin = (env.VITE_MSE_PIN ?? "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  return {
    base: env.VITE_MSE_BASE ?? "",
    pin,
    mode: env.VITE_MSE_MODE === "require" ? "require" : "auto",
    // VITE_MSE_REQUIRE_FS=1 的构建强制前向保密：剥掉 eph 只会连不上，不会静默降级。
    requireFs: env.VITE_MSE_REQUIRE_FS === "1" || env.VITE_MSE_REQUIRE_FS === "true",
  };
}

export class MseError extends Error {
  constructor(
    message: string,
    readonly code: string,
  ) {
    super(message);
    this.name = "MseError";
  }
}

// ---------------------------------------------------------------------------
// 编码
// ---------------------------------------------------------------------------

const enc = new TextEncoder();
const dec = new TextDecoder();

function b64uEncode(bytes: Uint8Array<ArrayBufferLike>): string {
  let s = "";
  // 分块，避免大 body 上 String.fromCharCode(...spread) 爆栈。
  const CHUNK = 0x8000;
  for (let i = 0; i < bytes.length; i += CHUNK) {
    s += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
  }
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function b64uDecode(text: string): Uint8Array<ArrayBuffer> {
  const t = text.replace(/-/g, "+").replace(/_/g, "/");
  const raw = atob(t + "=".repeat((4 - (t.length % 4)) % 4));
  const out = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
  return out;
}

/**
 * `\0` 分隔的 AAD。和 Rust 的 `join_nul` 必须逐字节一致。
 *
 * 用分隔符而不是直接拼，是为了让字段边界无法平移：否则 `sid="a", seq=12` 和
 * `sid="a1", seq=2` 会算出同一段 AAD。
 */
function joinNul(...parts: string[]): Uint8Array<ArrayBuffer> {
  const chunks = parts.map((p) => enc.encode(p));
  const total = chunks.reduce((n, c) => n + c.length, 0) + (chunks.length - 1);
  const out = new Uint8Array(total);
  let at = 0;
  chunks.forEach((c, i) => {
    if (i > 0) out[at++] = 0;
    out.set(c, at);
    at += c.length;
  });
  return out;
}

const aadReq = (sid: string, seq: number, ts: number, method: string, path: string) =>
  joinNul("MSE1/req", sid, String(seq), String(ts), method, path);
const aadRes = (sid: string, seq: number, path: string) =>
  joinNul("MSE1/res", sid, String(seq), path);
const aadSse = (sid: string, seq: number, frame: number) =>
  joinNul("MSE1/sse", sid, String(seq), String(frame));

// ---------------------------------------------------------------------------
// 引导与会话
// ---------------------------------------------------------------------------

type Boot = {
  kid: string;
  pub: string;
  prev: { kid: string; pub: string } | null;
  /**
   * 服务端的轮换临时密钥（前向保密）。`sig` 是**静态密钥**对它的 ECDSA-P384 签名。
   * 老服务端不带这个字段——那时回退到静态密钥做 ECDH（无前向保密但能用）。
   */
  eph?: { id: string; pub: string; exp: number; sig: string } | null;
  mode: "off" | "optional" | "required";
  session_ttl: number;
  max_skew_ms: number;
  server_time: number;
  suite: string;
  /** server_time - Date.now()。本地时钟不准时，靠它把每个请求的时间戳纠回来。 */
  offset: number;
  /**
   * **实际用于 ECDH 的服务端密钥**：验签通过且没过期的临时公钥，否则回退到静态公钥。
   * loadBoot 里算好，derive 直接用。activeKid 进 X-Mse-Kid 和 HKDF info。
   */
  activeKid: string;
  activePub: string;
  /** true = 用的是临时密钥（有前向保密）；false = 回退到了静态密钥。诊断用。 */
  forwardSecret: boolean;
};

/** i64 大端 8 字节。和 Rust 的 `exp_ms.to_be_bytes()` 必须一致。 */
function i64be(n: number): Uint8Array<ArrayBuffer> {
  const b = new Uint8Array(8);
  new DataView(b.buffer).setBigInt64(0, BigInt(Math.trunc(n)), false);
  return b;
}

/** 签名的域前缀。和 Rust 的 `EPH_SIG_CTX` 逐字节一致（含结尾的 NUL）。 */
const EPH_SIG_CTX: Uint8Array<ArrayBuffer> = new Uint8Array([...enc.encode("MSE-EPH-v1"), 0]);

/**
 * 验证临时密钥的签名，通过就返回它，否则返回 null（调用方回退静态密钥）。
 *
 * 信任链：钉住的静态密钥 → 用它验这个签名 → 签名背书了临时公钥 → 临时公钥做 ECDH。
 * 于是偷到静态私钥也解不了流量（它只签过名），而临时私钥会轮换、用完即弃。
 */
async function verifyEph(
  staticSpki: Uint8Array<ArrayBuffer>,
  eph: { id: string; pub: string; exp: number; sig: string },
  offset: number,
): Promise<{ id: string; pub: string } | null> {
  try {
    // 过期的临时密钥不用（用服务端时间基准判，避免本地时钟偏差误杀/误放）。
    if (eph.exp <= Date.now() + offset) return null;

    const ephSpki = b64uDecode(eph.pub);
    const sig = b64uDecode(eph.sig); // 96 字节 r||s
    // id 必须是临时公钥的真实指纹，不采信服务端报的字符串（和静态 kid 同理）。
    if ((await kidOf(ephSpki)) !== eph.id) return null;

    const verifyKey = await crypto.subtle.importKey(
      "spki",
      staticSpki,
      { name: "ECDSA", namedCurve: "P-384" },
      false,
      ["verify"],
    );
    const msg = concat(EPH_SIG_CTX, ephSpki, i64be(eph.exp));
    const ok = await crypto.subtle.verify({ name: "ECDSA", hash: "SHA-384" }, verifyKey, sig, msg);
    return ok ? { id: eph.id, pub: eph.pub } : null;
  } catch {
    return null;
  }
}

type Session = {
  kid: string;
  sid: string;
  epk: string;
  c2s: CryptoKey;
  s2c: CryptoKey;
  seq: number;
  expiresAt: number;
};

let boot: Promise<Boot> | null = null;
let session: Promise<Session> | null = null;

function url(path: string): string {
  return cfg.base ? `${cfg.base}${path}` : path;
}

/**
 * kid = base64url(SHA-384(SPKI))[..24]。和 Rust 的 `kid_of` 必须一致。
 *
 * 客户端要**自己算**，绝不能采信服务端报的那个字符串 —— 见 loadBoot 里的说明。
 */
async function kidOf(spki: Uint8Array<ArrayBuffer>): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-384", spki));
  return b64uEncode(digest).slice(0, 24);
}

async function loadBoot(): Promise<Boot> {
  const res = await cfg.fetchImpl(url("/api/crypto/pubkey"), { cache: "no-store" });
  if (!res.ok) throw new MseError(`取公钥失败 (${res.status})`, "bootstrap");
  const j = (await res.json()) as Omit<Boot, "offset">;
  if (j.suite !== MSE_SUITE) {
    throw new MseError(`网关用的是 ${j.suite}，这个客户端只会 ${MSE_SUITE}`, "suite");
  }

  /*
   * kid 一律由本地从公钥重算，**不采信响应里那个字符串**。
   *
   * 这里曾经只拿 j.kid 和固定名单比字符串，于是整套密钥固定是假的：中间人把
   * 诚实的 kid 原样报上来、公钥换成自己的，字符串比对照过不误，而 derive() 拿去做
   * ECDH 的是 b64uDecode(j.pub) —— 攻击者那一把。transcript 绑定也救不了，因为
   * tx = SHA-384(epk || server_spki) 里的 server_spki 就是攻击者自己的公钥，它算得出
   * 一模一样的值。审查时这条攻击是**实际跑通**的，不是理论上的。
   *
   * 重算之后，kid 变成公钥的指纹而不是一个自述的标签，替换公钥必然改变 kid。
   */
  const spki = b64uDecode(j.pub);
  const realKid = await kidOf(spki);
  if (realKid !== j.kid) {
    throw new MseError(`网关自称 ${j.kid}，但公钥的指纹是 ${realKid}`, "pin");
  }
  let prevKid: string | null = null;
  if (j.prev) {
    prevKid = await kidOf(b64uDecode(j.prev.pub));
    if (prevKid !== j.prev.kid) {
      throw new MseError(`网关上一把密钥的 kid 和公钥对不上`, "pin");
    }
  }

  // `spki` 是响应里**最初的当前静态公钥**（下面 pin 块可能把 j.pub 改成 prev，但这个
  // 变量已经在那之前解好了）。eph 的签名永远由当前静态密钥出，所以验签必须用它。
  const currentStaticSpki = spki;

  // 密钥固定。不在名单里就拒绝，**不回退明文** —— 回退等于把固定密钥变成装饰。
  // currentTrusted：客户端是否信任「当前静态密钥」。没配 pin 就都信；配了 pin 只信
  // 名单里的那把。这决定了敢不敢用当前静态密钥背书的 eph（见下）。
  let currentTrusted = cfg.pin.length === 0;
  if (cfg.pin.length > 0) {
    const pinnedCurrent = cfg.pin.includes(realKid);
    const pinnedPrev = prevKid !== null && cfg.pin.includes(prevKid);
    if (!pinnedCurrent && !pinnedPrev) {
      throw new MseError(`网关公钥 ${realKid} 不在固定名单里，拒绝连接`, "pin");
    }
    currentTrusted = pinnedCurrent;
    // 轮换期：名单里的那一把优先，即使服务端把它标成 prev。
    if (!pinnedCurrent && j.prev) {
      j.kid = j.prev.kid;
      j.pub = j.prev.pub;
    }
  }

  lastKid = j.kid;
  const offset = j.server_time - Date.now();

  /*
   * 前向保密：静态密钥（信任锚）只用来**验签**一把轮换的临时密钥；真正做 ECDH 的是那把
   * 临时公钥。偷到静态私钥也解不了历史流量——它从没握过 DH。
   *
   * 只有在**信任当前静态密钥**时才用 eph：eph 由当前静态密钥签名，若客户端只 pin 了
   * prev（当前那把不在名单里），就无从确认签名者可信 —— 这时回退到 prev 的静态 ECDH。
   */
  let activeKid = j.kid;
  let activePub = j.pub;
  let forwardSecret = false;
  if (j.eph && currentTrusted) {
    const verified = await verifyEph(currentStaticSpki, j.eph, offset);
    if (verified) {
      activeKid = verified.id;
      activePub = verified.pub;
      forwardSecret = true;
    }
    // 验不过就回退静态密钥：仍受 pin 保护、挡得住主动中间人，只是这一条会话没有前向
    // 保密。不抛错——老服务端或部署间隙都可能暂时给不出合法 eph。
  }

  // 强制前向保密：拿不到可信的临时密钥就**报错**，不静默降级到静态 ECDH。这把
  // 「主动中间人剥掉 eph → 悄悄降级 → 将来静态私钥泄露就能解密」变成一个可见的失败。
  // 剥 eph 不需要密钥，所以没有这道闸，pin + require 都拦不住这次降级。
  if (cfg.requireFs && !forwardSecret) {
    throw new MseError(
      "此构建要求前向保密，但网关没有提供可信的临时密钥（可能是 eph 被中间人剥掉）",
      "fs",
    );
  }

  lastForwardSecret = forwardSecret;
  return { ...j, offset, activeKid, activePub, forwardSecret };
}

function bootstrap(): Promise<Boot> {
  if (!boot) {
    boot = loadBoot().catch((e) => {
      boot = null; // 失败不缓存，下一次调用重试
      throw e;
    });
  }
  return boot;
}

async function derive(b: Boot): Promise<Session> {
  const pair = (await crypto.subtle.generateKey(
    { name: "ECDH", namedCurve: "P-384" },
    // 私钥不可导出：本页的 JS 也拿不走，只能拿去做运算。挡不住用户自己看解密后的
    // 数据，但挡住了 XSS 偷走密钥去离线解历史流量。
    false,
    ["deriveBits"],
  )) as CryptoKeyPair;

  const epkSpki = new Uint8Array(await crypto.subtle.exportKey("spki", pair.publicKey));
  // 用**活跃密钥**做 ECDH：验签通过就是临时公钥（前向保密），否则回退的静态公钥。
  // 服务端那边靠 activeKid 走对应分支（临时密钥环 or 静态密钥）。
  const serverSpki = b64uDecode(b.activePub);
  const serverPub = await crypto.subtle.importKey(
    "spki",
    serverSpki,
    { name: "ECDH", namedCurve: "P-384" },
    false,
    [],
  );

  // P-384 的 ECDH 共享秘密是 48 字节的 X 坐标，和 Rust 的 raw_secret_bytes() 一致。
  const z = new Uint8Array(
    await crypto.subtle.deriveBits({ name: "ECDH", public: serverPub }, pair.privateKey, 384),
  );

  // transcript 绑定：双方公钥都揉进 info。server_spki 是**真正做了 ECDH 的那把**（活跃
  // 密钥）。对面换了公钥不会得到一条能用的会话，只会得到一把不同的密钥。
  const tx = new Uint8Array(await crypto.subtle.digest("SHA-384", concat(epkSpki, serverSpki)));

  const prk = await crypto.subtle.importKey("raw", z, "HKDF", false, ["deriveBits"]);
  const expand = async (dir: "c2s" | "s2c") => {
    const info = concat(enc.encode(`MSE1/v1|${dir}|${b.activeKid}|`), tx);
    const bits = await crypto.subtle.deriveBits(
      { name: "HKDF", hash: "SHA-384", salt: new Uint8Array(0), info },
      prk,
      256,
    );
    return crypto.subtle.importKey("raw", bits, "AES-GCM", false, ["encrypt", "decrypt"]);
  };

  const epkHash = new Uint8Array(await crypto.subtle.digest("SHA-384", epkSpki));
  return {
    // 活跃密钥的 id 进 X-Mse-Kid —— 服务端据此选临时密钥环还是静态密钥来 ECDH。
    kid: b.activeKid,
    sid: b64uEncode(epkHash.subarray(0, 18)),
    epk: b64uEncode(epkSpki),
    c2s: await expand("c2s"),
    s2c: await expand("s2c"),
    seq: 0,
    // 提前 60 秒过期，免得请求正好卡在服务端的过期边界上。
    //
    // 下限 30 秒不是保守，是必须的：运维把 MSE_SESSION_TTL_SECS 设成 60 以下时，
    // `ttl - 60` 会让**刚建好的会话立刻就是过期的**，而 attempt() 遇到过期会重建会话
    // 再递归调用自己 —— 于是第一个请求就把页面卡死在无限递归里。
    expiresAt: Date.now() + Math.max(30, b.session_ttl - 60) * 1000,
  };
}

function ensureSession(): Promise<Session> {
  if (!session) {
    session = bootstrap()
      .then(derive)
      .catch((e) => {
        session = null;
        throw e;
      });
  }
  return session;
}

/** 丢掉当前会话和引导缓存。收到 `rekey` 时调用。 */
function resetSession(): void {
  boot = null;
  session = null;
}

function concat(...parts: Uint8Array<ArrayBuffer>[]): Uint8Array<ArrayBuffer> {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}

/** 提前把公钥取回来、密钥推好，让第一个业务请求不必等这一步。 */
export async function mseReady(): Promise<void> {
  await ensureSession();
}

/** 当前用的是哪一把服务端密钥。诊断用 —— 「它到底加密了没有」要能一眼看出来。 */
let lastKid: string | null = null;
/** 上一次引导时，是否用上了临时密钥（前向保密生效）。诊断用。 */
let lastForwardSecret = false;

export function mseStatus(): {
  ready: boolean;
  suite: string;
  kid: string | null;
  mode: MseMode;
  pinned: boolean;
  forwardSecret: boolean;
} {
  return {
    ready: session !== null,
    suite: MSE_SUITE,
    kid: lastKid,
    mode: cfg.mode,
    pinned: cfg.pin.length > 0,
    forwardSecret: lastForwardSecret,
  };
}

// ---------------------------------------------------------------------------
// AEAD
// ---------------------------------------------------------------------------

async function sealBytes(
  key: CryptoKey,
  aad: Uint8Array<ArrayBuffer>,
  plaintext: Uint8Array<ArrayBuffer>,
): Promise<Uint8Array<ArrayBuffer>> {
  const nonce = crypto.getRandomValues(new Uint8Array(NONCE_LEN));
  const ct = new Uint8Array(
    await crypto.subtle.encrypt(
      { name: "AES-GCM", iv: nonce, additionalData: aad, tagLength: 128 },
      key,
      plaintext,
    ),
  );
  const out = new Uint8Array(1 + NONCE_LEN + ct.length);
  out[0] = FORMAT_VERSION;
  out.set(nonce, 1);
  out.set(ct, 1 + NONCE_LEN);
  return out;
}

async function openBytes(
  key: CryptoKey,
  aad: Uint8Array<ArrayBuffer>,
  envelope: Uint8Array<ArrayBuffer>,
): Promise<Uint8Array<ArrayBuffer>> {
  // message 一律用短代号，不写协议细节。这些串会原样进构建产物，而「信封太短」
  // 「版本不认识」这类描述等于把信封格式画给读代码的人看。判断逻辑一律走 .code，
  // 所以收敛 message 不影响任何行为（见文件顶部注释）。
  if (envelope.length < 1 + NONCE_LEN + 16) throw new MseError("e.len", "malformed");
  if (envelope[0] !== FORMAT_VERSION) throw new MseError("e.ver", "malformed");
  try {
    return new Uint8Array(
      await crypto.subtle.decrypt(
        {
          name: "AES-GCM",
          iv: envelope.subarray(1, 1 + NONCE_LEN),
          additionalData: aad,
          tagLength: 128,
        },
        key,
        envelope.subarray(1 + NONCE_LEN),
      ),
    );
  } catch {
    throw new MseError("e.dec", "decrypt");
  }
}

// ---------------------------------------------------------------------------
// 请求
// ---------------------------------------------------------------------------

type SealedResponseInner = {
  s: number;
  b?: unknown;
  raw?: string | null;
  ct: string;
  h?: Record<string, string>;
};

/** body 转成信封内层的 `b`（JSON）或 `raw`+`ct`（其余一切）。 */
async function encodeBody(
  body: BodyInit | null | undefined,
  contentType: string | null,
): Promise<{ b?: unknown; raw?: string; ct?: string }> {
  if (body === null || body === undefined || body === "") return {};
  if (typeof body === "string") {
    // 调用方几乎总是 JSON.stringify 过的字符串。原样解回对象放进 `b`，服务端就能
    // 直接重建 JSON body，中间少一次字符串搬运。
    if (!contentType || contentType.includes("json")) {
      try {
        return { b: JSON.parse(body) };
      } catch {
        /* 声明是 JSON 却不是 —— 落到 raw */
      }
    }
    return { raw: b64uEncode(enc.encode(body)), ct: contentType ?? "text/plain;charset=UTF-8" };
  }
  if (body instanceof Uint8Array) {
    return { raw: b64uEncode(body), ct: contentType ?? "application/octet-stream" };
  }
  if (body instanceof ArrayBuffer) {
    return { raw: b64uEncode(new Uint8Array(body)), ct: contentType ?? "application/octet-stream" };
  }
  if (body instanceof Blob) {
    return {
      raw: b64uEncode(new Uint8Array(await body.arrayBuffer())),
      ct: contentType ?? body.type ?? "application/octet-stream",
    };
  }
  if (body instanceof URLSearchParams) {
    return {
      raw: b64uEncode(enc.encode(body.toString())),
      ct: "application/x-www-form-urlencoded;charset=UTF-8",
    };
  }
  if (typeof FormData !== "undefined" && body instanceof FormData) {
    // multipart 的分隔符由浏览器生成，拿不到原始字节。这类请求不封 —— 与其偷偷发
    // 一份坏掉的 body，不如让调用方知道。
    throw new MseError("e.form", "unsupported");
  }
  throw new MseError("e.body", "unsupported");
}

function splitUrl(input: string): { path: string; query: string; origin: string } {
  const absolute = /^https?:\/\//i.test(input);
  const u = new URL(input, absolute ? undefined : "http://placeholder.invalid");
  return {
    path: u.pathname,
    query: u.search.replace(/^\?/, ""),
    origin: absolute ? u.origin : "",
  };
}

/**
 * 一次发送的结果。`seq` 和 `key` 要交给流解码用 —— 帧的 AAD 里带着请求的序号，
 * 而序号是在 `attempt` 内部自增的，外面猜不到。
 */
type Sent = {
  res: Response;
  sid: string;
  seq: number;
  s2c: CryptoKey | null;
  /** true = 这一条最终是明文发出去的（豁免、降级或 auto 档回退）。 */
  plain: boolean;
};

/**
 * `fetch` 的加密替身。返回正常的 `Response`，调用点其余部分一律不用改。
 */
export async function mseFetch(input: string, init: RequestInit = {}): Promise<Response> {
  return (await sealedSend(input, init)).res;
}

async function sealedSend(input: string, init: RequestInit): Promise<Sent> {
  const target = input.startsWith("http") ? input : url(input);
  const { path } = splitUrl(target);

  if (clientExempt(path)) return plainSent(await cfg.fetchImpl(target, init));


  try {
    return await attempt(target, init, path, new Set());
  } catch (e) {
    if (cfg.mode === "require") throw e;
    // `auto` 档：加密这条路走不通（网关没开、浏览器没有 WebCrypto、公钥取不到）时
    // 退回明文，让产品继续可用。要杜绝这条路径，用 VITE_MSE_MODE=require 构建。
    if (e instanceof MseError && (e.code === "bootstrap" || e.code === "suite")) {
      return plainSent(await cfg.fetchImpl(target, init));
    }
    throw e;
  }
}

function plainSent(res: Response): Sent {
  return { res, sid: "", seq: 0, s2c: null, plain: true };
}

async function attempt(
  target: string,
  init: RequestInit,
  path: string,
  retried: Set<string>,
): Promise<Sent> {
  const b = await bootstrap();
  if (b.mode === "off") return plainSent(await cfg.fetchImpl(target, init));

  const s = await ensureSession();
  // 过期就换一条。只换一次 —— 上面那个下限已经保证新会话不会立刻过期，但这条递归
  // 是页面卡死的最短路径，值得再上一道闸，而不是依赖另一处的算术。
  if (Date.now() > s.expiresAt && !retried.has("expired")) {
    retried.add("expired");
    resetSession();
    return attempt(target, init, path, retried);
  }

  const method = (init.method ?? "GET").toUpperCase();
  const { query } = splitUrl(target);
  const outgoing = new Headers(init.headers ?? {});
  const contentType = outgoing.get("content-type");

  // 要封进密文的头，从外层摘掉再放进内层。摘掉这一步才是重点 —— 留在外层就等于没封。
  const sealedHeaders: Record<string, string> = {};
  for (const name of cfg.sealHeaders) {
    const v = outgoing.get(name);
    if (v !== null) {
      sealedHeaders[name] = v;
      outgoing.delete(name);
    }
  }

  const payload = {
    q: query,
    ...(await encodeBody(init.body, contentType)),
    ...(Object.keys(sealedHeaders).length ? { h: sealedHeaders } : {}),
  };

  const seq = ++s.seq;
  const ts = Date.now() + b.offset;
  const envelope = await sealBytes(
    s.c2s,
    aadReq(s.sid, seq, ts, method, path),
    enc.encode(JSON.stringify(payload)),
  );

  outgoing.set("x-mse-v", "1");
  outgoing.set("x-mse-kid", s.kid);
  outgoing.set("x-mse-epk", s.epk);
  outgoing.set("x-mse-sid", s.sid);
  outgoing.set("x-mse-seq", String(seq));
  outgoing.set("x-mse-ts", String(ts));

  // GET/HEAD 不能带 body（fetch 直接拒绝），信封只能走头部。
  const inHeader = method === "GET" || method === "HEAD";
  let body: BodyInit | null = null;
  if (inHeader) {
    outgoing.set("x-mse-q", b64uEncode(envelope));
    outgoing.delete("content-type");
  } else {
    outgoing.set("content-type", SEALED_CT);
    body = envelope;
  }

  // 外层 URL 不带 query —— query 已经封进信封了，这正是它从 nginx 日志里消失的原因。
  const bare = target.split("?")[0];
  const res = await cfg.fetchImpl(bare, { ...init, method, headers: outgoing, body });

  const sent = (r: Response): Sent => ({
    res: r,
    sid: s.sid,
    seq,
    s2c: s.s2c,
    plain: false,
  });

  const ct = res.headers.get("content-type") ?? "";
  if (ct.includes(SEALED_CT)) {
    return sent(await decodeSealed(res, s, seq, path));
  }

  // 明文回来了。MSE 层自己的错误一定是明文（否则状态坏掉的客户端读不懂原因）。
  let mseCode: string | undefined;
  if (res.status === 409 || res.status === 400 || res.status === 503) {
    const copy = res.clone();
    const j = (await copy.json().catch(() => null)) as { mse?: string; server_time?: number } | null;
    const code = j?.mse;
    mseCode = code;
    // 每类最多重试一次，总共最多两次 —— 时钟坏掉的客户端不能变成请求风暴。
    if (code && !retried.has(code) && retried.size < 2) {
      retried.add(code);
      if (code === "rekey" || code === "replay") {
        // rekey：服务端换密钥了。replay：序号撞了。两种都靠换一条全新会话解决
        // —— 新的临时密钥意味着新的 sid，序号从 1 重新开始，不会再撞。
        resetSession();
        return attempt(target, init, path, retried);
      }
      if (code === "skew") {
        if (typeof j?.server_time === "number") {
          (await bootstrap()).offset = j.server_time - Date.now();
        }
        return attempt(target, init, path, retried);
      }
      if (code === "unavailable" || code === "required") {
        // 服务端的 MSE_MODE 变了，而我们手里那份引导信息是开页面时取的。
        //
        // 少了这一条，运维把 MSE_MODE 改成 off 之后，所有已经打开的标签页会继续
        // 发密文、连续吃 503，直到会话过期才会重新引导 —— 按默认 1800 秒算，那是
        // 整整 29 分钟的白屏，而运维那边看不出和自己那次改动有关。
        resetSession();
        return attempt(target, init, path, retried);
      }
    }
  }

  /*
   * 我们发的是密文，回来的却是明文。
   *
   * 这里以前只在 require 档检查，而且**看响应头**决定要不要放行 —— 而响应头是没有
   * 认证的，攻击者随手加一个 `X-Mse-Downgrade: redirect` 就能让客户端接受一份伪造的
   * 明文 JSON。审查时用假网关实测：`res.json()` 拿到的就是攻击者构造的
   * `{"role":"admin"}`。那个头是诊断信息，不是授权凭据。
   *
   * 现在按「这份明文里有没有我们会当真的数据」来判，两个档位一视同仁：
   *   - 加密流：每一帧各自带 GCM tag，伪造的帧解不开，交给流解码器去炸即可
   *   - 3xx / 101：body 不是数据（重定向由浏览器跟，101 是协议升级）
   *   - 5xx：中间设施自己的错误页（Cloudflare 502 之类）。伪造它只是拒绝服务，
   *     传不进任何内容，放行反而能让上层显示「网关错误」而不是一个密码学错误
   * 其余一律拒绝 —— 2xx 和 4xx 是应用会照着行动的答案。
   */
  const sealedStream = ct.includes("text/event-stream") && res.headers.get("x-mse-stream") === "1";
  const notData = res.status >= 500 || res.status === 101 || (res.status >= 300 && res.status < 400);
  if (!sealedStream && !notData) {
    // 重试用尽的 MSE 错误走到这里。报它自己的原因，别报「回了明文」—— 后者是真的，
    // 但会让人去查中间人，而实际原因是密钥轮换没停、或者本机时钟一直对不上。
    if (mseCode) {
      throw new MseError(`${path}：加密层反复拒绝（${mseCode}），已放弃`, mseCode);
    }
    throw new MseError(
      `${path} 对一个加密请求回了明文（${res.status}）—— 不接受`,
      "downgrade",
    );
  }
  return sent(res);
}

async function decodeSealed(
  res: Response,
  s: Session,
  seq: number,
  path: string,
): Promise<Response> {
  const envelope = new Uint8Array(await res.arrayBuffer());
  const plaintext = await openBytes(s.s2c, aadRes(s.sid, seq, path), envelope);
  const inner = JSON.parse(dec.decode(plaintext)) as SealedResponseInner;

  const headers = new Headers(inner.h ?? {});
  headers.set("content-type", inner.ct);

  // 204/205/304 不允许带 body，构造 Response 时给 body 传 null 否则抛 TypeError。
  const bodyless = inner.s === 204 || inner.s === 205 || inner.s === 304;
  let body: BodyInit | null = null;
  if (!bodyless) {
    if ("b" in inner) body = JSON.stringify(inner.b);
    else if (inner.raw) body = b64uDecode(inner.raw);
  }
  // 状态码取内层那一个：它在密文里，被 GCM 的 tag 保护着。外层可能被
  // MSE_MASK_STATUS 抹成 200，也可能被中间人改过 —— 都不作数。
  return new Response(body, { status: inner.s, headers });
}

// ---------------------------------------------------------------------------
// 加密流
// ---------------------------------------------------------------------------

/**
 * 加密的 SSE。逐帧解出上游原本的事件块。
 *
 * 流正常结束时最后一帧是 `{"__mse_eos":true}`。**没有它就结束 = 这条流被截断了**，
 * 这里抛错而不是把半截答案当完整答案返回 —— 普通的 SSE 代理根本发现不了这件事。
 */
export async function* mseEventStream(
  input: string,
  init: RequestInit = {},
): AsyncGenerator<string, void, undefined> {
  // 序号是 sealedSend 内部自增的，而帧的 AAD 里带着它 —— 所以必须从发送结果里取回，
  // 不能在发送前先读一遍（那样每一帧都会差一个数，全部解不开）。
  const { res, sid, seq, s2c, plain } = await sealedSend(input, init);
  if (!res.ok || !res.body) {
    throw new MseError(`流打不开 (${res.status})`, "stream");
  }

  const reader = res.body.getReader();

  // 服务端没加密这条流（豁免路由、显式降级，或 auto 档回退）。原样交出去。
  if (plain || !s2c || !res.headers.get("x-mse-stream")) {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) return;
      yield dec.decode(value, { stream: true });
    }
  }

  let buf = "";
  let frame = 0;
  let sawEos = false;
  outer: for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += dec.decode(value, { stream: true });
    for (;;) {
      const cut = buf.indexOf("\n\n");
      if (cut < 0) break;
      const line = buf.slice(0, cut);
      buf = buf.slice(cut + 2);
      // `: ` 开头是 SSE 注释（服务端封帧失败时发的就是它）。跳过，让流最终以
      // 「没看到 EOS」收场 —— 那正是它想表达的意思。
      if (!line.startsWith("data: ")) continue;
      const data = line.slice(6).trim();
      if (!data) continue;
      const block = dec.decode(await openBytes(s2c, aadSse(sid, seq, frame), b64uDecode(data)));
      frame += 1;
      // 精确相等，不是 includes。服务端封的结束帧就是这一串字节（见 mse.rs 的
      // seal_frame 调用），而上游的事件块长成 `data: {...}\n\n` —— 用子串匹配的话，
      // 一条正好提到这个名字的回答就能让流提前「正常」结束，而且不会报错。
      if (block === EOS_FRAME) {
        sawEos = true;
        break outer;
      }
      yield block;
    }
  }
  if (!sawEos) {
    throw new MseError("e.trunc", "truncated");
  }
}
