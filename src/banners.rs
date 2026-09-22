use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use serde_json::Value;

use crate::assets_image;
use crate::download;
use crate::events::{BannerImage, Event};
use crate::logger::{self, Logger};

pub const BANNERS_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/GeraldGlitch/ggupdater/main/banners/banners.json";
pub const BANNERS_BASE_URL: &str =
    "https://raw.githubusercontent.com/GeraldGlitch/ggupdater/main/banners/";

/// Banner local embebido en el ejecutable. Se muestra siempre primero y no se
/// descarga del repo (se omite si aparece en el manifest).
pub const LOCAL_BANNER_FILE: &str = "GG.png";

const SUPPORTED_EXTENSIONS: [&str; 4] = ["webp", "png", "jpg", "jpeg"];

pub fn cache_dir() -> PathBuf {
    logger::user_data_dir().join("cache").join("banners")
}

pub fn load_banners(app_id: String, tx: Sender<Event>, logger: Logger) {
    let mut banners: Vec<BannerImage> = Vec::new();

    let mut has_local = false;
    if let Some(local) = assets_image::to_color_image(assets_image::LOCAL_BANNER_PNG) {
        banners.push(BannerImage { image: local, link: None });
        has_local = true;
        logger.info(&format!("Banner local precargado: {LOCAL_BANNER_FILE}"));
        let _ = tx.send(Event::BannersPartial(banners.clone()));
    }

    let text = download::download_text(BANNERS_MANIFEST_URL, &logger);
    let entries = parse_manifest(&text);
    if entries.is_empty() {
        logger.warn("Manifest de banners vacío o inaccesible; usando caché.");
        append_from_cache(&mut banners, &logger);
        finish(banners, &tx, &logger);
        return;
    }

    // El banner local no se descarga, pero sí puede tomar su `link` del manifest.
    if has_local {
        banners[0].link = entries.iter().find(|entry| is_local_entry(entry)).and_then(entry_link);
    }

    let entries = ordered_by_app(entries, &app_id);
    let mut loaded = 0usize;
    for entry in &entries {
        if is_local_entry(entry) {
            let url = entry.get("url").and_then(Value::as_str).unwrap_or("");
            logger.info(&format!("Se omite banner local del manifest: {url}"));
            continue;
        }

        let url = resolve_entry_url(entry);
        if url.is_empty() {
            continue;
        }
        let local = cache_path_for(entry, &url);
        let downloaded = download::download_file(&url, &local, &logger, |_, _| {});
        if !downloaded && !local.is_file() {
            continue;
        }

        if let Some(image) = load_texture(&local, &logger) {
            banners.push(BannerImage { image, link: entry_link(entry) });
            loaded += 1;
        }
    }

    logger.info(&format!("Carrusel listo: 1 local + {loaded} remotos."));
    finish(banners, &tx, &logger);
}

fn finish(banners: Vec<BannerImage>, tx: &Sender<Event>, logger: &Logger) {
    logger.info(&format!("Carrusel con {} banners.", banners.len()));
    let _ = tx.send(Event::BannersReady(banners));
}

fn parse_manifest(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }
    let parsed: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    if let Some(array) = parsed.as_array() {
        return array.clone();
    }
    parsed.get("banners").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn ordered_by_app(entries: Vec<Value>, app_id: &str) -> Vec<Value> {
    let (mut matching, rest): (Vec<Value>, Vec<Value>) = entries
        .into_iter()
        .partition(|entry| entry.get("app_id").and_then(Value::as_str).unwrap_or("") == app_id);
    matching.extend(rest);
    matching
}

fn entry_url(entry: &Value) -> String {
    entry.get("url").and_then(Value::as_str).unwrap_or("").trim().to_string()
}

/// Enlace opcional al que llevar al hacer clic en el banner.
fn entry_link(entry: &Value) -> Option<String> {
    entry
        .get("link")
        .and_then(Value::as_str)
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

/// True si la entrada del manifest corresponde al banner local (no descargar).
fn is_local_entry(entry: &Value) -> bool {
    let raw = entry_url(entry).replace('\\', "/");
    let file = raw.rsplit('/').next().unwrap_or("").to_lowercase();
    file == LOCAL_BANNER_FILE.to_lowercase()
}

/// Resuelve la URL: relativa se une a BANNERS_BASE_URL; absoluta se usa tal cual.
fn resolve_entry_url(entry: &Value) -> String {
    let raw = entry_url(entry);
    if raw.is_empty() {
        return String::new();
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return raw;
    }
    format!("{BANNERS_BASE_URL}{}", raw.trim_start_matches('/'))
}

fn cache_path_for(entry: &Value, url: &str) -> PathBuf {
    let mut file_name = url.rsplit('/').next().unwrap_or("").to_string();
    if file_name.is_empty() {
        file_name = format!("banner_{}", stable_hash(url));
    }
    let app_part = entry
        .get("app_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("generic");
    cache_dir().join(format!("{app_part}_{file_name}").replace(' ', "_"))
}

fn stable_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn append_from_cache(banners: &mut Vec<BannerImage>, logger: &Logger) {
    let dir = cache_dir();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut added = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
        if !is_supported(&name) || is_cached_local_banner(&name) {
            continue;
        }
        if let Some(image) = load_texture(&path, logger) {
            banners.push(BannerImage { image, link: None });
            added += 1;
        }
    }

    if added == 0 {
        logger.warn("Sin banners en caché.");
    } else {
        logger.info(&format!("Añadidos {added} banners desde caché."));
    }
}

fn is_cached_local_banner(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    let local = LOCAL_BANNER_FILE.to_lowercase();
    lower == local || lower.ends_with(&format!("_{local}"))
}

fn is_supported(file_name: &str) -> bool {
    let extension = file_name.rsplit('.').next().unwrap_or("").to_lowercase();
    SUPPORTED_EXTENSIONS.contains(&extension.as_str())
}

fn load_texture(path: &std::path::Path, logger: &Logger) -> Option<eframe::egui::ColorImage> {
    let bytes = fs::read(path).ok()?;
    let image = assets_image::to_color_image(&bytes);
    if image.is_none() {
        logger.warn(&format!("No se pudo decodificar banner: {}", path.display()));
    }
    image
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_optional_links() {
        assert_eq!(entry_link(&json!({ "link": " https://a.b " })), Some("https://a.b".to_string()));
        assert_eq!(entry_link(&json!({ "link": "" })), None);
        assert_eq!(entry_link(&json!({})), None);
    }

    #[test]
    fn detects_local_banner_entries() {
        assert!(is_local_entry(&json!({ "url": "GG.png" })));
        assert!(is_local_entry(&json!({ "url": "./GG.PNG" })));
        assert!(!is_local_entry(&json!({ "url": "chibifx.png" })));
    }

    #[test]
    fn resolves_relative_and_absolute_urls() {
        assert_eq!(resolve_entry_url(&json!({ "url": "promo.png" })), format!("{BANNERS_BASE_URL}promo.png"));
        assert_eq!(resolve_entry_url(&json!({ "url": "https://x/y.png" })), "https://x/y.png");
        assert_eq!(resolve_entry_url(&json!({ "url": "" })), "");
    }
}
