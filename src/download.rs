use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use crate::logger::Logger;
use crate::manifest::PLACEHOLDER;

const USER_AGENT: &str = "GGUpdater";
const TIMEOUT_SECS: u64 = 30;
const CHUNK_SIZE: usize = 64 * 1024;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(TIMEOUT_SECS)))
        .build()
        .new_agent()
}

fn error_message(error: &ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(code) => format!("el servidor respondió HTTP {code}"),
        other => format!("{other}"),
    }
}

/// Descarga texto (manifest) de forma síncrona. Devuelve "" si falla.
pub fn download_text(url: &str, logger: &Logger) -> String {
    if url.is_empty() || url == PLACEHOLDER {
        logger.warn("URL de manifest vacía o placeholder.");
        return String::new();
    }

    let response = agent()
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
    if let Some(parent) = destination.parent() {
        if fs::create_dir_all(parent).is_err() {
            logger.error(&format!("No se pudo crear el directorio de destino: {}", parent.display()));
            return false;
        }
    }

    let response = agent()
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "*/*")
        .call();

    let mut response = match response {
        Ok(response) => response,
        Err(error) => {
            logger.error(&format!("Descarga falló: {}.", error_message(&error)));
            return false;
        }
    };

    let total = response
        .headers()
        .get("Content-Length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);

    let mut reader = response.body_mut().as_reader();
    let mut file = match fs::File::create(destination) {
        Ok(file) => file,
        Err(error) => {
            logger.error(&format!("No se pudo escribir el archivo {}: {error}", destination.display()));
            return false;
        }
    };

    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut received: u64 = 0;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                if let Err(error) = file.write_all(&buffer[..count]) {
                    logger.error(&format!("No se pudo escribir el archivo {}: {error}", destination.display()));
                    return false;
                }
                received += count as u64;
                on_progress(received, total);
            }
            Err(error) => {
                logger.error(&format!("Descarga interrumpida: {error}."));
                return false;
            }
        }
    }

    logger.info(&format!("Descargados {received} bytes a {}", destination.display()));
    true
}
