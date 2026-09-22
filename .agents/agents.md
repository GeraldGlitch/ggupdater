# Contexto del Proyecto

- Stack: Rust (edición 2024) + egui/eframe (backend glow). Sin Godot.
- Función: updater genérico para aplicaciones desktop Windows y Linux.
- Distribución: un único ejecutable por build (sin `.pck` ni assets externos); todo va embebido.
- Configuración: cada build queda atado a un `APP_ID` en `src/config.rs` y se lanza con `--app <app_id>`.
  El repo central (`GeraldGlitch/ggupdater`) aloja `manifests/<app_id>.json`, `banners/` y los releases.
- Manifest: bloque por plataforma (`windows`/`linux`) con `url` y `sha256`, más `app_id`, `version`,
  `version_number` y `delete`.
- Decisión de actualización: GGUpdater compara `version_number`; solo instala si el remoto es mayor,
  informa si es igual y rechaza downgrades.
- Rutas de usuario: `~/.local/share/ggupdater` (Linux) o `%APPDATA%\ggupdater` (Windows) para logs,
  caché de banners y temporales. Nunca escribe dentro de la app salvo la instalación en `../`.

## Build

- `./build.sh` → `dist/ggupdater.x86_64` (Linux)
- `./build.sh windows` → `dist/ggupdater.exe` (cross con mingw-w64)
- `build.ps1` → `dist/ggupdater.exe` (Windows nativo)

## Verificación

- `cargo test` (tests de args, manifest, path_utils, extracción segura y flujo `--local-update` completo).
- Smoke test de UI: `cargo run` (modo preview sin `--app`).
