use tauri::{AppHandle, Manager};
fn test(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        window.on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // api.prevent_close();
            }
        });
    }
}
