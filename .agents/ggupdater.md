# GGUpdater (Rust) — Arquitectura

Updater genérico para apps desktop Windows/Linux. Se distribuye como un único ejecutable embebido
en la app, dentro de `ggupdater/`, y considera `../` como la raíz de la app a actualizar.

## Stack

- Rust edición 2024, `eframe`/`egui` 0.36 (backend glow), `ureq` (HTTP/TLS), `zip`, `sha2`, `image`,
  `serde_json`, `chrono`. Sin async runtime: hilos + `std::sync::mpsc`.
- Perfil release optimizado para tamaño (`lto="fat"`, `strip`, `opt-level="z"`, `codegen-units=1`,
  `panic="abort"`). Resultado: un binario de ~8.7 MB.

## Módulos

| Módulo            | Responsabilidad                                                        |
| ----------------- | ---------------------------------------------------------------------- |
| `main.rs`         | Arranque, logger, canal de eventos, ventana eframe                     |
| `args.rs`         | `LaunchArgs::parse` (mismos flags que la versión Godot)                |
| `config.rs`       | Owner/repo/branch + `APP_ID` fijado por build                          |
| `manifest.rs`     | Parseo/validación del manifest y comparación de `version_number`       |
| `download.rs`     | GET de texto y archivo con progreso (ureq)                             |
| `verify.rs`       | SHA-256 en streaming; salta si `PENDING`                               |
| `zip_extract.rs`  | Extracción con anti path-traversal                                     |
| `install.rs`      | Copia a `../` con exclusiones y aplica `delete`                        |
| `path_utils.rs`   | `simplify`, `resolve_under`, `is_inside`, `is_safe_*`, `is_excluded`   |
| `process_wait.rs` | Espera al `--pid` (o cortesía de 3 s)                                  |
| `relaunch.rs`     | Relanza la app y restaura permisos de ejecución en Unix                |
| `banners.rs`      | Carrusel: local embebido + remotos con caché                           |
| `assets_image.rs` | Decodifica PNG/JPG/WebP a `egui::ColorImage`; assets con `include_bytes!` |
| `ui.rs`           | Presentación egui (mismos colores/estados que Godot)                   |
| `events.rs`       | `Event`/`State`/`StateData` compartidos entre hilo y UI                |

## Argumentos

`--app`, `--executable`, `--current-version`, `--current-version-number`, `--pid`, `--local-update`.
Sin `--app` entra en modo preview. `target_root`:

- Release: `parent(exe)/..`
- Debug (`cargo run`): `cwd/..` para no instalar en `target/`

## Estados

`STARTING → LOADING_MANIFEST → LOADING_BANNERS → DOWNLOADING → VERIFYING → EXTRACTING → INSTALLING
→ COMPLETED / ERROR / PREVIEW`

El hilo `updater::run` emite eventos; la UI los drena en cada frame y repinta.

## Manifest

```json
{
  "app_id": "chibifx",
  "version": "1.5.0",
  "version_number": 15,
  "windows": { "url": "https://.../app-win.zip", "sha256": "..." },
  "linux":   { "url": "https://.../app-linux.zip", "sha256": "..." },
  "delete": ["old/file.txt"]
}
```

- Se selecciona el bloque de la plataforma actual.
- Solo instala si `version_number` remoto > local; igual = al día; menor = downgrade rechazado.
- `url`/`sha256` en `PENDING` o vacío: la descarga remota se omite y la verificación se salta
  (el modo `--local-update` siempre omite ambas).

## Banner local

`GG.png` se embebe con `include_bytes!` y se publica a la UI antes de cualquier petición de red.
Si el manifest remoto lista `GG.png`, se omite para no duplicarlo (pero sí se toma su `link`).

Cada entrada de `banners.json` puede incluir `link` (URL `http/https`); al hacer clic en ese banner
se abre en el navegador del sistema (`src/browser.rs`). El banner local también puede llevar `link`
declarándolo en su entrada de `GG.png`.

## Datos de usuario

```text
Linux:   ~/.local/share/ggupdater/{logs,cache/banners,temp}
Windows: %APPDATA%\ggupdater\{logs,cache\banners,temp}
```

Nunca se escribe fuera de `../` excepto estos datos de usuario y los temporales.

## Seguridad

- `is_safe_relative` rechaza absolutos, `..`/`.`, unidad `C:` y NUL.
- `is_inside` verifica que el destino quede bajo el root (comparación de prefijo normalizado).
- `EXCLUDED_PATHS = ["GGUpdater/", "ggupdater/"]` protege al propio updater.
- El ZIP se extrae primero en temporales y solo después se instala; nunca directo sobre `../`.

## Build

```bash
./build.sh            # Linux  -> dist/ggupdater.x86_64
./build.sh windows    # cross  -> dist/ggupdater.exe (mingw-w64)
./build.ps1           # Windows nativo -> dist/ggupdater.exe
```

## Verificación

```bash
cargo test
```

Incluye un end-to-end del flujo local (`updater::tests::local_update_flow_installs_and_completes`)
y un test anti-traversal (`zip_extract::tests::blocks_path_traversal_entries`).
