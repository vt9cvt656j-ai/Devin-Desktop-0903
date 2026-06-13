mod ai;
mod devin;
mod files;

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
        .invoke_handler(tauri::generate_handler![
            files::read_dir,
            files::read_text_file,
            files::write_text_file,
            files::home_dir,
            ai::ai_chat,
            devin::devin_create_session,
            devin::devin_send_message,
            devin::devin_get_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Devin IDE");
}
