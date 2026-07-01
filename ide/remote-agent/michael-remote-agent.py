#!/usr/bin/env python3
"""
Michael IDE — Remote Agent Daemon
=================================
Drop this ONE file on any server / PC and run it. The IDE (and its AI agent) then
read / write / run code on that machine directly — no SSH, no scp, no manual upload.

    python3 michael-remote-agent.py --token YOURSECRET --root /path/to/project

Zero dependencies (Python 3 stdlib only). Exposes a small authenticated HTTP API the
IDE talks to. EVERY request needs `Authorization: Bearer <token>`.

⚠️ SECURITY: this is remote file access + command execution by design (that's the point).
Use a LONG random token, only run it on machines you control, and prefer binding to a
private interface or putting it behind your gateway/TLS. Treat the token like an SSH key.

Endpoints (all POST JSON unless noted):
  GET  /ping                      → {ok, host, platform, cwd, root}
  POST /fs/list   {path}          → {entries:[{name,is_dir,size,mtime}]}
  POST /fs/read   {path,offset?,limit?} → {content, total_lines, truncated}
  POST /fs/write  {path, content} → {ok, bytes}
  POST /fs/mkdir  {path}          → {ok}
  POST /fs/delete {path}          → {ok}
  POST /fs/rename {from, to}      → {ok}
  POST /fs/stat   {path}          → {exists, is_dir, size, mtime}
  POST /fs/search {root, query, case_sensitive?, max?} → {hits:[{rel,line,text}]}
  POST /exec      {command, cwd?, timeout?}            → {stdout, stderr, code, timed_out}
"""
import argparse, json, os, re, shutil, socket, subprocess, sys, time, platform, hmac
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

CFG = {"token": "", "root": None, "max_read_bytes": 4_000_000}
IGNORE_DIRS = {".git", "node_modules", "target", "dist", "build", ".next", ".venv",
               "__pycache__", ".cache", "vendor", ".idea", ".gradle"}


def _within_root(p):
    """Resolve p; if a --root sandbox is set, RELATIVE paths resolve against the root
    (not cwd) and any path outside the root is refused."""
    p = os.path.expanduser(p)
    if CFG["root"] and not os.path.isabs(p):
        p = os.path.join(CFG["root"], p)  # relative → relative to the sandbox root
    rp = os.path.realpath(p)
    if CFG["root"]:
        root = os.path.realpath(CFG["root"])
        if rp != root and not rp.startswith(root + os.sep):
            raise PermissionError(f"路径越界（不在 --root {root} 内）: {p}")
    return rp


def h_ping(_):
    return {"ok": True, "host": socket.gethostname(), "platform": platform.platform(),
            "python": sys.version.split()[0], "cwd": os.getcwd(), "root": CFG["root"]}


def h_fs_list(b):
    p = _within_root(b["path"])
    out = []
    with os.scandir(p) as it:
        for e in it:
            try:
                st = e.stat(follow_symlinks=False)
                out.append({"name": e.name, "is_dir": e.is_dir(follow_symlinks=False),
                            "size": st.st_size, "mtime": int(st.st_mtime)})
            except OSError:
                continue
    out.sort(key=lambda x: (not x["is_dir"], x["name"].lower()))
    return {"entries": out}


def h_fs_read(b):
    p = _within_root(b["path"])
    if os.path.getsize(p) > CFG["max_read_bytes"]:
        return {"error": f"文件过大（>{CFG['max_read_bytes']//1_000_000}MB），用 offset/limit 分段读"}
    text = open(p, "r", encoding="utf-8", errors="replace").read()
    lines = text.split("\n")
    off = max(1, int(b.get("offset") or 1))
    lim = int(b.get("limit") or 0)
    seg = lines[off - 1:(off - 1 + lim) if lim else None]
    return {"content": "\n".join(seg), "total_lines": len(lines),
            "from": off, "to": off + len(seg) - 1, "truncated": bool(lim) and off - 1 + lim < len(lines)}


def h_fs_write(b):
    p = _within_root(b["path"])
    os.makedirs(os.path.dirname(p) or ".", exist_ok=True)
    data = b.get("content", "")
    open(p, "w", encoding="utf-8").write(data)
    return {"ok": True, "bytes": len(data.encode("utf-8"))}


def h_fs_mkdir(b):
    os.makedirs(_within_root(b["path"]), exist_ok=True)
    return {"ok": True}


def h_fs_delete(b):
    p = _within_root(b["path"])
    if os.path.isdir(p) and not os.path.islink(p):
        shutil.rmtree(p)
    elif os.path.exists(p) or os.path.islink(p):
        os.remove(p)
    return {"ok": True}


def h_fs_rename(b):
    src, dst = _within_root(b["from"]), _within_root(b["to"])
    os.makedirs(os.path.dirname(dst) or ".", exist_ok=True)
    os.rename(src, dst)
    return {"ok": True}


def h_fs_stat(b):
    try:
        p = _within_root(b["path"])
        st = os.stat(p)
        return {"exists": True, "is_dir": os.path.isdir(p), "size": st.st_size, "mtime": int(st.st_mtime)}
    except (OSError, PermissionError):
        return {"exists": False}


def h_fs_search(b):
    root = _within_root(b.get("root") or ".")
    q = b.get("query") or ""
    flags = 0 if b.get("case_sensitive") else re.IGNORECASE
    try:
        rx = re.compile(q, flags)
    except re.error:
        rx = re.compile(re.escape(q), flags)  # invalid regex → literal
    cap = int(b.get("max") or 200)
    hits, scanned = [], 0
    for dirpath, dirnames, files in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in IGNORE_DIRS and not d.startswith(".")]
        for fn in files:
            if len(hits) >= cap or scanned > 20000:
                return {"hits": hits, "truncated": len(hits) >= cap}
            fp = os.path.join(dirpath, fn)
            scanned += 1
            try:
                for i, ln in enumerate(open(fp, "r", encoding="utf-8", errors="replace"), 1):
                    if rx.search(ln):
                        hits.append({"rel": os.path.relpath(fp, root), "line": i, "text": ln.rstrip("\n")[:300]})
                        if len(hits) >= cap:
                            break
            except OSError:
                continue
    return {"hits": hits, "truncated": False}


def h_exec(b):
    cmd = b.get("command", "")
    if not cmd.strip():
        return {"error": "空命令"}
    cwd = _within_root(b["cwd"]) if b.get("cwd") else (CFG["root"] or os.getcwd())
    timeout = min(int(b.get("timeout") or 120), 600)
    try:
        r = subprocess.run(cmd, shell=True, cwd=cwd, capture_output=True, text=True, timeout=timeout)
        return {"stdout": r.stdout[-100_000:], "stderr": r.stderr[-50_000:], "code": r.returncode, "timed_out": False, "cwd": cwd}
    except subprocess.TimeoutExpired as e:
        return {"stdout": (e.stdout or "")[-50_000:] if isinstance(e.stdout, str) else "",
                "stderr": "[超时] 命令超过 %ss——起服务请用后台方式 nohup …&" % timeout, "code": -1, "timed_out": True}


ROUTES = {
    "/ping": h_ping, "/fs/list": h_fs_list, "/fs/read": h_fs_read, "/fs/write": h_fs_write,
    "/fs/mkdir": h_fs_mkdir, "/fs/delete": h_fs_delete, "/fs/rename": h_fs_rename,
    "/fs/stat": h_fs_stat, "/fs/search": h_fs_search, "/exec": h_exec,
}


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def _send(self, code, obj):
        body = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Headers", "authorization,content-type")
        self.end_headers()
        self.wfile.write(body)

    def _authed(self):
        got = (self.headers.get("Authorization") or "").removeprefix("Bearer ").strip()
        return CFG["token"] and hmac.compare_digest(got, CFG["token"])

    def do_OPTIONS(self):
        self._send(200, {"ok": True})

    def do_GET(self):
        if self.path.split("?")[0] == "/ping" and self._authed():
            return self._send(200, h_ping({}))
        self._send(404 if self.path != "/ping" else 401, {"error": "unauthorized" if not self._authed() else "not found"})

    def do_POST(self):
        if not self._authed():
            return self._send(401, {"error": "unauthorized — bad or missing token"})
        path = self.path.split("?")[0]
        fn = ROUTES.get(path)
        if not fn:
            return self._send(404, {"error": f"unknown endpoint {path}"})
        try:
            n = int(self.headers.get("Content-Length") or 0)
            body = json.loads(self.rfile.read(n) or b"{}") if n else {}
            return self._send(200, fn(body))
        except (PermissionError, KeyError) as e:
            return self._send(400, {"error": str(e)})
        except Exception as e:  # noqa: BLE001 — daemon must never crash on one bad call
            return self._send(500, {"error": f"{type(e).__name__}: {e}"})

    def log_message(self, *a):  # quieter
        sys.stderr.write("[%s] %s\n" % (time.strftime("%H:%M:%S"), (a[0] % a[1:]) if a else ""))


def main():
    ap = argparse.ArgumentParser(description="Michael IDE remote-agent daemon")
    ap.add_argument("--token", default=os.environ.get("MICHAEL_REMOTE_TOKEN", ""), help="auth token (or env MICHAEL_REMOTE_TOKEN)")
    ap.add_argument("--host", default="0.0.0.0", help="bind interface (default 0.0.0.0; use 127.0.0.1 for local-only)")
    ap.add_argument("--port", type=int, default=8765)
    ap.add_argument("--root", default=None, help="sandbox: restrict all file ops to this dir (recommended)")
    ap.add_argument("--cert", default=None, help="TLS 证书 (PEM)，配合 --key 启用 HTTPS（加密传输；公网/不可信网络强烈建议）")
    ap.add_argument("--key", default=None, help="TLS 私钥 (PEM)")
    args = ap.parse_args()
    if not args.token or len(args.token) < 12:
        sys.exit("拒绝启动：--token 必须设置且 ≥12 位（像 SSH 密钥一样保密）。例：--token $(openssl rand -hex 24)")
    CFG["token"] = args.token
    CFG["root"] = os.path.realpath(os.path.expanduser(args.root)) if args.root else None
    srv = ThreadingHTTPServer((args.host, args.port), Handler)
    scheme = "http"
    if args.cert and args.key:
        import ssl
        ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        ctx.load_cert_chain(args.cert, args.key)
        srv.socket = ctx.wrap_socket(srv.socket, server_side=True)
        scheme = "https"
    elif args.cert or args.key:
        sys.exit("启用 TLS 需要同时给 --cert 和 --key。生成自签证书：openssl req -x509 -newkey rsa:2048 -nodes -keyout key.pem -out cert.pem -days 365 -subj '/CN=michael-remote'")
    print(f"✓ Michael 远程代理已启动  {scheme}://{args.host}:{args.port}")
    print(f"  主机={socket.gethostname()}  root={CFG['root'] or '(整机)'}  平台={platform.platform()}  TLS={'开' if scheme=='https' else '关'}")
    print(f"  IDE 里填：地址 {scheme}://<本机IP>:{args.port}  +  这个 token。Ctrl-C 停止。")
    if scheme == "http":
        print("  ⚠ 未启用 TLS：token 明文传输。仅在可信网络/SSH 隧道/VPN(如 Tailscale) 内用；公网请加 --cert/--key 或放到带 TLS 的反代后面。")
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        print("\n已停止。")


if __name__ == "__main__":
    main()
