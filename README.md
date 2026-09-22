# GGUpdater

Updater genérico y reutilizable para aplicaciones desktop (Windows y Linux), construido en Godot 4.
Es totalmente independiente de la app que actualiza: se ejecuta desde una subcarpeta y considera `../` como la raíz de la app.

```text
App/
├── App.exe / App.x86_64
├── ...
└── GGUpdater/
    ├── GGUpdater.exe / GGUpdater.x86_64
    └── GGUpdater.pck
```

## Argumentos soportados

| Argumento                | Obligatorio | Descripción                                                        |
| ------------------------ | ----------- | ------------------------------------------------------------------ |
| `--app <app_id>`         | Sí*         | Identificador de la app (ej. `blackcatpos`).                       |
| `--executable <archivo>` | Sí*         | Ejecutable principal relativo a `../` (ej. `BlackCatPOS.exe`).     |
| `--current-version <x.y.z>`      | Sí*         | Versión actual para mostrarla en la UI.                            |
| `--current-version-number <n>`   | Sí*         | Entero de la versión actual. Obligatorio por compatibilidad; no decide. |
| `--pid <n>`              | No          | PID de la app a esperar antes de instalar.                         |
| `--local-update <zip>`   | No          | Modo desarrollo: usa un ZIP local en vez de descargar.             |

\* Si falta `--app`, GGUpdater entra en **modo preview** (ver más abajo) en vez de fallar.

> No uses `--version`: es un flag reservado del motor Godot. El binario exportado lo intercepta,
> imprime su versión y sale sin ejecutar el updater. Usa `--current-version`.

Ejemplo:

```bash
GGUpdater --app blackcatpos --executable BlackCatPOS.exe --current-version 1.0.0 --current-version-number 1 --pid 1234
```

## Modo preview (sin `--app`)

Si se ejecuta sin `--app` (por ejemplo al darle Run en el editor), el updater **no actualiza nada**:
solo carga el carrusel de banners y muestra la UI. Es ideal para previsualizar el diseño.

- No descarga manifest ni actualización.
- Descarga los banners del repo (`banners/banners.json` en GitHub raw).
- Si no hay red, usa la caché local. Si tampoco hay, mantiene el developer logo.
- El estado mostrado es `Modo preview (sin --app)`.

Para que el flujo real funcione al lanzarlo desde el editor, puedes definir los argumentos en
`project.godot` → `[editor] run/main_run_args` o usar **Debug → Customize Run Instances**.

> **Advertencia:** con argumentos reales, `target_root` es el `../` del proyecto. Ejecutar el flujo
> desde el editor instala en la carpeta padre del proyecto. Para probar sin riesgo hazlo en una
> copia con estructura `App/GGUpdater/`.

Si faltan argumentos esenciales, el updater muestra un error claro y lo registra en el log.

## Estructura del proyecto

```text
scripts/
├── main.gd              # Punto de entrada (autoload Main)
├── updater_controller.gd# Máquina de estados y orquestación
├── launch_args.gd       # Parseo/validación de argumentos
├── project_config.gd    # Owner/repo del repositorio central de manifests
├── update_manifest.gd   # Manifest de actualización
├── download_manager.gd  # Descarga HTTP con progreso
├── update_verifier.gd   # SHA-256
├── zip_manager.gd       # Extracción segura (ZIPReader)
├── install_manager.gd   # Copia a ../ y lista delete
├── banner_manager.gd    # Carrusel remoto + caché
├── path_utils.gd        # Seguridad de paths / anti traversal
├── process_waiter.gd    # Espera a --pid
├── app_relauncher.gd    # Relanza la app al pulsar OK
└── logger.gd            # Logging a user://ggupdater/logs/
ui/
├── bootstrap.tscn       # Escena principal vacía; el autoload Main monta la UI
├── updater_ui.tscn
└── updater_ui.gd
assets/
└── developer_logo.svg   # Logo local (siempre disponible)
manifests/               # manifest por proyecto: manifests/<app_id>.json
```

## Máquina de estados

`STARTING → LOADING_MANIFEST → LOADING_BANNERS → DOWNLOADING → VERIFYING → EXTRACTING → INSTALLING → COMPLETED / ERROR`

La UI reacciona a cada estado: texto de estado, barra de progreso, porcentaje, versión actual → nueva, botón `OK` al terminar y botón de cierre/reintento en error.

## Rutas en disco (user://)

```text
user://ggupdater/logs/                  # registros
user://ggupdater/cache/                 # caché de banners (nunca se borra)
user://ggupdater/temp/update.zip        # ZIP temporal
user://ggupdater/temp/extracted/        # extracción temporal
```

## Manifest de actualización

```json
{
  "app_id": "blackcatpos",
  "version": "1.5.0",
  "version_number": 15,
  "download_url": {
    "windows": "https://github.com/GeraldGlitch/ggupdater/releases/download/.../blackcatpos-win.zip",
    "linux": "https://github.com/GeraldGlitch/ggupdater/releases/download/.../blackcatpos-linux.zip"
  },
  "sha256": {
    "windows": "PENDING",
    "linux": "PENDING"
  },
  "delete": []
}
```

GGUpdater obtiene `download_url` directamente del manifest `manifests/<app_id>.json`
correspondiente al `--app` recibido y elige la entrada según el SO (Windows o Linux). No resuelve ni
construye URLs por su cuenta, y no decide si existe actualización: **la app decide** si hay una nueva
versión antes de lanzar GGUpdater.

- `download_url` es obligatorio y debe apuntar directo al asset `.zip` del GitHub Release de la
  plataforma. También se acepta un string único (ZIP universal) en vez del objeto por plataforma.
  Si falta la entrada del SO, está vacía o la descarga falla, se muestra un error y **no se modifica
  la instalación existente**.
- `version` y `version_number` son informativos (UI y logs); no alteran el flujo.
- `sha256` es opcional y acepta el mismo formato (string u objeto por plataforma). Si está vacío o
  vale `PENDING`, la validación se salta (modo desarrollo). El código ya está listo para activarla
  sin cambios.
- `delete` es opcional: lista de paths a eliminar en `../` tras instalar.

## Banners (carrusel)

El carrusel combina un **banner local precargado** con banners **remotos del repo**:

1. **Local (siempre primero):** `assets/banners/GG.png` viene embebido en el ejecutable y se
   muestra aunque no haya internet. No se descarga del repo.
2. **Remotos:** se descargan desde la carpeta `banners/` del repo según `banners/banners.json`.
   Si el manifest lista `GG.png`, se **omite** porque ya es local.

Para actualizar banners remotos: **sube las imágenes y edita `banners/banners.json`**; GGUpdater
las cargará al ejecutarse.

En `scripts/banner_manager.gd`:

```gdscript
const BANNERS_MANIFEST_URL := "https://raw.githubusercontent.com/GeraldGlitch/ggupdater/main/banners/banners.json"
const BANNERS_BASE_URL := "https://raw.githubusercontent.com/GeraldGlitch/ggupdater/main/banners/"

const LOCAL_BANNERS_DIR := "res://assets/banners/"
const LOCAL_BANNER_FILE := "GG.png"   # banner local que nunca se descarga
```

`banners/banners.json`:

```json
{
  "banners": [
    { "app_id": "blackcatpos", "url": "promo_blackcat.png" },
    { "app_id": "generic",     "url": "promo_1.png" }
  ]
}
```

- `url` puede ser relativo (se resuelve contra `BANNERS_BASE_URL`) o absoluto.
- El banner local se muestra primero; el resto rota cada `ROTATION_INTERVAL` segundos.
- Soporta WebP/PNG/JPG.
- Descarga a caché en `user://ggupdater/cache/banners/`. Si no hay red, usa la caché
  además del banner local.

> Nota: `banners/` tiene un `.gdignore` (solo para el contenido remoto). El banner local vive en
> `assets/banners/` para que Godot lo importe y quede embebido en el ejecutable.

## Agregar un proyecto (releases centralizados en GGUpdater)

Todos los releases viven en el repo `ggupdater` y cada proyecto tiene su manifest en
`manifests/<app_id>.json` dentro de ese repo. GGUpdater es una **única build genérica**: no se ata a
un proyecto en tiempo de compilación. El manifest se resuelve con el argumento `--app`, así que
`scripts/project_config.gd` solo define el repositorio central:

```gdscript
const GITHUB_OWNER := "GeraldGlitch"
const GITHUB_REPO  := "ggupdater"   # repo central, fijo
const GITHUB_BRANCH := "main"
```

Para publicar una actualización:

1. Publica el release en `ggupdater` con los assets `.zip` (ej. `blackcatpos-win.zip` y
   `blackcatpos-linux.zip`) y copia el **enlace directo** de cada uno.
2. Crea o edita `manifests/<app_id>.json` y coloca cada enlace en `download_url.windows` /
   `download_url.linux` (opcionalmente `sha256` por plataforma, `delete` y la `version` para la UI).
3. Lanza GGUpdater con `--app <app_id>`.

Al ejecutarse, GGUpdater baja
`https://raw.githubusercontent.com/GeraldGlitch/ggupdater/main/manifests/<app_id>.json`,
selecciona `download_url` según el SO y sigue el flujo normal: descarga → verifica `sha256` (si
existe) → extrae → instala en `../`.

> La app es quien detecta si existe una actualización antes de lanzar GGUpdater. GGUpdater no
> compara versiones ni construye URLs: solo usa el `download_url` que corresponde a su plataforma.

## Cómo probar con `--local-update`

1. Prepara un ZIP con el contenido nuevo de la app (sin la carpeta `GGUpdater/`, se ignora automáticamente):

   ```bash
   7z a -tzip /ruta/update.zip ./contenido/*
   ```

2. Coloca GGUpdater dentro de la app como `App/GGUpdater/` y ejecuta:

   ```bash
   cd App/GGUpdater
   godot --path . -- --app miapp --executable bin/App.x86_64 --current-version 1.0.0 --current-version-number 1 --local-update /ruta/update.zip
   ```

   O con el binario exportado:

   ```bash
   GGUpdater.x86_64 --app miapp --executable bin/App.x86_64 --current-version 1.0.0 --current-version-number 1 --local-update /ruta/update.zip
   ```

3. El flujo será: carga ZIP local → extrae → ignora `GGUpdater/` → instala en `../` → limpia temporales → `Completed` → `OK` → relanza la app.

### Cómo probar `--pid`

```bash
# En Linux, lanza un proceso cualquiera y usa su PID
sleep 60 &
GGUpdater.x86_64 --app miapp --executable bin/App.x86_64 --current-version 1.0.0 --current-version-number 1 --pid $!
```

Sin `--pid` el updater espera un intervalo corto de cortesía (3 s).

## Seguridad de paths

- Todo path proveniente del ZIP o del JSON `delete` se valida en `path_utils.gd`.
- Se bloquean rutas absolutas, `../`, `..`, unidades Windows (`C:`) y traversal.
- El destino final siempre debe quedar dentro de `../`.
- `GGUpdater/` y `ggupdater/` están en `EXCLUDED_PATHS` y nunca se sobrescriben ni se eliminan. Puedes añadir más carpetas protegidas ahí.

## Linux: permisos de ejecución

Al pulsar `OK`, `app_relauncher.gd` restaura `chmod +x` sobre el ejecutable (vía `chmod` del sistema, ya que Godot 4.7 no expone API nativa de permisos). En Windows no se altera nada.

## Exportar para Windows y Linux

Requisitos: plantillas de exportación instaladas (Editor → Manage Export Templates).

1. **Linux** (`GGUpdater.x86_64`):
   - Project → Export → Add → Linux/X11.
   - Export Path: `GGUpdater.x86_64`, modo "X11 (binary)".
   - Export Project.

2. **Windows** (`GGUpdater.exe`):
   - Project → Export → Add → Windows Desktop.
   - Export Path: `GGUpdater.exe`.
   - Export Project.

3. Copia `GGUpdater(.exe)` **y** `GGUpdater.pck` dentro de `App/GGUpdater/`.

Export por línea de comandos:

```bash
godot --headless --path . --export-release "Linux/X11" ./build/GGUpdater.x86_64
godot --headless --path . --export-release "Windows Desktop" ./build/GGUpdater.exe
```

## Registro (logs)

Log en consola y en `user://ggupdater/logs/ggupdater_YYYY-MM-DD.log`.
Se registran fecha/hora, app_id, versión actual/destino, plataforma, pasos y errores. No se guardan secretos.

## Restricciones respetadas

- Godot 4, sin librerías externas.
- Genérico: no asume ni hardcodea ningún `app_id`.
- No toca archivos fuera de `../`.
- No se actualiza a sí mismo (ignora `GGUpdater/`).
- Compatible con Windows y Linux.
