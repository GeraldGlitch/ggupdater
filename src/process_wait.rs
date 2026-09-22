use std::thread::sleep;
use std::time::Duration;

use crate::logger::Logger;

const DEFAULT_WAIT_SECONDS: f64 = 3.0;
const POLL_INTERVAL: f64 = 0.5;

/// Espera a que termine el proceso indicado por --pid. Sin PID espera un
/// intervalo corto de cortesía antes de continuar.
pub fn wait_for_exit(pid: i64, logger: &Logger, max_seconds: f64) -> bool {
    if pid <= 0 {
        logger.info(&format!("Sin --pid; esperando {DEFAULT_WAIT_SECONDS:.1}s de cortesía."));
        sleep(Duration::from_millis((DEFAULT_WAIT_SECONDS * 1000.0) as u64));
        return true;
    }

    if !is_process_alive(pid) {
        logger.info(&format!("El proceso {pid} ya no está activo."));
        return true;
    }

    logger.info(&format!("Esperando cierre del proceso {pid}..."));
    let mut waited = 0.0f64;
    while waited < max_seconds {
        if !is_process_alive(pid) {
            logger.info(&format!("Proceso {pid} cerrado tras {waited:.1}s."));
            return true;
        }
        sleep(Duration::from_millis((POLL_INTERVAL * 1000.0) as u64));
        waited += POLL_INTERVAL;
    }

    logger.warn(&format!("Timeout esperando al proceso {pid}; se continúa igualmente."));
    false
}

pub fn is_process_alive(pid: i64) -> bool {
    if pid <= 0 {
        return false;
    }
    #[cfg(target_os = "windows")]
    {
        alive_windows(pid)
    }
    #[cfg(not(target_os = "windows"))]
    {
        alive_unix(pid)
    }
}

#[cfg(not(target_os = "windows"))]
fn alive_unix(pid: i64) -> bool {
    if std::path::Path::new(&format!("/proc/{pid}")).exists() {
        return true;
    }
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn alive_windows(pid: i64) -> bool {
    let output = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output();
    match output {
        Ok(output) => String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()),
        Err(_) => false,
    }
}
