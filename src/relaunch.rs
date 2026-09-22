use std::path::Path;
use std::process::Command;

use crate::logger::Logger;
use crate::path_utils::{self, EXCLUDED_PATHS};

/// Relanza el ejecutable de la app tras completar la actualización.
/// En Unix asegura permisos de ejecución; en Windows no altera nada.
pub fn relaunch(target_root: &Path, executable: &str, logger: &Logger) -> bool {
    if path_utils::is_excluded(executable, &EXCLUDED_PATHS) {
        logger.error("Por seguridad no se relanza un ejecutable dentro de GGUpdater/.");
        return false;
    }
    if !path_utils::is_safe_target(target_root, executable) {
        logger.error(&format!("Ruta de ejecutable insegura: {executable}"));
        return false;
    }

    let absolute = path_utils::resolve_under(target_root, executable);
    if !absolute.is_file() {
        logger.error(&format!("El ejecutable no existe: {}", absolute.display()));
        return false;
    }

    #[cfg(unix)]
    ensure_executable(&absolute, logger);

    let working_dir = absolute.parent().map(Path::to_path_buf).unwrap_or_else(|| target_root.to_path_buf());
    match Command::new(&absolute).current_dir(working_dir).spawn() {
        Ok(child) => {
            logger.info(&format!("Aplicación relanzada (pid {}): {}", child.id(), absolute.display()));
            true
        }
        Err(error) => {
            logger.error(&format!("No se pudo lanzar el ejecutable {}: {error}", absolute.display()));
            false
        }
    }
}

#[cfg(unix)]
fn ensure_executable(path: &Path, logger: &Logger) {
    use std::os::unix::fs::PermissionsExt;

    match std::fs::metadata(path) {
        Ok(metadata) => {
            let mut permissions = metadata.permissions();
            permissions.set_mode(permissions.mode() | 0o111);
            match std::fs::set_permissions(path, permissions) {
                Ok(()) => logger.info(&format!("Permiso de ejecución restaurado en: {}", path.display())),
                Err(error) => logger.warn(&format!("No se pudo marcar como ejecutable: {} ({error})", path.display())),
            }
        }
        Err(error) => logger.warn(&format!("No se pudieron leer permisos de {} ({error})", path.display())),
    }
}
