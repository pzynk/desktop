fn main() {
    let output_path = "/tmp/test_ss_run.png";
    let _ = std::fs::remove_file(output_path);

    println!("=== Testing flameshot ===");
    let flameshot_res = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("timeout 5 flameshot full -r > {}", output_path))
        .output();

    match &flameshot_res {
        Ok(out) => {
            println!("flameshot exit status: {}", out.status);
            println!("flameshot stderr: {}", String::from_utf8_lossy(&out.stderr));
            println!("flameshot stdout len: {}", out.stdout.len());
        }
        Err(e) => {
            println!("flameshot spawn error: {}", e);
        }
    }

    let is_valid_file = std::path::Path::new(output_path).exists()
        && std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0) > 0;
    println!("Flameshot produced valid file: {}", is_valid_file);
}
