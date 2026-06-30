fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let env_path = std::path::Path::new(&manifest_dir).join(".env");

    println!("cargo:rerun-if-changed=.env");

    if env_path.exists() {
        let content = std::fs::read_to_string(&env_path)
            .expect(".envファイルの読み込みに失敗しました");
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                if matches!(key, "GOOGLE_CLIENT_ID" | "GOOGLE_CLIENT_SECRET") {
                    println!("cargo:rustc-env={key}={value}");
                }
            }
        }
    }

    tauri_build::build()
}
