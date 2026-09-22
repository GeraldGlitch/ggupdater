use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::Local;

/// Directorio base de datos de usuario del updater.
/// Windows: %APPDATA%\ggupdater · Linux: $XDG_DATA_HOME/ggupdater o ~/.local/share/ggupdater
pub fn user_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata).join("ggupdater");
            }
        }
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("ggupdater");
        }
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".local").join("share").join("ggupdater")
}

#[derive(Clone)]
pub struct Logger {
    dir: PathBuf,
    context: Arc<Mutex<String>>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            dir: user_data_dir().join("logs"),
            context: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn configure(&self, app_id: &str, current_version: &str) {
        let context = serde_json::json!({
            "app_id": app_id,
            "current_version": current_version,
            "platform": std::env::consts::OS,
        })
        .to_string();
        if let Ok(mut slot) = self.context.lock() {
            *slot = context;
        }
    }

    pub fn info(&self, message: &str) {
        self.write("INFO", message);
    }

    pub fn warn(&self, message: &str) {
        self.write("WARN", message);
    }

    pub fn error(&self, message: &str) {
        self.write("ERROR", message);
    }

    fn write(&self, level: &str, message: &str) {
        let now = Local::now();
        let stamp = now.format("%Y-%m-%d %H:%M:%S");
        let context = self.context.lock().map(|c| c.clone()).unwrap_or_default();
        let line = if context.is_empty() {
            format!("{stamp} {level} {message}")
        } else {
            format!("{stamp} {level} {message} | {context}")
        };
        println!("{line}");

        if fs::create_dir_all(&self.dir).is_err() {
            return;
        }
        let log_path = self.dir.join(format!("ggupdater_{}.log", now.format("%Y-%m-%d")));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(file, "{line}");
        }
    }
}
