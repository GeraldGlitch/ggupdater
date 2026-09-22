use std::fs;
use std::path::Path;

use crate::logger::Logger;
use crate::path_utils::{self, EXCLUDED_PATHS};

/// Copia los archivos extraídos hacia el root de la app (`../`) y aplica la
/// lista delete. Toda operación verifica que el destino siga dentro del root.
pub fn install(
    staged_root: &Path,
    target_root: &Path,
    delete_list: &[String],
    logger: &Logger,
) -> bool {
    if !staged_root.is_dir() {
        logger.error(&format!("No existe la carpeta extraída: {}", staged_root.display()));
        return false;
    }
    if !target_root.is_dir() {
        logger.error(&format!("No existe la carpeta destino de la app: {}", target_root.display()));
        return false;
    }

    let target_abs = path_utils::simplify(target_root);
    let staged_abs = path_utils::simplify(staged_root);

    let installed = match install_directory(&staged_abs, &staged_abs, &target_abs, logger) {
        Some(count) => count,
        None => return false,
    };

    let deleted = apply_delete_list(delete_list, &target_abs, logger);

    logger.info(&format!("Instalados {installed} archivos, eliminados {deleted} elementos."));
    true
}

fn install_directory(
    root_staged: &Path,
    current_dir: &Path,
    target_root: &Path,
    logger: &Logger,
) -> Option<usize> {
    let entries = match fs::read_dir(current_dir) {
        Ok(entries) => entries,
        Err(error) => {
            logger.error(&format!("No se pudo abrir el directorio {}: {error}", current_dir.display()));
            return None;
        }
    };

    let mut count = 0usize;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                logger.error(&format!("No se pudo leer una entrada de {}: {error}", current_dir.display()));
                return None;
            }
        };

        let absolute = entry.path();
        let relative = match absolute.strip_prefix(root_staged) {
            Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };

        if path_utils::is_excluded(&relative, &EXCLUDED_PATHS) {
            logger.info(&format!("Excluido durante instalación: {relative}"));
            continue;
        }

        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        if is_dir {
            count += install_directory(root_staged, &absolute, target_root, logger)?;
        } else {
            if !path_utils::is_safe_target(target_root, &relative) {
                logger.error(&format!("Ruta de destino insegura bloqueada: {relative}"));
                return None;
            }
            let destination = path_utils::resolve_under(target_root, &relative);
            if !copy_file(&absolute, &destination, logger) {
                return None;
            }
            count += 1;
        }
    }

    Some(count)
}

fn copy_file(source: &Path, destination: &Path, logger: &Logger) -> bool {
    if let Some(parent) = destination.parent() {
        if !parent.as_os_str().is_empty() && fs::create_dir_all(parent).is_err() {
            logger.error(&format!("No se pudo crear la carpeta para: {}", destination.display()));
            return false;
        }
    }

    match fs::copy(source, destination) {
        Ok(_) => true,
        Err(error) => {
            logger.error(&format!(
                "No se pudo copiar a {} (¿permisos o archivo bloqueado?): {error}",
                destination.display()
            ));
            false
        }
    }
}

fn apply_delete_list(delete_list: &[String], target_root: &Path, logger: &Logger) -> usize {
    let mut deleted = 0usize;
    for raw_path in delete_list {
        let relative = path_utils::sanitize_zip_path(raw_path);
        if relative.is_empty() {
            continue;
        }
        if path_utils::is_excluded(&relative, &EXCLUDED_PATHS) {
            logger.warn(&format!("delete bloqueado por exclusión: {relative}"));
            continue;
        }
        if !path_utils::is_safe_target(target_root, &relative) {
            logger.warn(&format!("delete bloqueado (ruta insegura): {relative}"));
            continue;
        }

        let absolute = path_utils::resolve_under(target_root, &relative);
        if absolute.is_dir() {
            if remove_tree(&absolute) {
                deleted += 1;
                logger.info(&format!("Carpeta eliminada: {relative}"));
            }
        } else if absolute.is_file() && fs::remove_file(&absolute).is_ok() {
            deleted += 1;
            logger.info(&format!("Archivo eliminado: {relative}"));
        }
    }
    deleted
}

pub fn remove_tree(path: &Path) -> bool {
    fs::remove_dir_all(path).is_ok()
}
