use std::fs;
use std::io;
use std::path::Path;

use zip::ZipArchive;

use crate::logger::Logger;
use crate::path_utils;

/// Extrae un ZIP de forma segura (anti path-traversal). Nunca escribe fuera
/// del directorio de extracción.
pub fn extract(
    zip_path: &Path,
    output_dir: &Path,
    logger: &Logger,
    mut on_progress: impl FnMut(usize, usize),
) -> bool {
    if !zip_path.is_file() {
        logger.error(&format!("El archivo ZIP no existe: {}", zip_path.display()));
        return false;
    }

    let file = match fs::File::open(zip_path) {
        Ok(file) => file,
        Err(error) => {
            logger.error(&format!("No se pudo abrir el ZIP: {error}"));
            return false;
        }
    };
    let mut archive = match ZipArchive::new(file) {
        Ok(archive) => archive,
        Err(error) => {
            logger.error(&format!("No se pudo abrir el ZIP (archivo inválido o corrupto): {error}"));
            return false;
        }
    };

    let total = archive.len();
    if total == 0 {
        logger.error("El ZIP no contiene archivos.");
        return false;
    }

    if let Err(error) = fs::create_dir_all(output_dir) {
        logger.error(&format!("No se pudo crear el directorio de extracción: {error}"));
        return false;
    }

    let output_abs = path_utils::simplify(output_dir);
    let mut extracted = 0usize;

    for index in 0..total {
        let mut entry = match archive.by_index(index) {
            Ok(entry) => entry,
            Err(error) => {
                logger.error(&format!("Entrada ZIP ilegible (#{index}): {error}"));
                return false;
            }
        };

        let raw_name = entry.name().to_string();
        let mut relative = path_utils::sanitize_zip_path(&raw_name);
        if relative.is_empty() {
            continue;
        }

        let is_dir = relative.ends_with('/');
        relative = relative.trim_end_matches('/').to_string();

        if !path_utils::is_safe_relative(&relative) {
            logger.error(&format!("Entrada ZIP insegura bloqueada: {raw_name}"));
            return false;
        }

        let destination = path_utils::resolve_under(&output_abs, &relative);
        if !path_utils::is_inside(&output_abs, &destination) {
            logger.error(&format!("Entrada ZIP escapó del directorio destino: {raw_name}"));
            return false;
        }

        if is_dir {
            let _ = fs::create_dir_all(&destination);
            continue;
        }

        if let Some(parent) = destination.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                logger.error(&format!("No se pudo crear directorio para {relative}: {error}"));
                return false;
            }
        }

        let mut out = match fs::File::create(&destination) {
            Ok(out) => out,
            Err(error) => {
                logger.error(&format!("No se pudo escribir el archivo extraído {relative}: {error}"));
                return false;
            }
        };
        if let Err(error) = io::copy(&mut entry, &mut out) {
            logger.error(&format!("No se pudo extraer {relative}: {error}"));
            return false;
        }

        extracted += 1;
        on_progress(index + 1, total);
    }

    logger.info(&format!("Extraídos {extracted} archivos a {}", output_abs.display()));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn blocks_path_traversal_entries() {
        let root = std::env::temp_dir().join(format!("ggupdater_zip_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let zip_path = root.join("evil.zip");
        {
            let mut writer = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
            writer.start_file("../evil.txt", zip::write::SimpleFileOptions::default()).unwrap();
            writer.write_all(b"boom").unwrap();
            writer.finish().unwrap();
        }

        let output = root.join("out");
        let logger = Logger::new();
        assert!(!extract(&zip_path, &output, &logger, |_, _| {}));
        assert!(!root.join("evil.txt").exists());

        let _ = fs::remove_dir_all(&root);
    }
}
