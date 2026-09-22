use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::logger::Logger;
use crate::manifest::PLACEHOLDER;

const CHUNK_SIZE: usize = 1 << 20;

/// Verifica el SHA-256 del ZIP. Si el hash es placeholder/vacío salta la
/// validación (modo dev). En error borra el archivo si así se pide.
pub fn verify(zip_path: &Path, expected_sha: &str, delete_on_error: bool, logger: &Logger) -> bool {
    if !zip_path.is_file() {
        logger.error(&format!("No existe el archivo a verificar: {}", zip_path.display()));
        return false;
    }

    if expected_sha.is_empty() || expected_sha == PLACEHOLDER {
        logger.warn("SHA-256 PENDING; se omite validación (modo desarrollo).");
        return true;
    }

    let actual = match sha256_file(zip_path) {
        Ok(actual) => actual,
        Err(error) => {
            logger.error(&format!("No se pudo calcular el SHA-256 de {}: {error}", zip_path.display()));
            return false;
        }
    };

    if !actual.eq_ignore_ascii_case(expected_sha) {
        if delete_on_error {
            let _ = fs::remove_file(zip_path);
        }
        logger.error(&format!(
            "SHA-256 incorrecto. Esperado={expected_sha} Obtenido={actual} (archivo eliminado)."
        ));
        return false;
    }

    logger.info("SHA-256 verificado correctamente.");
    true
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut file = File::open(path)?;
    let mut buffer = vec![0u8; CHUNK_SIZE];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex_encode(&hasher.finalize()))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_placeholder_and_validates_hash() {
        let logger = Logger::new();
        let path = std::env::temp_dir().join("ggupdater_verify_test.bin");
        fs::write(&path, b"ggupdater").unwrap();
        let expected = "e5f4c1e2cbd1a99a6f8ffb0d6f0f4b0d3b0d0b3a0a6b3f4c1e2cbd1a99a6f8ff";
        assert!(verify(&path, PLACEHOLDER, false, &logger));
        assert!(!verify(&path, expected, false, &logger));
        let actual = sha256_file(&path).unwrap();
        assert!(verify(&path, &actual, false, &logger));
        let _ = fs::remove_file(&path);
    }
}
