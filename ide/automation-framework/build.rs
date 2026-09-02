fn main() {
    // 必须读 CARGO_CFG_TARGET_OS，不能用 cfg!(target_os = ...)：构建脚本是为**宿主**
    // 编译的，宿主的 cfg 和交叉编译的目标平台没有关系。写成 cfg 的话，在 macOS 上
    // `--target x86_64-pc-windows-msvc` 会照样发出这条 framework 链接指令，然后
    // 编译器报「library kind `framework` is only supported on Apple targets」。
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-lib=framework=ApplicationServices");
    }
}
