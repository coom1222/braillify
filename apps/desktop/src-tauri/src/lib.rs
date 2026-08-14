mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::translate_to_unicode,
            commands::decode_from_unicode
        ])
        .run(tauri::generate_context!())
        .expect("Braillify 앱을 실행하지 못했습니다.");
}
