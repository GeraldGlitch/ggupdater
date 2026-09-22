use std::process::Command;

/// Abre una URL http/https en el navegador por defecto del sistema.
/// Devuelve false si el esquema no está permitido o no se pudo lanzar.
pub fn open_url(url: &str) -> bool {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return false;
    }

    #[cfg(target_os = "windows")]
    let spawned = Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn();

    #[cfg(target_os = "macos")]
    let spawned = Command::new("open").arg(url).spawn();

    #[cfg(all(unix, not(target_os = "macos")))]
    let spawned = Command::new("xdg-open").arg(url).spawn();

    spawned.is_ok()
}
