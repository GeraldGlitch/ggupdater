use serde_json::Value;

pub const PLACEHOLDER: &str = "PENDING";

#[derive(Clone, Debug, Default)]
pub struct UpdateManifest {
    pub app_id: String,
    pub version: String,
    pub version_number: i64,
    pub download_url: String,
    pub sha256: String,
    pub delete_list: Vec<String>,
    pub loaded: bool,
    pub errors: Vec<String>,
}

impl UpdateManifest {
    pub fn from_value(data: &Value, platform: &str) -> Self {
        let mut manifest = UpdateManifest::default();
        if !data.is_object() {
            manifest.errors.push("El manifest está vacío.".to_string());
            return manifest;
        }

        manifest.app_id = data.get("app_id").and_then(Value::as_str).unwrap_or("").trim().to_string();
        manifest.version = data.get("version").and_then(Value::as_str).unwrap_or("").trim().to_string();
        manifest.version_number = Self::parse_version_number(data.get("version_number").unwrap_or(&Value::Null));

        if manifest.app_id.is_empty() {
            manifest.errors.push("El manifest no contiene 'app_id'.".to_string());
        }
        if manifest.version.is_empty() {
            manifest.errors.push("El manifest no contiene 'version'.".to_string());
        }
        if manifest.version_number < 0 {
            manifest.errors.push("El manifest debe contener 'version_number' como entero no negativo.".to_string());
        }

        match data.get(platform).and_then(Value::as_object) {
            Some(block) if !block.is_empty() => {
                manifest.download_url =
                    block.get("url").and_then(Value::as_str).unwrap_or("").trim().to_string();
                manifest.sha256 =
                    block.get("sha256").and_then(Value::as_str).unwrap_or("").trim().to_string();
            }
            _ => manifest.errors.push(format!("El manifest no contiene bloque para la plataforma '{platform}'.")),
        }

        if let Some(items) = data.get("delete").and_then(Value::as_array) {
            for item in items {
                let value = item.as_str().unwrap_or("").trim().to_string();
                if !value.is_empty() {
                    manifest.delete_list.push(value);
                }
            }
        }

        manifest.loaded = manifest.errors.is_empty();
        manifest
    }

    pub fn stub(app_id: &str, version: &str, version_number: i64) -> Self {
        UpdateManifest {
            app_id: app_id.to_string(),
            version: version.to_string(),
            version_number,
            download_url: PLACEHOLDER.to_string(),
            sha256: PLACEHOLDER.to_string(),
            delete_list: Vec::new(),
            loaded: true,
            errors: Vec::new(),
        }
    }

    /// JSON parsea los números como float; se aceptan int, float entero y string numérico.
    pub fn parse_version_number(value: &Value) -> i64 {
        match value {
            Value::Number(number) => {
                if let Some(int) = number.as_i64() {
                    return if int >= 0 { int } else { -1 };
                }
                if let Some(float) = number.as_f64() {
                    if float >= 0.0 && (float - float.round()).abs() < 1e-6 {
                        return float.round() as i64;
                    }
                }
                -1
            }
            Value::String(text) => {
                let text = text.trim();
                match text.parse::<i64>() {
                    Ok(value) if value >= 0 => value,
                    _ => -1,
                }
            }
            _ => -1,
        }
    }

    pub fn is_url_pending(&self) -> bool {
        self.download_url.is_empty() || self.download_url == PLACEHOLDER
    }

    #[allow(dead_code)]
    pub fn is_sha_pending(&self) -> bool {
        self.sha256.is_empty() || self.sha256 == PLACEHOLDER
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_manifest() {
        let value: Value = serde_json::from_str(
            r#"{
                "app_id": "chibifx",
                "version": "1.2.0",
                "version_number": 12,
                "linux": { "url": "https://x/y.zip", "sha256": "abc" },
                "delete": [" old.txt ", ""]
            }"#,
        )
        .unwrap();
        let manifest = UpdateManifest::from_value(&value, "linux");
        assert!(manifest.loaded);
        assert_eq!(manifest.version_number, 12);
        assert_eq!(manifest.download_url, "https://x/y.zip");
        assert_eq!(manifest.delete_list, vec!["old.txt".to_string()]);
    }

    #[test]
    fn rejects_missing_fields_and_wrong_platform() {
        let value: Value = serde_json::from_str(r#"{ "version_number": 1.5 }"#).unwrap();
        let manifest = UpdateManifest::from_value(&value, "windows");
        assert!(!manifest.loaded);
        assert!(manifest.errors.iter().any(|e| e.contains("app_id")));
        assert!(manifest.errors.iter().any(|e| e.contains("version_number")));
        assert!(manifest.errors.iter().any(|e| e.contains("windows")));
    }

    #[test]
    fn parses_version_number_variants() {
        assert_eq!(UpdateManifest::parse_version_number(&serde_json::json!(5)), 5);
        assert_eq!(UpdateManifest::parse_version_number(&serde_json::json!(5.0)), 5);
        assert_eq!(UpdateManifest::parse_version_number(&serde_json::json!("7")), 7);
        assert_eq!(UpdateManifest::parse_version_number(&serde_json::json!(-2)), -1);
        assert_eq!(UpdateManifest::parse_version_number(&serde_json::json!(2.5)), -1);
        assert_eq!(UpdateManifest::parse_version_number(&serde_json::json!(null)), -1);
    }
}
