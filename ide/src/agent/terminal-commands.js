// 终端相关的纯数据与纯函数。**没有 DOM、没有模块级可变状态、不发任何请求**，从 main.js
// 搬到这里（尺寸闸撞线时先搬模块，
// 不抬线）。名字一个字没改：test/helpers/source.mjs 会把 main.js 和 src/agent/*.js 拼成
// 一份文本供源码断言用，改名会让按名字断言的用例以「这段代码不见了」的形式假红。

// ---- terminal command suggestions (history + common commands + paths) ----
export const TERM_COMMON_CMDS = [
  // git
  "git status", "git status -s", "git add .", "git add -A", "git add -p",
  "git commit -m \"\"", "git commit -am \"\"", "git commit --amend",
  "git push", "git push -u origin ", "git push --force-with-lease", "git push --tags",
  "git pull", "git pull --rebase", "git fetch", "git fetch --all --prune",
  "git log", "git log --oneline", "git log --oneline --graph --all", "git log -p",
  "git checkout ", "git checkout -b ", "git switch ", "git switch -c ", "git switch -",
  "git branch", "git branch -a", "git branch -d ", "git branch -D ", "git branch -m ",
  "git merge ", "git merge --abort", "git rebase ", "git rebase -i ", "git rebase --abort", "git rebase --continue",
  "git diff", "git diff --staged", "git diff HEAD", "git diff --stat",
  "git stash", "git stash pop", "git stash list", "git stash apply", "git stash drop", "git stash show -p",
  "git reset ", "git reset --hard ", "git reset --soft HEAD~1", "git restore ", "git restore --staged ",
  "git clone ", "git remote -v", "git remote add origin ", "git tag ", "git cherry-pick ",
  "git show ", "git blame ", "git clean -fd", "git revert ", "git config --global ", "git init",
  // npm
  "npm install", "npm install ", "npm install -D ", "npm install -g ", "npm uninstall ",
  "npm run ", "npm run dev", "npm run build", "npm run test", "npm run lint", "npm run start",
  "npm start", "npm test", "npm ci", "npm update", "npm outdated", "npm audit", "npm audit fix",
  "npm publish", "npm version patch", "npm list", "npm cache clean --force", "npx ",
  // pnpm / yarn / bun
  "pnpm install", "pnpm add ", "pnpm add -D ", "pnpm remove ", "pnpm dev", "pnpm build", "pnpm test", "pnpm run ", "pnpm up",
  "yarn", "yarn add ", "yarn add -D ", "yarn remove ", "yarn dev", "yarn build", "yarn test", "yarn install",
  "bun install", "bun add ", "bun run ", "bun dev",
  // cargo / rust
  "cargo build", "cargo build --release", "cargo run", "cargo run --release", "cargo test",
  "cargo check", "cargo clippy", "cargo clippy --all-targets -- -D warnings", "cargo fmt",
  "cargo add ", "cargo update", "cargo install ", "cargo new ", "cargo doc --open", "rustup update", "rustc ",
  // python
  "python3 ", "python3 -m venv venv", "python3 -m pip install ", "pip install ", "pip install -r requirements.txt",
  "pip freeze > requirements.txt", "pip list", "pip3 install ", "source venv/bin/activate", "pytest", "python -m http.server",
  // node / go / others
  "node ", "deno run ", "deno task ", "tsx ", "ts-node ",
  "go run .", "go build", "go test ./...", "go mod tidy", "go get ", "go install ",
  "java -jar ", "javac ", "mvn ", "gradle ", "ruby ", "rails ", "php ", "php artisan ", "composer install",
  "dotnet run", "dotnet build", "dotnet test",
  // docker / k8s
  "docker ps", "docker ps -a", "docker images", "docker build -t ", "docker run ", "docker exec -it ",
  "docker stop ", "docker rm ", "docker rmi ", "docker logs -f ", "docker pull ", "docker push ", "docker system prune",
  "docker compose up", "docker compose up -d", "docker compose down", "docker compose logs -f", "docker compose build",
  "kubectl get pods", "kubectl get svc", "kubectl get nodes", "kubectl apply -f ", "kubectl delete -f ",
  "kubectl logs ", "kubectl describe pod ", "kubectl exec -it ", "helm install ",
  // filesystem
  "cd ", "cd ..", "cd ~", "cd -", "ls", "ls -la", "ls -lah", "pwd", "clear",
  "mkdir ", "mkdir -p ", "rmdir ", "rm ", "rm -rf ", "rm -f ", "cp ", "cp -r ", "mv ",
  "touch ", "cat ", "less ", "head ", "tail ", "tail -f ", "ln -s ", "stat ", "file ", "tree",
  "chmod +x ", "chmod 755 ", "chown ", "open ", "open .", "code .", "du -sh ", "df -h",
  // text / search
  "grep -r ", "grep -rn ", "grep -i ", "rg ", "rg -i ", "find . -name ", "find . -type f -name ",
  "sed -i ", "awk ", "sort ", "uniq ", "wc -l ", "xargs ", "diff ", "pbcopy < ", "pbpaste",
  // net / process
  "curl ", "curl -O ", "curl -L ", "wget ", "ssh ", "scp ", "rsync -av ", "ping ",
  "ps aux", "ps aux | grep ", "kill ", "kill -9 ", "killall ", "lsof -i :", "top", "htop",
  "netstat -an", "ifconfig", "nslookup ", "dig ",
  // archive / pkg managers
  "tar -xzf ", "tar -czf ", "zip -r ", "unzip ", "gzip ", "gunzip ",
  "brew install ", "brew update", "brew upgrade", "brew list", "brew search ", "brew uninstall ",
  "apt install ", "apt update", "apt upgrade", "sudo apt install ",
  // misc
  "echo ", "export ", "source ", "which ", "whereis ", "man ", "history", "alias ", "env",
  "sudo ", "watch ", "sleep ", "date", "whoami", "uname -a", "say ", "code .",
];

// ---------------------------------------------------------------------------
// 终端输出 → 结构化信号（从 main.js 搬来的第二块）
// ---------------------------------------------------------------------------
//
// 下面四个函数都只吃字符串、只回值：没有 DOM、没有模块级可变状态、不发任何请求 ——
// 正是尺寸闸注释里写的「边界干净就该搬」。名字一个字没改：test/helpers/source.mjs 会把
// main.js 和 src/agent/*.js 拼成一份文本供源码断言用，改名会让按名字断言的用例以
// 「这段代码不见了」的形式假红。

/**
 * 从终端输出里认出「这台机器上真正打得开的那个开发服务器地址」。
 *
 * 老版只认一种形状：明写出来的 loopback URL。实测 14 种常见框架的默认启动输出，它只
 * 认得 5 种（vite / next / CRA / django / php -S），uvicorn、flask --host、puma、
 * python -m http.server、express、gin、Spring Boot、docker 端口映射、裸 host:port 全漏。
 * 漏掉的后果不是少一条日志：run_in_terminal 之后 `_devServer.url` 是空的，预览、抓包、
 * performance_profile 全部失去目标，模型只能自己猜端口。
 *
 * 三级判据，先到先得：
 *   ① 明写的 loopback URL —— 原样返回（含路径）。取最后一条：反复重启时后印的才是活的。
 *   ② 通配地址（0.0.0.0 / [::] / [::0]）—— 端口是真的，主机名只表示「绑到所有网卡」，
 *      改写成 127.0.0.1 才是本机真正打得开的地址。
 *   ③ 只印端口号的那一大类 —— express「listening on port 3000」、gin「on :8080」、
 *      Spring Boot「port(s): 8080」、docker「0.0.0.0:8080->80/tcp」、裸 localhost:4000。
 *
 * **三级都不认局域网 IP**，这是刻意的：vite / CRA / streamlit 会同时印 Local 和 Network
 * 两行，认了 Network 就会把预览指到网卡地址（无线网下经常不通）。已有用例正钉着
 * `Network: http://192.168.1.5:5173` → ""。
 *
 * ②③ 之前先剔掉失败行：EADDRINUSE / `Error: listen ... :3000` 自己就带着端口号，不剔
 * 就会把刚崩掉的服务报成已就绪。
 */
export function _localDevServerUrl(output) {
  const plain = String(output || "").replace(/\x1b\[[0-?]*[ -\/]*[@-~]/g, "");
  const trim = (s) => s.replace(/[),.;'\"]+$/, "");
  const loop = plain.match(/https?:\/\/(?:localhost|127\.0\.0\.1|\[::1\])(?::\d{2,5})(?:\/[^\s]*)?/gi) || [];
  if (loop.length) return trim(loop.at(-1));
  const usable = plain.split("\n").filter((l) => !/EADDRINUSE|already in use|\berror\b/i.test(l)).join("\n");
  const wild = [...usable.matchAll(/https?:\/\/(?:0\.0\.0\.0|\[::\]|\[::0\])(:\d{2,5})(\/[^\s]*)?/gi)];
  if (wild.length) return trim(`http://127.0.0.1${wild.at(-1)[1]}${wild.at(-1)[2] || ""}`);
  // 第三级（只印了端口号）必须逐行收窄，否则误报会直接打到用户脸上：实测
  // `postgres://user:pw@localhost:5432/appdb` → 5432、`Redis ready ... on port 6379` → 6379、
  // `npm WARN config port 3000 is deprecated` → 3000 —— 预览面板会指到数据库端口。
  // 两条判据：那一行要有服务动词，且不能带非 http 的 scheme（DSN 自带 localhost:5432）。
  const SERVES = /\b(?:listening|serving|server|running|started|listen|available on|ready on)\b/i;
  const OTHER_SCHEME = /\b(?:postgres(?:ql)?|mysql|redis|mongodb|amqp|rediss|grpc):\/\//i;
  const PORT_IN_LINE = /\b(?:localhost|127\.0\.0\.1|0\.0\.0\.0):(\d{2,5})\b|\bport\(s\):?\s*(\d{2,5})\b|\bport\s+(\d{2,5})\b|\bon\s+:(\d{2,5})\b/i;
  for (const line of usable.split("\n").reverse()) {
    if (!SERVES.test(line) || OTHER_SCHEME.test(line)) continue;
    const m = PORT_IN_LINE.exec(line);
    if (!m) continue;
    const p = Number(m[1] || m[2] || m[3] || m[4]);
    if (p >= 1 && p <= 65535) return `http://127.0.0.1:${p}`;
  }
  return "";
}

// 终端 ready/失败信号检测：给 run_in_terminal 的自动就绪轮询用。只做纯文本判定、
// 不发任何请求。失败模式优先于 ready 模式：启动报错的服务往往也会印出端口/URL，
// 先判错误才不会把崩掉的服务误报成已就绪。返回 { ready, failed?, pattern }。
export function _detectTerminalReady(logText) {
  const plain = String(logText || "").replace(/\x1b\[[0-?]*[ -\/]*[@-~]/g, "");
  if (!plain.trim()) return { ready: false, pattern: "" };
  const failPatterns = [/EADDRINUSE/i, /error:/i, /cannot find module/i, /fatal/i];
  for (const re of failPatterns) {
    if (re.test(plain)) return { ready: false, failed: true, pattern: String(re) };
  }
  const readyPatterns = [
    /listening on/i,
    /server (?:ready|running|started)/i,
    /ready in \d/i,
    /compiled successfully/i,
    /local:\s*https?:\/\//i,
    /started server/i,
    // 裸 /✓/ 已删：单独的 ✓ 在测试通过/安装步骤等非 ready 场景大量出现，误报率高；
    // 真正的 ready 行（如 "✓ ready in 300ms"）已被上下文中其他模式覆盖
    /webpack.*compiled/i,
    /vite.*ready/i,
    /serving (?:at|on)/i,
    /port\s*\d{2,5}/i,
  ];
  for (const re of readyPatterns) {
    if (re.test(plain)) return { ready: true, pattern: String(re) };
  }
  return { ready: false, pattern: "" };
}

// 简单启发式判断"服务型命令"：dev server / serve / start 这类长驻启动才值得自动
// 轮询 ready 信号；npm test / build 这类一次性命令跑完就退出，不轮询。
export function _looksLikeServiceCommand(command) {
  const cmd = String(command || "");
  if (/\b(?:npm|pnpm|yarn|bun)\s+(?:run\s+)?(?:test|build)\b/i.test(cmd)) return false;
  return /\b(?:dev|serve|start|preview|run)\b/.test(cmd);
}

// 剥掉 `timeout`/`gtimeout` 包装，取出被它包住的内层命令。GNU timeout 形如
// `timeout [选项] <时长> <内层命令>`，选项可能带独立取值（-k 5 / -s TERM）或
// 等号形式（--kill-after=5 / --signal=TERM），也有无值开关（--preserve-status）。
// 用按空白分词的方式解析（天然对 \r\n 免疫，无需在正则里处理换行）：命中 timeout
// 前缀、且时长位是合法数字（可带 s/m/h/d 后缀）时，返回内层命令字符串；否则返回
// ""（不是 timeout 包装、或不成形，一律不剥离，避免误伤）。
export function _stripTimeoutWrapper(command) {
  const toks = String(command || "").trim().split(/\s+/);
  if (!toks.length || !/^g?timeout$/i.test(toks[0])) return "";
  let i = 1;
  // 跳过 timeout 自身的选项；-k / -s / --kill-after / --signal 的独立取值形式
  // 会多占一个 token（下一个 token 是它的取值），要一并跳过。
  while (i < toks.length && /^-/.test(toks[i])) {
    const opt = toks[i];
    i++;
    if (/^(?:-k|-s|--kill-after|--signal)$/i.test(opt) && i < toks.length) i++;
  }
  // 时长位必须是合法数字，否则视为不成形的 timeout，不剥离
  if (i >= toks.length || !/^\d+(?:\.\d+)?[smhd]?$/i.test(toks[i])) return "";
  i++;
  return toks.slice(i).join(" ").trim();
}
