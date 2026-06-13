mod ai;
mod extensions;
mod files;
mod terminal;

/// Entry point shared by the binary and (potentially) mobile targets.
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(terminal::TerminalState::default())
        .invoke_handler(tauri::generate_handler![
            files::read_dir,
            files::read_text_file,
            files::write_text_file,
            files::home_dir,
            files::create_file,
            files::create_dir,
            files::rename_path,
            files::delete_path,
            files::search_in_project,
            ai::ai_chat,
            extensions::ext_list_installed,
            extensions::ext_read_asset,
            extensions::ext_set_enabled,
            extensions::ext_uninstall,
            extensions::ext_available_builtin,
            extensions::ext_install_builtin,
            extensions::ext_install_from_path,
            terminal::term_open,
            terminal::term_write,
            terminal::term_resize,
            terminal::term_close,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Michael IDE");
}
