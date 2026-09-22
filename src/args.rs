use std::collections::HashMap;
use std::path::PathBuf;

use crate::path_utils;

#[derive(Clone, Debug)]
pub struct LaunchArgs {
    pub app_id: String,
    pub executable: String,
    pub current_version: String,
    pub current_version_number: i64,
    pub pid: i64,
    pub target_root: PathBuf,
    pub local_update: String,
    pub platform: String,
    pub valid: bool,
    pub errors: Vec<String>,
    pub preview_mode: bool,
}

impl LaunchArgs {
    pub fn parse<I: IntoIterator<Item = String>>(argv: I) -> Self {
        let mut raw: HashMap<String, String> = HashMap::new();
        let tokens: Vec<String> = argv.into_iter().collect();
        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];
            if let Some(rest) = token.strip_prefix("--") {
                let (key, value) = if let Some(eq) = rest.find('=') {
                    (rest[..eq].to_string(), rest[eq + 1..].to_string())
                } else if i + 1 < tokens.len() && !tokens[i + 1].starts_with("--") {
                    i += 1;
                    (rest.to_string(), tokens[i].clone())
                } else {
                    (rest.to_string(), String::new())
                };
                raw.insert(key, value);
            }
            i += 1;
        }

        let mut args = LaunchArgs {
            app_id: get(&raw, "app"),
            executable: get(&raw, "executable"),
            current_version: get(&raw, "current-version"),
            current_version_number: -1,
            pid: -1,
            target_root: path_utils::target_root(),
            local_update: get(&raw, "local-update"),
            platform: current_platform(),
            valid: false,
            errors: Vec::new(),
            preview_mode: false,
        };

        if let Some(text) = raw.get("current-version-number") {
            let text = text.trim();
            match text.parse::<i64>() {
                Ok(value) if value >= 0 => args.current_version_number = value,
                _ => args.errors.push(format!(
                    "El argumento --current-version-number debe ser un entero no negativo: '{text}'"
                )),
            }
        }

        if let Some(text) = raw.get("pid") {
            let text = text.trim();
            match text.parse::<i64>() {
                Ok(value) => args.pid = value,
                Err(_) => args.errors.push(format!("El argumento --pid no es un entero válido: '{text}'")),
            }
        }

        args.validate();
        args
    }

    fn validate(&mut self) {
        self.preview_mode = self.app_id.is_empty();

        if self.preview_mode {
            self.errors.clear();
            self.valid = false;
            return;
        }

        if self.executable.is_empty() {
            self.errors.push("Falta el argumento obligatorio --executable <archivo>.".to_string());
        }
        if self.current_version.is_empty() {
            self.errors.push("Falta el argumento obligatorio --current-version <x.y.z>.".to_string());
        }
        if self.current_version_number < 0 {
            self.errors.push("Falta el argumento obligatorio --current-version-number <n>.".to_string());
        }
        self.errors = dedupe(&self.errors);
        self.valid = self.errors.is_empty();
    }

    pub fn to_dictionary(&self) -> String {
        serde_json::json!({
            "app_id": self.app_id,
            "executable": self.executable,
            "current_version": self.current_version,
            "current_version_number": self.current_version_number,
            "pid": self.pid,
            "target_root": self.target_root.to_string_lossy(),
            "local_update": self.local_update,
            "platform": self.platform,
            "valid": self.valid,
            "preview_mode": self.preview_mode,
        })
        .to_string()
    }
}

fn get(raw: &HashMap<String, String>, key: &str) -> String {
    raw.get(key).map(|value| value.trim().to_string()).unwrap_or_default()
}

fn dedupe(source: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for item in source {
        if seen.insert(item.clone()) {
            out.push(item.clone());
        }
    }
    out
}

fn current_platform() -> String {
    std::env::consts::OS.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(list: &[&str]) -> LaunchArgs {
        LaunchArgs::parse(list.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_full_command_line() {
        let args = parse_args(&[
            "--app",
            "chibifx",
            "--executable",
            "bin/App.x86_64",
            "--current-version",
            "1.0.0",
            "--current-version-number=7",
            "--pid",
            "1234",
            "--local-update",
            "/tmp/update.zip",
        ]);
        assert!(args.valid);
        assert!(!args.preview_mode);
        assert_eq!(args.app_id, "chibifx");
        assert_eq!(args.executable, "bin/App.x86_64");
        assert_eq!(args.current_version, "1.0.0");
        assert_eq!(args.current_version_number, 7);
        assert_eq!(args.pid, 1234);
        assert_eq!(args.local_update, "/tmp/update.zip");
    }

    #[test]
    fn without_app_enters_preview_mode() {
        let args = parse_args(&["--executable", "App.exe"]);
        assert!(args.preview_mode);
        assert!(!args.valid);
        assert!(args.errors.is_empty());
    }

    #[test]
    fn reports_missing_arguments_and_invalid_numbers() {
        let args = parse_args(&["--app", "x", "--current-version-number", "abc", "--pid", "no"]);
        assert!(!args.valid);
        assert!(args.errors.iter().any(|e| e.contains("--executable")));
        assert!(args.errors.iter().any(|e| e.contains("--current-version ")));
        assert!(args.errors.iter().any(|e| e.contains("entero no negativo")));
        assert!(args.errors.iter().any(|e| e.contains("--pid")));
    }
}
