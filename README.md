# GGUpdater

Updater genérico y reutilizable para aplicaciones desktop (Windows y Linux), escrito en **Rust**
con **egui/eframe**. Es totalmente independiente de la app que actualiza: se ejecuta desde una
subcarpeta y considera `../` como la raíz de la app.

A diferencia de la versión Godot, es un **único ejecutable** (~8.7 MB, ≈3.9 MB gzip) sin `.pck` ni
assets externos: código, logo y banner local van embebidos.

```text
App/
├── App.exe / App.x86_64
├── ...
└── ggupdater/
    └── ggupdater.exe / ggupdater.x86_64
```

> Solo se copia el ejecutable a `ggupdater/`. No hay carpetas, assets ni `.pck` que distribuir.
> Logs, caché y temporales viven en los datos de usuario, nunca junto a la app.

## Argumentos soportados

| Argumento                        | Obligatorio | Descripción                                                        |
| -------------------------------- | ----------- | ------------------------------------------------------------------ |
| `--app <app_id>`                 | Sí*         | Identificador de la app (ej. `blackcatpos`).                       |
| `--executable <archivo>`         | Sí*         | Ejecutable principal relativo a `../` (ej. `BlackCatPOS.exe`).     |
| `--current-version <x.y.z>`      | Sí*         | Versión actual para mostrarla en la UI.                            |
| `--current-version-number <n>`   | Sí*         | Entero monótono usado para decidir la actualización.               |
| `--pid <n>`                      | No          | PID de la app a esperar antes de instalar.                         |
| `--local-update <zip>`           | No          | Modo desarrollo: usa un ZIP local en vez de descargar.             |

\* Si falta `--app`, GGUpdater entra en **modo preview** (ver más abajo) en vez de fallar.

Ejemplo:

```bash
ggupdater.x86_64 --app blackcatpos --executable BlackCatPOS.exe --current-version 1.0.0 --current-version-number 1 --pid 1234
```

## Modo preview (sin `--app`)

Si se ejecuta sin `--app` (por ejemplo con `cargo run`), el updater **no actualiza nada**:
solo carga el carrusel de banners y muestra la UI. Es ideal para previsualizar el diseño.

- No descarga manifest ni actualización.
- Descarga los banners del repo (`banners/banners.json` en GitHub raw).
- Si no hay red, usa la caché local. Si tampoco hay, mantiene el banner local embebido.
- El estado mostrado es `Modo preview (sin --app)`.

> **Advertencia:** en build de release `target_root` es el `../` del ejecutable. En debug (cargo run)
> usa el `../` del directorio de trabajo para no instalar en `target/`.

Si faltan argumentos esenciales, el updater muestra un error claro y lo registra en el log.

## Estructura del proyecto

```text
src/
├── main.rs             # Arranque, canal de eventos y ventana eframe
├── updater.rs          # Máquina de estados y orquestación (hilo aparte)
├── launch.rs → args.rs # Parseo/validación de argumentos
├── config.rs           # Owner/repo/branch del repo central (manifest resuelto por --app)
├── manifest.rs         # Manifest de actualización
├── download.rs         # Descarga HTTP (ureq) con progreso
├── verify.rs           # SHA-256
├── zip_extract.rs      # Extracción segura (zip)
├── install.rs          # Copia a ../ y lista delete
├── banners.rs          # Carrusel remoto + caché + banner local embebido
├── path_utils.rs       # Seguridad de paths / anti traversal
├── process_wait.rs     # Espera a --pid
├── relaunch.rs         # Relanza la app al pulsar OK
├── logger.rs           # Logging a datos de usuario
├── assets_image.rs     # Decodificación PNG/JPG/WebP y assets embebidos
├── ui.rs               # UI egui (estados, progreso, carrusel, botones)
└── events.rs           # Eventos y estados compartidos con la UI
assets/
├── developer_logo.png  # Logo local (siempre disponible)
└── GG.png              # Banner local embebido
banners/                # banners.json e imágenes servidas por el repo
manifests/              # manifest por proyecto: manifests/<app_id>.json
```

## Máquina de estados

`STARTING → LOADING_MANIFEST → LOADING_BANNERS → DOWNLOADING → VERIFYING → EXTRACTING → INSTALLING → COMPLETED / ERROR`

La UI reacciona a cada estado: texto de estado, barra de progreso, porcentaje, versión actual → nueva,
botón `OK` al terminar y botón de cierre en error.

## Rutas en disco (datos de usuario)

```text
Linux:   ~/.local/share/ggupdater/{logs,cache/banners,temp}
Windows: %APPDATA%\ggupdater\{logs,cache\banners,temp}
```

- `logs/` registros por día.
- `cache/banners/` caché de banners (nunca se borra).
- `temp/update.zip` y `temp/extracted/` temporales de la actualización.

## Manifest de actualización

```json
{
  "app_id": "blackcatpos",
  "version": "1.5.0",
  "version_number": 15,
  "windows": { "url": "PENDING", "sha256": "PENDING" },
  "linux":   { "url": "PENDING", "sha256": "PENDING" },
  "delete": []
}
```

Por compatibilidad también se acepta el formato antiguo: `download_url` y `sha256` como objetos por
plataforma (`{"linux": "..."}`) o como string único universal.

Se selecciona automáticamente `windows` o `linux`. `version` solo se muestra en la UI;
`version_number` es un entero no negativo y monótono usado para comparar. El updater solo instala si
el número remoto es mayor; si es igual informa que ya está actualizado y si es menor rechaza el
downgrade. Incrementa `version_number` en cada release. Si `url`/`sha256` valen `PENDING` o están vacíos:

- La descarga se omite salvo en modo local.
- La validación SHA-256 se salta temporalmente (modo desarrollo).

## Banners (carrusel)

El carrusel combina un **banner local embebido** con banners **remotos del repo**:

1. **Local (siempre primero):** `assets/GG.png` va dentro del ejecutable y se muestra aunque no haya
   internet. No se descarga del repo.
2. **Remotos:** se descargan desde `banners/` del repo según `banners/banners.json`. Si el manifest
   lista `GG.png`, se **omite** porque ya es local.

Para actualizar banners remotos: **sube las imágenes y edita `banners/banners.json`**; GGUpdater las
cargará al ejecutarse (rotación cada 3 segundos e indicadores clicables).

`banners/banners.json`:

```json
{
  "banners": [
    { "app_id": "blackcatpos", "url": "promo_blackcat.png", "link": "https://ejemplo.com/promo" },
    { "app_id": "generic",     "url": "promo_1.png", "link": "" }
  ]
}
```

- `url` puede ser relativo (se resuelve contra la base del repo) o absoluto.
- `link` (opcional): URL `http/https` que se abre en el navegador al hacer clic en el banner. Si se
  define en la entrada de `GG.png`, también aplica al banner local. Si queda vacío, el banner no es clickable.
- Soporta WebP/PNG/JPG.
- Descarga a caché en datos de usuario; si no hay red, usa la caché además del banner local.

## Agregar un proyecto (releases centralizados en GGUpdater)

Todos los releases viven en el repo `ggupdater` y cada proyecto tiene su manifest en
`manifests/<app_id>.json`. El binario es **genérico**: `src/config.rs` solo fija el repo central
y el manifest se resuelve con el `--app` que pasa la aplicación, así que un mismo build sirve
para todas las apps:

```rust
pub const GITHUB_OWNER: &str = "GeraldGlitch";
pub const GITHUB_REPO: &str = "ggupdater";   // repo central, fijo
pub const GITHUB_BRANCH: &str = "main";
```

Para agregar un proyecto:

1. Crea `manifests/<app_id>.json` en el repo `ggupdater` (plantilla en
   `manifests/blackcatpos.json`). Los `url` apuntan a los assets `.zip` del release.
2. Publica el release en `ggupdater` con los assets `.zip` (ej. `blackcatpos-win.zip`,
   `blackcatpos-linux.zip`) y calcula su `sha256`.
3. Listo: lanza GGUpdater con `--app <app_id>` y usará ese manifest.

Al ejecutarse, GGUpdater baja
`https://raw.githubusercontent.com/GeraldGlitch/ggupdater/main/manifests/<app_id>.json`
y sigue el flujo normal: verifica `sha256` → extrae → instala en `../`.

> No uses `releases/latest/download/`: con releases centralizados `latest` es del repo
> entero y puede devolver el manifest de otro proyecto. El manifest se resuelve por
> `app_id` en `manifests/`.

## Cómo probar con `--local-update`

1. Prepara un ZIP con el contenido nuevo de la app (sin la carpeta `ggupdater/`, se ignora automáticamente):

   ```bash
   7z a -tzip /ruta/update.zip ./contenido/*
   ```

2. Coloca GGUpdater dentro de la app como `App/ggupdater/` y ejecuta:

   ```bash
   cd App/ggupdater
   ./ggupdater.x86_64 --app miapp --executable App.x86_64 --current-version 1.0.0 --current-version-number 1 --local-update /ruta/update.zip
   ```

   O en desarrollo, desde la raíz del repo (con `../` del cwd como destino):

   ```bash
   cargo run -- --app miapp --executable App.x86_64 --current-version 1.0.0 --current-version-number 1 --local-update /ruta/update.zip
   ```

3. El flujo será: carga ZIP local → extrae → ignora `ggupdater/` → instala en `../` → limpia temporales
   → `Completed` → `OK` → relanza la app.

### Cómo probar `--pid`

```bash
sleep 60 &
./ggupdater.x86_64 --app miapp --executable App.x86_64 --current-version 1.0.0 --current-version-number 1 --pid $!
```

Sin `--pid` el updater espera un intervalo corto de cortesía (3 s).

## Seguridad de paths

- Todo path proveniente del ZIP o del JSON `delete` se valida en `path_utils.rs`.
- Se bloquean rutas absolutas, `../`, `..`, unidades Windows (`C:`) y traversal.
- El destino final siempre debe quedar dentro de `../`.
- `GGUpdater/` y `ggupdater/` están en `EXCLUDED_PATHS` y nunca se sobrescriben ni se eliminan.

## Linux: permisos de ejecución

Al pulsar `OK`, `relaunch.rs` restaura el bit de ejecución del binario lanzado (vía
`PermissionsExt` de Rust). En Windows no se altera nada.

## Compilar

- Linux: `./build.sh` → `dist/ggupdater.x86_64`
- Windows desde Linux: `./build.sh windows` → `dist/ggupdater.exe`. Detecta `mingw-w64` del sistema o
  `llvm-mingw` portátil en `~/.local/share/llvm-mingw` (no requiere sudo).
- Windows nativo: `build.ps1` → `dist/ggupdater.exe`
- Manual: `cargo build --release` (perfil optimizado para tamaño: LTO, strip, `opt-level="z"`)

El binario es autocontenido: solo depende de libc/libm/libgcc y carga GL/X11/Wayland del sistema en
tiempo de ejecución. Para máxima compatibilidad en releases públicos, compilar en CI con una base
con glibc antigua (ej. Ubuntu 20.04/22.04).

## Publicar un release

1. Comprime el contenido nuevo de la app (sin `ggupdater/`) en `<app>-win.zip` / `<app>-linux.zip`.
2. Súbelos como assets del release en el repo `ggupdater`.
3. Actualiza `manifests/<app_id>.json`: `version`, `version_number`, `url` y `sha256` de cada plataforma.
4. `delete` permite eliminar archivos o carpetas viejas durante la instalación.

## Registro (logs)

Log en consola y en `<datos de usuario>/ggupdater/logs/ggupdater_YYYY-MM-DD.log`.
Se registran fecha/hora, app_id, versión actual/destino, plataforma, pasos y errores. No se guardan secretos.

## Tests

```bash
cargo test
```

Cubren parseo/validación de argumentos, manifest, seguridad de paths, SHA-256, bloqueo de ZIP con
path traversal y un flujo end-to-end de `--local-update` (extrae, instala en `../`, respeta
`ggupdater/` y termina en `Completed`).
