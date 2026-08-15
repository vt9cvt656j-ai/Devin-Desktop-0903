// MSE-1：TypeScript 客户端和 Rust 服务端必须算出**同一批字节**。
//
// 两边各自的单元测试都只证明「自己和自己对得上」——  HKDF 的 info 少一个竖线、tx 里两个
// 公钥的顺序反了、AAD 用逗号而不是 \0 分隔，任何一侧单独看都自洽，合起来就是所有请求
// 解不开，而且报出来的错只有一句「解密失败」，看不出是哪一步错的。
//
// 所以 server/testdata/mse-vectors.json 把一次完整的会话钉死了：固定服务端私钥、固定
// 客户端临时私钥、固定 nonce。那份文件是用 node 的 WebCrypto 真跑出来的，
// server/src/mse.rs 的 frozen_* 两个测试证明 Rust 能复现它；这个文件证明 TS 这一侧也能。
//
// 这里不 import web-shared/mse.ts —— 它是 TypeScript，node --test 直接跑不了。推导过程
// 在下面按它的写法重来一遍；而 AAD 的拼法（最容易两边写岔的一处）是把源文件里的
// joinNul / aadReq / aadRes 抠出来真跑，不是抄一份，抄的那份会跟着源文件一起错。
//
// 向量在父仓库里。ide/ 可以被单独 clone 出来（见 repo_sync.rs 顶部），那时候整个文件
// 跳过 —— 缺文件不是漂移。
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const PARENT = join(HERE, "..", "..");
const VECTORS_PATH = join(PARENT, "server", "testdata", "mse-vectors.json");
const CLIENT_PATH = join(PARENT, "server", "web-shared", "mse.ts");

const missing = [VECTORS_PATH, CLIENT_PATH].filter((p) => !existsSync(p));
const SKIP =
  missing.length > 0
    ? `父仓库不在（${missing.map((p) => p.replace(PARENT, "..")).join(", ")}）—— 缺文件不是漂移`
    : false;

const V = SKIP ? null : JSON.parse(readFileSync(VECTORS_PATH, "utf8"));
const SRC = SKIP ? "" : readFileSync(CLIENT_PATH, "utf8");

const enc = new TextEncoder();
const dec = new TextDecoder();
const subtle = globalThis.crypto.subtle;

const b64u = (b) => Buffer.from(b).toString("base64url");
const fromB64u = (s) => new Uint8Array(Buffer.from(s, "base64url"));
const fromB64 = (s) => new Uint8Array(Buffer.from(s, "base64"));

function concat(...parts) {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}

/**
 * 把 web-shared/mse.ts 里的一段函数抠出来真跑。
 *
 * 只有签名带类型标注，函数体是纯 JS，所以去掉 `: string` / `: number` / `: Uint8Array`
 * 这几处返回值和参数标注就能直接执行。抄一份到测试里也能跑，但抄的那份不会跟着源文件
 * 变 —— 而「两边拼法不一致」正是这个测试要抓的东西。
 *
 * 泛型参数要一起吃掉：TS 5.7 起 Uint8Array 是泛型的，源文件里写的是
 * `Uint8Array<ArrayBuffer>`，只剥裸名字会在 `<` 上炸成 SyntaxError。
 */
function liftFromClient(...names) {
  const decls = names.map((name) => {
    const fnAt = SRC.indexOf(`function ${name}(`);
    const constAt = SRC.indexOf(`const ${name} = (`);
    const at = fnAt >= 0 ? fnAt : constAt;
    assert.ok(at >= 0, `web-shared/mse.ts 里找不到 ${name}`);
    const end = fnAt >= 0 ? SRC.indexOf("\n}\n", at) + 3 : SRC.indexOf(";\n", at) + 2;
    assert.ok(end > at, `${name} 没有能识别的收尾`);
    return SRC.slice(at, end);
  });
  const js = decls
    .join("\n")
    .replace(/:\s*(?:string|number|Uint8Array(?:<[^>]*>)?)(?:\[\])?/g, "");
  const make = new Function("enc", `${js}\nreturn { ${names.join(", ")} };`);
  return make(enc);
}

test("冻结向量：ECDH、transcript、kid/sid、两把流量密钥", { skip: SKIP }, async () => {
  assert.equal(V.suite, "MSE1-P384-HKDF-SHA384-AES256GCM");
  assert.equal(V.format_version, 1);

  const serverSpki = fromB64u(V.keys.server_spki_b64u);
  const clientSpki = fromB64u(V.keys.client_epk_spki_b64u);

  const importPriv = (b) =>
    subtle.importKey("pkcs8", b, { name: "ECDH", namedCurve: "P-384" }, false, ["deriveBits"]);
  const importPub = (b) =>
    subtle.importKey("spki", b, { name: "ECDH", namedCurve: "P-384" }, false, []);

  const clientPriv = await importPriv(fromB64(V.keys.client_epk_pkcs8_b64));
  const serverPriv = await importPriv(fromB64(V.keys.server_pkcs8_b64));
  const serverPub = await importPub(serverSpki);
  const clientPub = await importPub(clientSpki);

  // P-384 的共享秘密是 48 字节的 X 坐标，和 Rust 的 raw_secret_bytes() 是同一个东西。
  const z = new Uint8Array(await subtle.deriveBits({ name: "ECDH", public: serverPub }, clientPriv, 384));
  assert.equal(z.length, 48);
  assert.equal(b64u(z), V.derived.z_b64u, "ECDH 输出和冻结的 z 不一致");
  // 两个方向都要落到同一个 z，否则服务端和客户端各推各的密钥。
  const zBack = new Uint8Array(await subtle.deriveBits({ name: "ECDH", public: clientPub }, serverPriv, 384));
  assert.equal(b64u(zBack), V.derived.z_b64u);

  // kid = base64url(SHA-384(服务端 SPKI))[..24]，sid = base64url(SHA-384(epk SPKI)[..18])。
  // 两个都是算出来的，不是发的 —— 服务端不需要为「这个 sid 是谁的」存任何状态。
  const kidDigest = new Uint8Array(await subtle.digest("SHA-384", serverSpki));
  assert.equal(b64u(kidDigest).slice(0, 24), V.derived.kid, "kid 不一致");
  const sidDigest = new Uint8Array(await subtle.digest("SHA-384", clientSpki));
  assert.equal(b64u(sidDigest.subarray(0, 18)), V.derived.sid, "sid 不一致");

  // transcript：SHA-384(客户端 SPKI || 服务端 SPKI)。顺序反了两边就再也谈不拢。
  const tx = new Uint8Array(await subtle.digest("SHA-384", concat(clientSpki, serverSpki)));
  assert.equal(b64u(tx), V.derived.tx_b64u, "tx 不一致");

  const infoFor = (dir) => concat(enc.encode(`MSE1/v1|${dir}|${V.derived.kid}|`), tx);
  assert.equal(b64u(infoFor("c2s")), V.derived.info_c2s_b64u, "HKDF info（c2s）不一致");
  assert.equal(b64u(infoFor("s2c")), V.derived.info_s2c_b64u, "HKDF info（s2c）不一致");

  // WebCrypto 的 HKDF 是 extract+expand 一步走，salt 给空数组；Rust 那边是
  // Hkdf::new(None, z)，salt 用 hashlen 个零字节。HMAC 会把不足一个分组的密钥补零，
  // 所以这两者本来就是同一件事 —— 这两把密钥对得上就是证明。
  const prk = await subtle.importKey("raw", z, "HKDF", false, ["deriveBits"]);
  const expand = async (dir) =>
    new Uint8Array(
      await subtle.deriveBits(
        { name: "HKDF", hash: "SHA-384", salt: new Uint8Array(0), info: infoFor(dir) },
        prk,
        256,
      ),
    );
  assert.equal(b64u(await expand("c2s")), V.derived.k_c2s_b64u, "k_c2s 不一致");
  assert.equal(b64u(await expand("s2c")), V.derived.k_s2c_b64u, "k_s2c 不一致");
  assert.notEqual(V.derived.k_c2s_b64u, V.derived.k_s2c_b64u, "方向密钥必须不同");
});

test("冻结向量：AAD 就是 web-shared/mse.ts 的 joinNul 拼出来的那几个字节", { skip: SKIP }, () => {
  const { joinNul, aadReq, aadRes } = liftFromClient("joinNul", "aadReq", "aadRes");

  const r = V.request.aad;
  assert.equal(b64u(aadReq(r.sid, r.seq, r.ts, r.method, r.path)), V.request.aad_b64u, "请求 AAD 拼法不一致");
  const s = V.response.aad;
  assert.equal(b64u(aadRes(s.sid, s.seq, s.path)), V.response.aad_b64u, "响应 AAD 拼法不一致");

  // 响应 AAD 里没有状态码，是有意的：MSE_MASK_STATUS=1 时外层被改写成 200，客户端
  // 拼不出带真实状态码的 AAD，一开这个开关全线解不开。它在密文内层的 s 字段里。
  assert.ok(!V.response.aad_b64u.includes(b64u(enc.encode("200"))));

  // 分隔符必须是 \0 而不是别的：否则 sid="a", seq=12 和 sid="a1", seq=2 会算出同一段
  // AAD，一条请求可以被平移成另一条。
  assert.deepEqual(Array.from(joinNul("a", "b")), [97, 0, 98]);
  assert.notDeepEqual(Array.from(joinNul("a", "12")), Array.from(joinNul("a1", "2")));
});

test("冻结信封：解得开，而且用同一个 nonce 重封能一字节不差地封回去", { skip: SKIP }, async () => {
  const aesKey = (b64) =>
    subtle.importKey("raw", fromB64u(b64), "AES-GCM", false, ["encrypt", "decrypt"]);
  const c2s = await aesKey(V.derived.k_c2s_b64u);
  const s2c = await aesKey(V.derived.k_s2c_b64u);

  const open = async (key, aad, env) =>
    new Uint8Array(
      await subtle.decrypt(
        { name: "AES-GCM", iv: env.subarray(1, 13), additionalData: aad, tagLength: 128 },
        key,
        env.subarray(13),
      ),
    );

  for (const [side, key] of [
    ["request", c2s],
    ["response", s2c],
  ]) {
    const v = V[side];
    const aad = fromB64u(v.aad_b64u);
    const env = fromB64u(v.envelope_b64u);
    assert.equal(env[0], 1, `${side}：信封第一个字节是格式版本`);
    assert.equal(b64u(env.subarray(1, 13)), v.nonce_b64u, `${side}：nonce 位置不对`);

    const pt = await open(key, aad, env);
    assert.equal(b64u(pt), v.plaintext_b64u, `${side}：明文字节不一致`);
    // 明文里有中文。两边的 UTF-8 编码也必须一致，否则只有带非 ASCII 的请求会坏。
    assert.equal(dec.decode(pt), v.plaintext_utf8, `${side}：UTF-8 解码不一致`);
    assert.equal(b64u(enc.encode(v.plaintext_utf8)), v.plaintext_b64u);

    // 用冻结的 nonce 重新封一遍，必须得到一模一样的信封。Rust 那边做不到这一条
    // （seal() 每次自己取随机 nonce），所以确定性这一半只能在这里钉。
    const ct = new Uint8Array(
      await subtle.encrypt(
        { name: "AES-GCM", iv: fromB64u(v.nonce_b64u), additionalData: aad, tagLength: 128 },
        key,
        pt,
      ),
    );
    assert.equal(b64u(concat(Uint8Array.of(1), fromB64u(v.nonce_b64u), ct)), v.envelope_b64u,
      `${side}：重封出来的信封和冻结的不一致`);
  }

  // 方向分离：响应密钥开不了请求信封，否则一条响应可以被原样重放成一个请求。
  await assert.rejects(() => open(s2c, fromB64u(V.request.aad_b64u), fromB64u(V.request.envelope_b64u)));
  await assert.rejects(() => open(c2s, fromB64u(V.response.aad_b64u), fromB64u(V.response.envelope_b64u)));
  // AAD 动一个字节就打不开 —— 换条路由重放同一个信封是打不开的。
  const tampered = fromB64u(V.request.aad_b64u);
  tampered[tampered.length - 1] ^= 1;
  await assert.rejects(() => open(c2s, tampered, fromB64u(V.request.envelope_b64u)));
});
