use std::path::{Component, Path, PathBuf};

pub const EXCLUDED_PATHS: [&str; 2] = ["GGUpdater/", "ggupdater/"];

const FORBIDDEN_SEGMENTS: [&str; 2] = ["..", "."];

/// Root absoluto de la app a actualizar. En builds de release es siempre `../`
/// respecto al ejecutable; en debug (equivalente al modo editor de Godot) es
/// `../` respecto al directorio de trabajo.
pub fn target_root() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        if let Ok(cwd) = std::env::current_dir() {
            return simplify(&cwd.join(".."));
        }
    }
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let dir = exe.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    simplify(&dir.join(".."))
}

/// Normaliza un path de forma léxica (equivalente a simplify_path de Godot).
pub fn simplify(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn to_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Rechaza paths absolutos, con traversal, unidades Windows o NUL.
pub fn is_safe_relative(relative: &str) -> bool {
    if relative.is_empty() || relative.contains('\0') {
        return false;
    }
    let normalized = relative.replace('\\', "/");
    if normalized.starts_with('/') {
        return false;
    }
    if normalized.as_bytes().len() >= 2 && normalized.as_bytes()[1] == b':' {
        return false;
    }
    !normalized.split('/').any(|segment| FORBIDDEN_SEGMENTS.contains(&segment))
}

/// Une root + relative y devuelve un path absoluto simplificado.
pub fn resolve_under(root: &Path, relative: &str) -> PathBuf {
    let rel = relative.replace('\\', "/");
    let rel = rel.strip_prefix('/').unwrap_or(&rel);
    simplify(&root.join(rel))
}

/// True si candidate queda dentro de root (o es igual a root).
pub fn is_inside(root: &Path, candidate: &Path) -> bool {
    let root_abs = to_slash(&simplify(root));
    let root_abs = root_abs.trim_end_matches('/');
    let cand = to_slash(&simplify(candidate));
    cand == root_abs || cand.starts_with(&format!("{root_abs}/"))
}

/// Comprueba que relative sea seguro y quede dentro de root.
pub fn is_safe_target(root: &Path, relative: &str) -> bool {
    if !is_safe_relative(relative) {
        return false;
    }
    is_inside(root, &resolve_under(root, relative))
}

/// True si relative está protegido por la lista de exclusiones.
pub fn is_excluded(relative: &str, excluded: &[&str]) -> bool {
    let normalized = relative.replace('\\', "/");
    let normalized = normalized.strip_prefix('/').unwrap_or(&normalized);
    excluded.iter().any(|entry| {
        let prefix = entry.replace('\\', "/");
        let prefix = prefix.strip_prefix('/').unwrap_or(&prefix);
        normalized == prefix || normalized.starts_with(prefix)
    })
}

/// Normaliza un path venido del ZIP para usarlo como path relativo seguro.
pub fn sanitize_zip_path(entry_path: &str) -> String {
    let mut normalized = entry_path.replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_string();
    }
    normalized.strip_prefix('/').unwrap_or(&normalized).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_relatives() {
        assert!(is_safe_relative("a/b.txt"));
        assert!(!is_safe_relative(""));
        assert!(!is_safe_relative("/abs/path"));
        assert!(!is_safe_relative("../evil"));
        assert!(!is_safe_relative("a/../../evil"));
        assert!(!is_safe_relative("C:/windows"));
        assert!(!is_safe_relative("a/./b"));
    }

    #[test]
    fn resolves_and_checks_inside() {
        let root = PathBuf::from("/tmp/app");
        assert_eq!(resolve_under(&root, "a/b"), PathBuf::from("/tmp/app/a/b"));
        assert_eq!(resolve_under(&root, "../x"), PathBuf::from("/tmp/x"));
        assert!(is_inside(&root, Path::new("/tmp/app/a")));
        assert!(is_inside(&root, Path::new("/tmp/app")));
        assert!(!is_inside(&root, Path::new("/tmp/application")));
        assert!(!is_safe_target(&root, "../x"));
        assert!(is_safe_target(&root, "bin/app"));
    }

    #[test]
    fn exclusions_match_prefix_with_slash() {
        assert!(is_excluded("GGUpdater/file.txt", &EXCLUDED_PATHS));
        assert!(is_excluded("ggupdater", &EXCLUDED_PATHS) == false);
        assert!(!is_excluded("GGUpdater", &EXCLUDED_PATHS));
        assert!(!is_excluded("bin/app", &EXCLUDED_PATHS));
    }

    #[test]
    fn sanitizes_zip_entries() {
        assert_eq!(sanitize_zip_path(".\\a\\b.txt"), "a/b.txt");
        assert_eq!(sanitize_zip_path("././a/b"), "a/b");
        assert_eq!(sanitize_zip_path("/a/b"), "a/b");
    }

    #[test]
    fn simplify_resolves_dot_segments() {
        assert_eq!(simplify(Path::new("/a/b/../c")), PathBuf::from("/a/c"));
        assert_eq!(simplify(Path::new("/a/./b")), PathBuf::from("/a/b"));
    }
}
