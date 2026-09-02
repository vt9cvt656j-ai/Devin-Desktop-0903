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
    // crate 源码也要盯着。Tauri **不会**自动重编 automation-framework（它是独立 crate、
    // 没有依赖边），所以改了那边的源码、只重编本 crate，跑起来的还是旧二进制 ——
    // 改了等于没改，而且不报错。这是本仓库记录在案的坑，实测踩过。
    let crate_src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../automation-framework/src");
    println!("cargo:rerun-if-changed={}", crate_src.display());

    let Ok(bytes) = std::fs::read(&path) else {
        // 文件不存在是 Tauri 自己的 externalBin 检查该报的错，这里不越俎代庖。
        return;
    };

    // 源码比二进制新 = 这份二进制是旧的。这里必须**拦下来**：让它安静地打进包，
    // 用户拿到的就是一个"你以为修好了"的版本。
    if let (Ok(bin_time), Some(src_time)) = (
        std::fs::metadata(&path).and_then(|m| m.modified()),
        newest_mtime(&crate_src),
    ) {
        if src_time > bin_time {
            panic!(
                "\n\n  {} 比它自己的源码旧。\n\
                 \n  automation-framework 是独立 crate，Tauri 不会替你重编它 —— 直接打包的话，\
                 \n  你对那份源码做的任何修改都不会出现在产品里，而且一声不响。\
                 \n\n  重新构建：\
                 \n      cd automation-framework && cargo build --release --target {target} --bin automation-server\
                 \n      cp target/{target}/release/automation-server{} ../src-tauri/binaries/automation-server-{target}{}\n\n",
                path.display(),
                suffix,
                suffix,
            );
        }
    }
    let has = |needle: &str| bytes.windows(needle.len()).any(|w| w == needle.as_bytes());
    // `mouse.move` 是 RPC 方法名，只有真的 automation-server 才有。仓库里出现过一个
    // 名字体积都像真的、里面却是 Rust std 空壳的 Windows 二进制——只验鉴权字符串挡不住它。
    if !has("MICHAEL_AUTOMATION_TOKEN") || !has("unauthorized") || !has("mouse.move") {
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

/// 目录下最新的修改时间（递归）。
fn newest_mtime(dir: &std::path::Path) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let t = if path.is_dir() {
            newest_mtime(&path)
        } else {
            entry.metadata().and_then(|m| m.modified()).ok()
        };
        if let Some(t) = t {
            if newest.is_none_or(|n| t > n) {
                newest = Some(t);
            }
        }
    }
    newest
}

fn main() {
    assert_sidecar_is_authenticated();
    // DPI 声明交给 tauri-build 一起嵌，**不要**自己再塞一份。
    // 详见 WINDOWS_APP_MANIFEST 上面那段。
    let attrs = tauri_build::Attributes::new().windows_attributes(
        tauri_build::WindowsAttributes::new().app_manifest(WINDOWS_APP_MANIFEST),
    );
    tauri_build::try_build(attrs).expect("tauri build failed");
}

/// Windows 的应用清单。**整个进程只能有一份**。
///
/// 这里原来是一个 `windows_dpi_aware_manifest()`：自己写一份只含 dpiAwareness 的清单，
/// 再用 `/MANIFESTINPUT` + `/MANIFEST:EMBED` 交给链接器。而 `tauri_build::build()`
/// 本来就会嵌一份自己的清单——两份 MANIFEST 都是 id 1，链接期直接失败：
///
///   CVTRES : fatal error CVT1100: duplicate resource. type:MANIFEST, name:1, language:0x0409
///   LINK   : fatal error LNK1123: failure during conversion to COFF
///
/// 也就是说那次「补 DPI 感知」的修复，代价是 **Windows 整个编不出来**。
/// 而它没被任何一道闸拦住：`cargo xwin check` 和发版前置脚本里那句「Windows 那条
/// 分支能不能编过」走的都是 `check`，**不做链接**，而冲突恰恰只在链接期出现。
///
/// 正确做法是把 DPI 声明并进 tauri-build 要嵌的那一份里，只留一份清单。
/// 下面这份是完整清单（它会**替换**掉 tauri-build 的默认清单，所以默认里那些
/// 该有的都得带上）：
///   · Common-Controls v6 依赖 —— 没有它，原生控件退回 Win95 外观；
///   · compatibility/supportedOS —— 缺了的话 Windows 按 Vista 兼容模式对待本进程，
///     GetVersion 之类会撒谎，而且 PerMonitorV2 也不会生效；
///   · trustInfo asInvoker —— 不提权，和 Tauri 默认一致；
///   · windowsSettings/dpiAwareness —— 这次真正要加的东西。
///
/// 为什么走清单而不是运行时调 SetProcessDpiAwarenessContext：清单在进程启动前就
/// 生效，不依赖代码执行时机，也不会被先跑到的某个库抢先设成别的等级。
///
/// 不声明 DPI 感知的后果（这也是当初要加它的原因）：进程会被 Windows **虚拟化**——
/// UI Automation 的 BoundingRectangle 给真实物理像素，而 GetSystemMetrics /
/// GetCursorPos 给被缩放过的逻辑像素，两者差一个缩放系数。而 read_screen 出坐标、
/// mouse.move 吃坐标，工具描述还明写着「这几个说的是同一套坐标，绝不要乘 scale_factor」。
/// 在 125%/150% 缩放（Windows 笔记本出厂默认）下，模型照着读到的坐标去点必然点在别处，
/// 而每一步回执都说成功。
const WINDOWS_APP_MANIFEST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*" />
    </dependentAssembly>
  </dependency>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{e2011457-1546-43c5-a5fe-008deee3d3f0}" />
      <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}" />
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}" />
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}" />
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}" />
    </application>
  </compatibility>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
    </windowsSettings>
  </application>
</assembly>
"#;
