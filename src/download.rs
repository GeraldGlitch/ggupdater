use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use crate::logger::Logger;
use crate::manifest::PLACEHOLDER;

const USER_AGENT: &str = "GGUpdater";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
// Las descargas grandes pueden tardar muchos minutos en conexiones lentas.
// Este límite es solo una red de seguridad contra conexiones colgadas.
const DOWNLOAD_BODY_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);
const DOWNLOAD_ATTEMPTS: u32 = 3;
const RETRY_BACKOFF: Duration = Duration::from_secs(1);
const CHUNK_SIZE: usize = 64 * 1024;

/// Respuestas pequeñas (manifest/banners): un límite global es seguro.
fn text_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(RESPONSE_TIMEOUT))
        .build()
        .new_agent()
}

/// Descargas de archivos: sin límite global, porque el ZIP de una release
/// puede tardar más de 30 segundos. Solo se limitan conexión, cabeceras y un
/// máximo muy holgado para el cuerpo.
fn file_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_recv_response(Some(RESPONSE_TIMEOUT))
        .timeout_recv_body(Some(DOWNLOAD_BODY_TIMEOUT))
        .build()
        .new_agent()
}

fn error_message(error: &ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(code) => format!("el servidor respondió HTTP {code}"),
        other => format!("{other}"),
    }
}

fn is_retryable(error: &ureq::Error) -> bool {
    match error {
        ureq::Error::StatusCode(code) => matches!(*code, 408 | 425 | 429) || *code >= 500,
        _ => true,
    }
}

struct DownloadFailure {
    message: String,
    retryable: bool,
}

/// Descarga texto (manifest) de forma síncrona. Devuelve "" si falla.
pub fn download_text(url: &str, logger: &Logger) -> String {
    if url.is_empty() || url == PLACEHOLDER {
        logger.warn("URL de manifest vacía o placeholder.");
        return String::new();
    }

    let response = text_agent()
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json, text/plain, */*")
        .call();

    match response {
        Ok(mut response) => match response.body_mut().read_to_string() {
            Ok(text) => {
                logger.info(&format!("Manifest descargado ({} bytes).", text.len()));
                text
            }
            Err(error) => {
                logger.error(&format!("Manifest no se pudo leer: {error}."));
                String::new()
            }
        },
        Err(error) => {
            logger.error(&format!("Manifest no se pudo descargar: {}.", error_message(&error)));
            String::new()
        }
    }
}

/// Descarga un archivo a disco con progreso real (recibidos, total).
///
/// Reintenta fallos temporales (cortes de red, HTTP 5xx, 408/429) y elimina
/// el archivo parcial antes de volver a intentarlo.
pub fn download_file(
    url: &str,
    destination: &Path,
    logger: &Logger,
    mut on_progress: impl FnMut(u64, u64),
) -> bool {
    if url.is_empty() || url == PLACEHOLDER {
        logger.error("URL de descarga no configurada.");
        return false;
    }
    if let Some(parent) = destination.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        logger.error(&format!(
            "No se pudo crear el directorio de destino {}: {error}",
            parent.display()
        ));
        return false;
    }

    for attempt in 1..=DOWNLOAD_ATTEMPTS {
        if attempt > 1 {
            let delay = RETRY_BACKOFF * (attempt - 1);
            logger.info(&format!(
                "Reintentando descarga en {} s (intento {attempt}/{DOWNLOAD_ATTEMPTS})...",
                delay.as_secs()
            ));
            std::thread::sleep(delay);
        }
        on_progress(0, 0);
        match download_once(url, destination, logger, &mut on_progress) {
            Ok(()) => return true,
            Err(failure) => {
                let _ = fs::remove_file(destination);
                logger.error(&format!(
                    "Descarga falló (intento {attempt}/{DOWNLOAD_ATTEMPTS}): {}.",
                    failure.message
                ));
                if !failure.retryable || attempt == DOWNLOAD_ATTEMPTS {
                    break;
                }
                logger.warn("El fallo parece temporal; se reintentará.");
            }
        }
    }

    false
}

fn download_once(
    url: &str,
    destination: &Path,
    logger: &Logger,
    on_progress: &mut impl FnMut(u64, u64),
) -> Result<(), DownloadFailure> {
    let response = file_agent()
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "*/*")
        .call();

    let mut response = match response {
        Ok(response) => response,
        Err(error) => {
            return Err(DownloadFailure {
                message: error_message(&error),
                retryable: is_retryable(&error),
            });
        }
    };

    let total = response
        .headers()
        .get("Content-Length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);

    let mut reader = response.body_mut().as_reader();
    let mut file = fs::File::create(destination).map_err(|error| DownloadFailure {
        message: format!("no se pudo escribir {}: {error}", destination.display()),
        retryable: false,
    })?;

    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut received: u64 = 0;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                file.write_all(&buffer[..count]).map_err(|error| DownloadFailure {
                    message: format!("no se pudo escribir {}: {error}", destination.display()),
                    retryable: false,
                })?;
                received += count as u64;
                on_progress(received, total);
            }
            Err(error) => {
                return Err(DownloadFailure {
                    message: format!("descarga interrumpida: {error}"),
                    retryable: true,
                });
            }
        }
    }

    if total > 0 && received != total {
        return Err(DownloadFailure {
            message: format!("descarga incompleta: {received} de {total} bytes"),
            retryable: true,
        });
    }

    logger.info(&format!("Descargados {received} bytes a {}", destination.display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_downloads_have_no_global_timeout() {
        let timeouts = file_agent().config().timeouts();
        assert_eq!(timeouts.global, None);
        assert_eq!(timeouts.connect, Some(CONNECT_TIMEOUT));
        assert_eq!(timeouts.recv_response, Some(RESPONSE_TIMEOUT));
        assert_eq!(timeouts.recv_body, Some(DOWNLOAD_BODY_TIMEOUT));
    }

    #[test]
    fn only_small_text_responses_have_a_global_timeout() {
        let timeouts = text_agent().config().timeouts();
        assert_eq!(timeouts.global, Some(RESPONSE_TIMEOUT));
    }

    #[test]
    #[ignore = "slow: simula una descarga de 35s para verificar que no hay corte a los 30s"]
    fn slow_body_longer_than_30s_finishes() {
        use std::io::{Read as _, Write as _};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let total = 35u64;

        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = socket.read(&mut request);
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            socket.flush().unwrap();
            for _ in 0..total {
                socket.write_all(b"x").unwrap();
                socket.flush().unwrap();
                thread::sleep(Duration::from_secs(1));
            }
        });

        let destination = std::env::temp_dir().join(format!(
            "ggupdater_slow_{}_{}.bin",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let started = std::time::Instant::now();
        let downloaded = download_file(
            &format!("http://{address}/slow.bin"),
            &destination,
            &Logger::new(),
            |_, _| {},
        );
        let elapsed = started.elapsed();
        server.join().unwrap();

        assert!(downloaded, "la descarga de 35 s no debe abortarse");
        assert!(elapsed.as_secs() >= 30);
        assert_eq!(fs::metadata(&destination).unwrap().len(), total);
        let _ = fs::remove_file(&destination);
    }

    #[test]
    fn retries_only_transient_status_codes() {
        assert!(is_retryable(&ureq::Error::StatusCode(500)));
        assert!(is_retryable(&ureq::Error::StatusCode(503)));
        assert!(is_retryable(&ureq::Error::StatusCode(429)));
        assert!(!is_retryable(&ureq::Error::StatusCode(404)));
        assert!(!is_retryable(&ureq::Error::StatusCode(403)));
    }
}
