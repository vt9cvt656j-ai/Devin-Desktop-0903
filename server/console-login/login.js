// 管理后台登录。三步，缺一不可：
//
//   1. POST /api/auth/login          —— 拿 JWT，同时拿到 user.role
//   2. 不是 admin 就到此为止          —— 普通客户不该走到第二步
//   3. POST /api/admin/session       —— 用这张 Bearer 换一张 HttpOnly 门禁 cookie
//
// 第 3 步是这一页存在的理由。浏览器敲 URL 时不会带 Authorization 头，所以 nginx 在
// "要不要把管理台的 HTML 发出去" 那一刻没有任何东西可判断。cookie 是唯一能被浏览器
// 自动带上、又能被服务端验证的凭据。它由服务端下发（所以 HttpOnly 真的生效），并且
// **不是** JWT —— 偷到它只能换到静态文件，换不到接口权限。
//
// JWT 仍然写进 localStorage：SPA 的 XHR 用它做 Authorization 头。这一页和 /console/
// 同源，所以写进去的东西那边读得到。
(function () {
  "use strict";

  // 加密层预热。index.html 先加载了 mse.js，把客户端挂在 window.MSE 上。
  // 不等它 —— 提前取公钥只是为了让第一次提交少一个往返，取不到就在 send() 里回退明文。
  if (window.MSE) {
    try {
      window.MSE.configureMse({
        mode: "auto",
        // 网关静态公钥的指纹。这一页提交的是**管理员**密码，全站最值钱的一份凭据。
        // 轮换密钥时必须同时更新这里、ide/gate/gate.html，以及三个前端的 VITE_MSE_PIN。
        pin: ["yddBgF9eaS-gVWQtCJKoTwoM"],
      });
      window.MSE.mseReady().catch(function () {});
    } catch (_) {}
  }

  var TOKEN_KEY = "michael_admin_token";
  var f = document.getElementById("f");
  var go = document.getElementById("go");
  var msg = document.getElementById("msg");

  function say(text, bad) {
    msg.textContent = text || "";
    if (bad) msg.setAttribute("data-bad", "1");
    else msg.removeAttribute("data-bad");
  }

  // 只接受站内的相对路径，且不能以 // 开头（//evil.com 是协议相对的外链）。
  // 否则 /console/login?next=https://evil.com 就成了一个开放重定向，
  // 挂在一个真实的登录域名下发出去特别好使。
  function safeNext() {
    try {
      var raw = new URLSearchParams(location.search).get("next") || "";
      if (raw.charAt(0) !== "/" || raw.charAt(1) === "/") return "/console/";
      if (raw.indexOf("/console/login") === 0) return "/console/";
      return raw;
    } catch (_) {
      return "/console/";
    }
  }

  /**
   * 打开登录页 = 清空当前身份。
   *
   * 这一条修的是一个真实的坑：门禁 cookie 和 localStorage 里的令牌都是**上一个人**留下的。
   * 一个普通客户在这台机器上登录时，下面的流程会正确地拒绝给他签发任何东西 —— 但它并不会
   * 把已经存在的那份管理员凭据**拿掉**。结果是：客户点了登录、被拒绝，然后 /console/ 依然
   * 能打开，而且是以管理员身份打开的（nginx 看到的是旧的管理员 cookie，SPA 读到的是旧的
   * 管理员令牌）。看上去就像"非管理员登录进了后台"，实际上是上一段管理员会话根本没退。
   *
   * 所以：只要走到登录页，就先把服务端会话和本地令牌都清掉，任何人都从零开始。
   * 管理员误点到这一页也会被登出 —— 这是对的，登录页本来就该是"重新证明你是谁"的地方。
   */
  async function wipeIdentity() {
    // 本地令牌先清：它是同步的，不会因为网络出问题而留下。
    try {
      localStorage.removeItem(TOKEN_KEY);
    } catch (_) {}
    // 服务端会话再清。带 3 秒超时 —— 这一步绝不能把人卡在登录页外面：
    // 清理失败最坏是旧会话多活一会儿，而登录卡死是直接进不去。
    try {
      var ctl = typeof AbortController === "function" ? new AbortController() : null;
      var timer = setTimeout(function () { if (ctl) ctl.abort(); }, 3000);
      await send("/api/admin/session/logout", {
        method: "POST",
        credentials: "same-origin",
        signal: ctl ? ctl.signal : undefined,
      });
      clearTimeout(timer);
    } catch (_) {}
  }
  // 保存这个 promise：清理是异步的，而下面的提交流程会写入新的 cookie 和令牌。
  // 不等它结束就登录，清理可能在成功之后才落地，把刚签发的凭据又抹掉。
  var wiped = wipeIdentity();

  // 和 Login.tsx 一致：两个框都填了才可点。空着时按钮是灰的，这是原来那一屏的行为。
  var accountEl = document.getElementById("account");
  var passwordEl = document.getElementById("password");
  function syncEnabled() {
    go.disabled = !accountEl.value.trim() || !passwordEl.value;
  }
  accountEl.addEventListener("input", syncEnabled);
  passwordEl.addEventListener("input", syncEnabled);
  syncEnabled();

  /*
   * 加密优先，失败回退明文。
   *
   * mse.js（index.html 里先加载的那个）把 MSE 客户端挂成 window.MSE。mseFetch 返回的
   * 是正常的 Response，所以下面读 res.ok / res.status / res.json() 的代码一行都不用改。
   *
   * 回退是必须的：这是管理员唯一的入口，加密层不通时宁可这一次走明文，也不能把人
   * 锁在外面。/api/admin/session 下发的门禁 cookie 走的是**外层** Set-Cookie，
   * 加密不会把它吃掉（见 server/src/mse.rs 的 forward_set_cookie）。
   */
  async function send(path, init) {
    if (window.MSE) {
      try {
        return await window.MSE.mseFetch(path, init);
      } catch (e) {
        /*
         * `pin`（对面公钥不认识）和 `downgrade`（我们发密文、对面回明文）是敌意信号，
         * 不是「加密不可用」。这一页输入框里是管理员密码，这种时候退回明文等于直接
         * 把它交出去 —— 宁可这次登录失败。其余情况（网关没有加密层、老浏览器）才回退。
         */
        if (e && (e.code === "pin" || e.code === "downgrade")) throw e;
      }
    }
    return fetch(path, init);
  }

  async function post(path, body, token) {
    var headers = { "Content-Type": "application/json" };
    if (token) headers.Authorization = "Bearer " + token;
    var res = await send(path, {
      method: "POST",
      headers: headers,
      body: JSON.stringify(body || {}),
      // cookie 要能被写进来
      credentials: "same-origin",
    });
    var data = null;
    try {
      data = await res.json();
    } catch (_) {}
    return { ok: res.ok, status: res.status, data: data || {} };
  }

  f.addEventListener("submit", async function (e) {
    e.preventDefault();
    var account = document.getElementById("account").value.trim();
    var password = document.getElementById("password").value;
    if (!account || !password) return;

    go.disabled = true;
    say("登录中…");

    try {
      // 先确保上一段身份已经清干净，再开始写新的 —— 但最多等 3 秒。
      // 清理是"尽力而为"，它出问题不该变成"谁也登不进来"。
      await Promise.race([
        wiped,
        new Promise(function (r) { setTimeout(r, 3000); }),
      ]);
      // 线上字段名仍然是 email，但服务端会先按邮箱、再按用户名去找账号。
      // 改成 account 会直接 422 —— 这个坑踩过一次。
      var login = await post("/api/auth/login", { email: account, password: password });
      if (!login.ok) {
        say(login.data.error || login.data.message || "账号或密码不对", true);
        syncEnabled();
        return;
      }
      var token = login.data.token;
      var role = login.data.user && login.data.user.role;
      if (!token) {
        say("登录响应异常", true);
        syncEnabled();
        return;
      }
      if (role !== "admin") {
        // 普通客户登录成功了，但这里不是他该来的地方。不给 cookie，也不留 token。
        // 再清一次：这个账号无权进入，机器上不该留下任何仍然有效的后台身份。
        await wipeIdentity();
        say("这个账号不是管理员", true);
        syncEnabled();
        return;
      }

      var sess = await post("/api/admin/session", {}, token);
      if (!sess.ok) {
        say(sess.data.error || "门禁签发失败，请重试", true);
        syncEnabled();
        return;
      }

      try {
        localStorage.setItem(TOKEN_KEY, token);
      } catch (_) {}
      say("正在进入…");
      location.replace(safeNext());
    } catch (err) {
      say("网络异常，请重试", true);
      syncEnabled();
    }
  });
})();
