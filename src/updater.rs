use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use crate::args::LaunchArgs;
use crate::config;
use crate::download;
use crate::events::{Event, State, StateData};
use crate::install;
use crate::logger::{self, Logger};
use crate::manifest::{UpdateManifest, PLACEHOLDER};
use crate::process_wait;
use crate::verify;
use crate::zip_extract;

/// Máquina de estados principal. Corre en su propio hilo y notifica a la UI
/// por el canal de eventos.
pub fn run(args: LaunchArgs, tx: Sender<Event>, logger: Logger) {
    let temp_dir = logger::user_data_dir().join("temp");
    run_with_temp(args, tx, logger, temp_dir);
}

fn run_with_temp(args: LaunchArgs, tx: Sender<Event>, logger: Logger, temp_dir: PathBuf) {
    let zip_path = temp_dir.join("update.zip");
    let extract_dir = temp_dir.join("extracted");

    cleanup_old_temp(&temp_dir, &logger);

    go(&tx, &logger, State::Starting, StateData::message("Starting updater..."));
    send_versions(&tx, &args.current_version, "");

    if args.preview_mode {
        logger.info("Modo preview: sin --app, no se actualizará ninguna aplicación.");
        go(&tx, &logger, State::LoadingBanners, StateData::message("Cargando banners..."));
        go(&tx, &logger, State::Preview, StateData::message("Modo preview (sin --app)"));
        return;
    }

    if !args.valid {
        error(&tx, &logger, &format!("Argumentos inválidos:\n{}", args.errors.join("\n")));
        return;
    }

    go(&tx, &logger, State::LoadingManifest, StateData::message("Cargando manifest..."));

    let manifest = match fetch_manifest(&args, &tx, &logger) {
        Some(manifest) => manifest,
        None => return,
    };

    if manifest.app_id != args.app_id {
        error(
            &tx,
            &logger,
            &format!("El manifest pertenece a '{}' pero la app es '{}'.", manifest.app_id, args.app_id),
        );
        return;
    }

    send_versions(&tx, &args.current_version, &manifest.version);

    if same_version(&args, &manifest) {
        let mut data = StateData::message("Ya tienes la última versión.");
        data.up_to_date = true;
        data.current = args.current_version.clone();
        data.target = manifest.version.clone();
        go(&tx, &logger, State::Completed, data);
        return;
    }
    if is_downgrade(&args, &manifest) {
        error(
            &tx,
            &logger,
            &format!(
                "La versión remota ({}) es menor que la instalada ({}). Se cancela para evitar un downgrade.",
                manifest.version_number, args.current_version_number
            ),
        );
        return;
    }
    if manifest.is_url_pending() && args.local_update.is_empty() {
        error(
            &tx,
            &logger,
            "La URL de descarga del manifest está como PENDING y no se indicó --local-update.",
        );
        return;
    }

    start_download(&args, &manifest, &tx, &logger, &temp_dir, &zip_path, &extract_dir);
}

fn fetch_manifest(args: &LaunchArgs, tx: &Sender<Event>, logger: &Logger) -> Option<UpdateManifest> {
    if !args.local_update.is_empty() {
        logger.info("Modo local: se omite manifest remoto.");
        return Some(UpdateManifest::stub(&args.app_id, &args.current_version, args.current_version_number));
    }

    let url = config::manifest_url(&args.app_id);
    let text = download::download_text(&url, logger);
    if text.is_empty() {
        error(tx, logger, &format!("Manifest inaccesible o sin conexión: {url}"));
        return None;
    }

    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => {
            error(tx, logger, "El manifest no es un JSON válido.");
            return None;
        }
    };

    let manifest = UpdateManifest::from_value(&parsed, &args.platform);
    if !manifest.loaded {
        error(tx, logger, &format!("Manifest inválido:\n{}", manifest.errors.join("\n")));
        return None;
    }

    logger.info(&format!(
        "Manifest cargado: {} -> {} ({})",
        manifest.app_id, manifest.version, manifest.version_number
    ));
    Some(manifest)
}

fn same_version(args: &LaunchArgs, manifest: &UpdateManifest) -> bool {
    args.local_update.is_empty() && manifest.version_number == args.current_version_number
}

fn is_downgrade(args: &LaunchArgs, manifest: &UpdateManifest) -> bool {
    args.local_update.is_empty() && manifest.version_number < args.current_version_number
}

fn start_download(
    args: &LaunchArgs,
    manifest: &UpdateManifest,
    tx: &Sender<Event>,
    logger: &Logger,
    temp_dir: &Path,
    zip_path: &Path,
    extract_dir: &Path,
) {
    let _ = fs::create_dir_all(temp_dir);
    if temp_dir.exists() {
        install::remove_tree(temp_dir);
    }
    let _ = fs::create_dir_all(temp_dir);

    go(tx, logger, State::Downloading, StateData::message("Descargando actualización..."));
    send_progress(tx, 0.0);

    if !args.local_update.is_empty() {
        let source = PathBuf::from(&args.local_update);
        if !source.is_file() {
            error(tx, logger, &format!("El ZIP local no existe: {}", source.display()));
            return;
        }
        if let Err(problem) = fs::copy(&source, zip_path) {
            error(tx, logger, &format!("No se pudo copiar el ZIP local: {problem}"));
            return;
        }
        logger.info(&format!("ZIP local copiado a temporales: {}", source.display()));
    } else {
        let downloaded = download::download_file(&manifest.download_url, zip_path, logger, |received, total| {
            let ratio = if total > 0 {
                (received as f64 / total as f64).clamp(0.0, 1.0) as f32
            } else {
                0.0
            };
            send_progress(tx, ratio);
        });
        if !downloaded {
            error(tx, logger, "Descarga fallida.");
            return;
        }
    }

    if args.local_update.is_empty() && manifest.is_url_pending() {
        error(tx, logger, "El manifest no tiene URL de descarga válida.");
        return;
    }

    verify_step(args, manifest, tx, logger, zip_path, extract_dir);
}

fn verify_step(
    args: &LaunchArgs,
    manifest: &UpdateManifest,
    tx: &Sender<Event>,
    logger: &Logger,
    zip_path: &Path,
    extract_dir: &Path,
) {
    go(tx, logger, State::Verifying, StateData::message("Verificando integridad..."));
    send_progress(tx, 1.0);

    let expected = if args.local_update.is_empty() {
        manifest.sha256.clone()
    } else {
        PLACEHOLDER.to_string()
    };

    if !verify::verify(zip_path, &expected, true, logger) {
        error(tx, logger, "La verificación falló. La actualización se canceló.");
        return;
    }

    extract_step(args, manifest, tx, logger, zip_path, extract_dir);
}

fn extract_step(
    args: &LaunchArgs,
    manifest: &UpdateManifest,
    tx: &Sender<Event>,
    logger: &Logger,
    zip_path: &Path,
    extract_dir: &Path,
) {
    go(tx, logger, State::Extracting, StateData::message("Extrayendo archivos..."));
    if extract_dir.exists() {
        install::remove_tree(extract_dir);
    }
    let _ = fs::create_dir_all(extract_dir);

    let extracted = zip_extract::extract(zip_path, extract_dir, logger, |index, total| {
        if total > 0 {
            send_progress(tx, index as f32 / total as f32);
        }
    });
    if !extracted {
        error(tx, logger, "Error de extracción.");
        return;
    }

    install_step(args, manifest, tx, logger, zip_path, extract_dir);
}

fn install_step(
    args: &LaunchArgs,
    manifest: &UpdateManifest,
    tx: &Sender<Event>,
    logger: &Logger,
    zip_path: &Path,
    extract_dir: &Path,
) {
    go(tx, logger, State::Installing, StateData::message("Instalando actualización..."));
    send_progress(tx, 0.0);

    process_wait::wait_for_exit(args.pid, logger, 120.0);

    if !install::install(extract_dir, &args.target_root, &manifest.delete_list, logger) {
        error(tx, logger, "Error de instalación.");
        return;
    }

    cleanup_after_success(zip_path, extract_dir, logger);

    let mut data = StateData::message("Actualización completada");
    data.current = args.current_version.clone();
    data.target = manifest.version.clone();
    go(tx, logger, State::Completed, data);
}

fn cleanup_after_success(zip_path: &Path, extract_dir: &Path, logger: &Logger) {
    let _ = fs::remove_file(zip_path);
    if extract_dir.exists() {
        install::remove_tree(extract_dir);
    }
    logger.info("Temporales limpiados.");
}

fn cleanup_old_temp(temp_dir: &Path, logger: &Logger) {
    if temp_dir.exists() {
        install::remove_tree(temp_dir);
    }
    let _ = fs::create_dir_all(temp_dir);
    logger.info("Temporales antiguos limpiados al iniciar.");
}

fn go(tx: &Sender<Event>, logger: &Logger, state: State, data: StateData) {
    logger.info(&format!("Estado -> {} {}", state.name(), data.to_json()));
    let _ = tx.send(Event::State(state, data));
}

fn send_versions(tx: &Sender<Event>, current: &str, target: &str) {
    let _ = tx.send(Event::Versions { current: current.to_string(), target: target.to_string() });
}

fn send_progress(tx: &Sender<Event>, ratio: f32) {
    let _ = tx.send(Event::Progress { ratio });
}

fn error(tx: &Sender<Event>, logger: &Logger, message: &str) {
    logger.error(message);
    let _ = tx.send(Event::State(State::Error, StateData::message(message)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::mpsc;

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ggupdater_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_zip(zip_path: &Path, files: &[(&str, &[u8])]) {
        let mut writer = zip::ZipWriter::new(fs::File::create(zip_path).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        for (name, content) in files {
            writer.start_file(*name, options).unwrap();
            writer.write_all(content).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn local_update_flow_installs_and_completes() {
        let root = sandbox("local_update");
        let app_dir = root.join("app");
        let updater_dir = app_dir.join("GGUpdater");
        fs::create_dir_all(&updater_dir).unwrap();
        fs::write(updater_dir.join("self.bin"), b"do not touch").unwrap();

        let zip_path = root.join("update.zip");
        make_zip(&zip_path, &[("newfile.txt", b"new content"), ("sub/nested.txt", b"nested")]);

        let args = LaunchArgs {
            app_id: "testapp".to_string(),
            executable: "app.bin".to_string(),
            current_version: "1.0.0".to_string(),
            current_version_number: 1,
            pid: -1,
            target_root: app_dir.clone(),
            local_update: zip_path.to_string_lossy().to_string(),
            platform: "linux".to_string(),
            valid: true,
            errors: Vec::new(),
            preview_mode: false,
        };

        let (tx, rx) = mpsc::channel();
        run_with_temp(args, tx, Logger::new(), root.join("temp"));

        let events: Vec<Event> = rx.try_iter().collect();
        assert!(events.iter().any(|event| matches!(event, Event::State(State::Completed, _))));
        assert_eq!(fs::read_to_string(app_dir.join("newfile.txt")).unwrap(), "new content");
        assert_eq!(fs::read_to_string(app_dir.join("sub/nested.txt")).unwrap(), "nested");
        assert_eq!(fs::read_to_string(updater_dir.join("self.bin")).unwrap(), "do not touch");

        let _ = fs::remove_dir_all(&root);
    }
}
