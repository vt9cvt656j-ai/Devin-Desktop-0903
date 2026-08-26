// 终端面板的常用命令表。**纯数据、零依赖**，从 main.js 搬到这里（尺寸闸撞线时先搬模块，
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
