use std::process::Command;

#[tauri::command]
pub async fn install_update_linux(url: String) -> Result<(), String> {
    println!("[updater] Downloading update from: {}", url);
    
    // 1. Download the file using curl to /tmp
    let download_status = Command::new("curl")
        .args(&["-L", "-o", "/tmp/pzync_update.deb", &url])
        .status()
        .map_err(|e| format!("Failed to execute download command: {}", e))?;

    if !download_status.success() {
        return Err("Failed to download update package".to_string());
    }

    println!("[updater] Launching pkexec to install the debian package...");

    // 2. Install using pkexec. This will show a graphical password prompt on Ubuntu/Debian.
    let install_status = Command::new("pkexec")
        .args(&["apt-get", "install", "-y", "--reinstall", "/tmp/pzync_update.deb"])
        .status()
        .map_err(|e| format!("Failed to launch installation command via pkexec: {}", e))?;

    if !install_status.success() {
        return Err("Installation was cancelled or failed".to_string());
    }

    println!("[updater] Installation successful!");
    Ok(())
}

#[tauri::command]
pub fn relaunch_app(app: tauri::AppHandle) {
    app.restart();
}
