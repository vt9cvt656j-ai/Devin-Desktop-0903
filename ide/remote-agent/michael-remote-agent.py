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
  POST /fs/copy   {from, to}      → {ok}
  POST /fs/delete {path}          → {ok}
  POST /fs/rename {from, to}      → {ok}
  POST /fs/stat   {path}          → {exists, is_dir, size, mtime}
  POST /fs/search {root, query, mode?:"literal"|"regex", case_sensitive?, max?}
                  → {hits:[{path,rel,line,column,start,end,text}], scanned_files, truncated}
  POST /exec      {command, cwd?, timeout?}            → {stdout, stderr, code, timed_out}
"""
import argparse, json, os, re, shutil, socket, subprocess, sys, time, platform, hmac, threading, tempfile, stat
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

CFG = {"token": "", "root": None, "max_read_bytes": 4_000_000}
FILE_MUTATION_LOCK = threading.RLock()
IGNORE_DIRS = {".git", "node_modules", "target", "dist", "build", "out", ".next", ".venv",
               "__pycache__", ".cache", "vendor", ".idea", ".gradle", "coverage"}
SEARCH_MAX_FILE_BYTES = 2 * 1024 * 1024
SEARCH_MAX_SCANNED_FILES = 20_000
SEARCH_MAX_RESULTS = 2_000
SEARCH_MAX_PER_FILE = 50
SEARCH_MAX_LINE_CHARS = 500


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
    try:
        off = int(b.get("offset") or 1)
        lim = int(b.get("limit") or 0)
    except (TypeError, ValueError):
        return {"error": "[INVALID_READ_RANGE] offset/limit 必须是整数"}
    if off < 1 or lim < 0:
        return {"error": "[INVALID_READ_RANGE] offset 必须 >= 1，limit 必须 >= 0"}
    if os.path.getsize(p) > CFG["max_read_bytes"] and not lim:
        return {"error": f"文件过大（>{CFG['max_read_bytes']//1_000_000}MB），用 offset/limit 分段读"}
    seg = []
    total = 0
    selected_bytes = 0
    ended_with_newline = False
    with open(p, "r", encoding="utf-8", newline=None) as source:
        for raw_line in source:
            total += 1
            ended_with_newline = raw_line.endswith("\n")
            line = raw_line[:-1] if ended_with_newline else raw_line
            if line.endswith("\r"):
                line = line[:-1]
            if total >= off and (not lim or total < off + lim):
                selected_bytes += len(line.encode("utf-8")) + (1 if seg else 0)
                if selected_bytes > CFG["max_read_bytes"]:
                    return {"error": f"读取结果过大（>{CFG['max_read_bytes']//1_000_000}MB），请缩小 limit"}
                seg.append(line)
    if total == 0:
        total = 1
        if off == 1 and (not lim or lim >= 1):
            seg.append("")
    elif ended_with_newline:
        total += 1
        if total >= off and (not lim or total < off + lim):
            seg.append("")
    shown_to = off + len(seg) - 1
    return {"content": "\n".join(seg), "total_lines": total,
            "from": off, "to": shown_to, "truncated": bool(lim) and shown_to < total}


def _atomic_write_text(path, data):
    """Stage complete UTF-8 content beside the target, then atomically replace it."""
    parent = os.path.dirname(path) or "."
    os.makedirs(parent, exist_ok=True)
    old_stat = os.stat(path) if os.path.isfile(path) else None
    old_mode = stat.S_IMODE(old_stat.st_mode) if old_stat is not None else None
    fd, staged = tempfile.mkstemp(prefix=f".{os.path.basename(path)}.michael-write-", suffix=".tmp", dir=parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="") as out:
            out.write(data)
            out.flush()
            if old_stat is not None and hasattr(os, "fchown"):
                try:
                    os.fchown(out.fileno(), old_stat.st_uid, old_stat.st_gid)
                except (AttributeError, NotImplementedError, OSError):
                    pass
            mode = old_mode if old_mode is not None else 0o644
            fchmod = getattr(os, "fchmod", None)
            if callable(fchmod):
                try:
                    fchmod(out.fileno(), mode)
                except (AttributeError, NotImplementedError, OSError):
                    fchmod = None
            if not callable(fchmod):
                try:
                    os.chmod(staged, mode)
                except (AttributeError, NotImplementedError, OSError):
                    pass
            os.fsync(out.fileno())
        os.replace(staged, path)
        staged = None
        try:
            dir_fd = os.open(parent, os.O_RDONLY)
            try:
                os.fsync(dir_fd)
            finally:
                os.close(dir_fd)
        except OSError:
            pass
    finally:
        if staged is not None:
            try:
                os.unlink(staged)
            except FileNotFoundError:
                pass


def h_fs_write(b):
    if "content" not in b:
        return {"error": "[INVALID_WRITE_CONTENT] 缺少 content；已拒绝写入，文件未变化"}
    data = b.get("content")
    if not isinstance(data, str):
        return {"error": "[INVALID_WRITE_CONTENT] content 必须是字符串；已拒绝写入，文件未变化"}
    with FILE_MUTATION_LOCK:
        p = _within_root(b["path"])
        if "expected_content" in b:
            expected = b.get("expected_content")
            exists = os.path.exists(p)
            if expected is None and exists:
                return {"error": "[CONFLICT] file was created by another task"}
            if expected is not None:
                if not exists:
                    return {"error": "[CONFLICT] file was deleted after it was read"}
                current = open(p, "r", encoding="utf-8").read()
                if current != expected:
                    return {"error": "[CONFLICT] file changed after it was read"}
        _atomic_write_text(p, data)
    return {"ok": True, "bytes": len(data.encode("utf-8"))}


def h_fs_mkdir(b):
    with FILE_MUTATION_LOCK:
        p = _within_root(b["path"])
        if os.path.exists(p) or os.path.islink(p):
            return {"error": "[CONFLICT] create directory target already exists"}
        os.makedirs(p, exist_ok=False)
    return {"ok": True}


def h_fs_copy(b):
    with FILE_MUTATION_LOCK:
        src, dst = _within_root(b["from"]), _within_root(b["to"])
        if os.path.exists(dst) or os.path.islink(dst):
            return {"error": "[CONFLICT] copy destination already exists"}
        os.makedirs(os.path.dirname(dst) or ".", exist_ok=True)
        if os.path.isdir(src) and not os.path.islink(src):
            shutil.copytree(src, dst)
        else:
            shutil.copy2(src, dst)
    return {"ok": True}


def h_fs_delete(b):
    with FILE_MUTATION_LOCK:
        p = _within_root(b["path"])
        if "expected_content" in b:
            expected = b.get("expected_content")
            if expected is None or not os.path.isfile(p) or os.path.islink(p):
                return {"error": "[CONFLICT] path is no longer the expected text file"}
            current = open(p, "r", encoding="utf-8").read()
            if current != expected:
                return {"error": "[CONFLICT] file changed after it was written"}
        if os.path.isdir(p) and not os.path.islink(p):
            shutil.rmtree(p)
        elif os.path.exists(p) or os.path.islink(p):
            os.remove(p)
    return {"ok": True}


def h_fs_rename(b):
    with FILE_MUTATION_LOCK:
        src, dst = _within_root(b["from"]), _within_root(b["to"])
        if os.path.exists(dst) or os.path.islink(dst):
            return {"error": "[CONFLICT] rename destination already exists"}
        if "expected_content" in b:
            expected = b.get("expected_content")
            if expected is None or not os.path.isfile(src) or os.path.islink(src):
                return {"error": "[CONFLICT] rename source is no longer the expected text file"}
            current = open(src, "r", encoding="utf-8").read()
            if current != expected:
                return {"error": "[CONFLICT] rename source changed"}
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
    q = b.get("query")
    if not isinstance(q, str) or not q:
        return {"error": "[INVALID_SEARCH_QUERY] search query cannot be empty",
                "hits": [], "scanned_files": 0, "truncated": False}

    mode = "literal" if b.get("mode") is None else b.get("mode")
    if not isinstance(mode, str) or mode.strip().lower() not in {"literal", "regex"}:
        return {"error": f"[INVALID_SEARCH_MODE] unsupported search mode '{mode}'; expected 'literal' or 'regex'",
                "hits": [], "scanned_files": 0, "truncated": False}
    mode = mode.strip().lower()

    if not os.path.isfile(root) and not os.path.isdir(root):
        return {"error": f"[INVALID_SEARCH_SCOPE] search scope '{root}' is neither a file nor a directory",
                "hits": [], "scanned_files": 0, "truncated": False}

    flags = 0 if b.get("case_sensitive") else re.IGNORECASE
    try:
        rx = re.compile(re.escape(q) if mode == "literal" else q, flags)
    except re.error as error:
        return {"error": f"[INVALID_SEARCH_PATTERN] invalid regex: {error}",
                "hits": [], "scanned_files": 0, "truncated": False}

    try:
        raw_cap = b.get("max")
        cap = SEARCH_MAX_RESULTS if raw_cap is None else int(raw_cap)
    except (TypeError, ValueError):
        return {"error": "[INVALID_SEARCH_LIMIT] max must be an integer between 1 and 2000",
                "hits": [], "scanned_files": 0, "truncated": False}
    if cap < 1 or cap > SEARCH_MAX_RESULTS:
        return {"error": "[INVALID_SEARCH_LIMIT] max must be an integer between 1 and 2000",
                "hits": [], "scanned_files": 0, "truncated": False}

    hits, scanned, truncated = [], 0, False
    scope_is_file = os.path.isfile(root)
    relative_base = os.path.dirname(root) if scope_is_file else root

    def search_paths():
        if scope_is_file:
            yield root
            return
        for dirpath, dirnames, files in os.walk(root, followlinks=False):
            dirnames[:] = sorted(
                d for d in dirnames if d not in IGNORE_DIRS and not d.startswith(".")
            )
            for filename in sorted(files):
                if not filename.startswith("."):
                    yield os.path.join(dirpath, filename)

    for fp in search_paths():
        if len(hits) >= cap or scanned >= SEARCH_MAX_SCANNED_FILES:
            return {"hits": hits, "scanned_files": scanned, "truncated": True}
        if os.path.islink(fp):
            continue
        try:
            if os.path.getsize(fp) > SEARCH_MAX_FILE_BYTES:
                continue
            with open(fp, "r", encoding="utf-8") as source:
                content = source.read()
        except (OSError, UnicodeError):
            continue
        if "\0" in content[:8000]:
            continue

        scanned += 1
        rel = os.path.relpath(fp, relative_base)
        file_hits = 0
        file_capped = False
        for line_number, text in enumerate(content.splitlines(), 1):
            display = text if len(text) <= SEARCH_MAX_LINE_CHARS else text[:SEARCH_MAX_LINE_CHARS] + "…"
            for matched in rx.finditer(text):
                if file_hits >= SEARCH_MAX_PER_FILE:
                    truncated = True
                    file_capped = True
                    break
                hits.append({
                    "path": fp,
                    "rel": rel,
                    "line": line_number,
                    "column": matched.start() + 1,
                    "start": matched.start(),
                    "end": matched.end(),
                    "text": display,
                })
                file_hits += 1
                if len(hits) >= cap:
                    return {"hits": hits, "scanned_files": scanned, "truncated": True}
            if file_capped:
                break

    if scanned == 0:
        return {"error": f"[NO_SEARCHABLE_FILES] search scope '{root}' contained no readable UTF-8 text files",
                "hits": [], "scanned_files": 0, "truncated": False}
    return {"hits": hits, "scanned_files": scanned, "truncated": truncated}


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
    "/fs/mkdir": h_fs_mkdir, "/fs/copy": h_fs_copy, "/fs/delete": h_fs_delete, "/fs/rename": h_fs_rename,
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
