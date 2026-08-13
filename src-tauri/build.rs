use std::path::PathBuf;

/// 打包进 app 的 automation sidecar 里必须带着鉴权。
///
/// 这个 sidecar 能合成**真实的鼠标键盘事件**（mouse.click / keyboard.type / keyboard.combo），
/// 也就是说能打开终端敲任意命令。它监听 127.0.0.1 的固定端口、随签名安装包分发，所以
/// `automation-framework/src/rpc.rs` 给它上了两道闸：共享密钥走自定义请求头（自定义头强制
/// 浏览器发 CORS 预检，而它不应答 OPTIONS，网页因此物理上发不出这个头），以及任何浏览器
/// 指纹头（Origin / Referer / Sec-Fetch-*）一律 401。
///
/// **2026-08-13 实测到的事故形状：** 那两道闸 2026-08-02 就加上了，但
/// `src-tauri/binaries/` 里放的是**手工构建、未被 git 跟踪**的二进制。aarch64 那个是
/// 加固之后编的（有鉴权），而 x86_64 和 universal 两个虽然文件日期更晚，却是加固**之前**
/// 的产物——零鉴权。Tauri 按目标三元组挑文件，于是 Apple Silicon 上装到的是安全的那份、
/// **Intel Mac 和 Windows 上装到的是零鉴权那份**，而且没有任何报错。我自己也是先只查了
/// arm64 那个就下了"不成立"的结论。
///
/// 光把二进制换掉不够——手工产物迟早会再次和源码脱节。这里在**构建期**把它钉死：
/// 当前目标要用的那个 sidecar 如果找不到鉴权痕迹，直接让构建失败，而不是安静地打包出去。
///
/// 判据用的是 `MICHAEL_AUTOMATION_TOKEN`（读环境变量的参数）和 `unauthorized`（401 响应体）
/// 这两个**确实会进 .rodata** 的字符串。**不要**改成搜 `x-automation-token:` 之类的短比较
/// 字面量——那种会被 release 编译内联成立即数，从已加固的源码新编出来也搜不到，用它做判据
/// 只会得到假警报（这个坑今天踩过三次，见 memory: dont-grep-binaries-to-verify）。
fn assert_sidecar_is_authenticated() {
    let target = match std::env::var("TARGET") {
        Ok(value) => value,
        Err(_) => return, // 拿不到目标三元组就不猜，别把构建卡死在一个判断不了的条件上
    };
    let suffix = if target.contains("windows") { ".exe" } else { "" };
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(format!("automation-server-{target}{suffix}"));
    println!("cargo:rerun-if-changed={}", path.display());
    let Ok(bytes) = std::fs::read(&path) else {
        // 文件不存在是 Tauri 自己的 externalBin 检查该报的错，这里不越俎代庖。
        return;
    };
    let has = |needle: &str| bytes.windows(needle.len()).any(|w| w == needle.as_bytes());
    if !has("MICHAEL_AUTOMATION_TOKEN") || !has("unauthorized") {
        panic!(
            "\n\n  {} 里没有鉴权痕迹。\n\
             \n  这个 sidecar 能合成真实键鼠事件；没有鉴权 = 本机任意进程（含浏览器里的网页）\
             \n  都能驱动它执行任意命令。\n\
             \n  它是手工构建的产物，多半和 automation-framework/src/rpc.rs 脱节了。重新构建：\
             \n      cd automation-framework && cargo build --release --target {target} --bin automation-server\
             \n      cp target/{target}/release/automation-server{} ../src-tauri/binaries/automation-server-{target}{}\n\n",
            path.display(),
            suffix,
            suffix,
        );
    }
}

fn main() {
    assert_sidecar_is_authenticated();
    tauri_build::build()
}
