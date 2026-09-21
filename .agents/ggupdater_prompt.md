# Prompt reutilizable: integrar actualizaciones con GGUpdater

> **Cómo usar este archivo:** rellena la tabla de configuración (o deja que el agente la detecte) y
> pasa **todo el contenido de la sección "PROMPT"** como tarea inicial a un agente dentro del
> proyecto destino. El agente implementa el chequeo de versión y el lanzamiento de GGUpdater.
> GGUpdater ya existe y **no se modifica** desde el proyecto destino.

## Configuración a rellenar

| Dato                       | Descripción                                                                                      | Ejemplo                                  |
| -------------------------- | ------------------------------------------------------------------------------------------------ | ---------------------------------------- |
| `APP_ID`                   | Slug del proyecto. Debe coincidir con `manifests/<app_id>.json` y con `--app`. Minúsculas, sin guiones. | `chibifx`                          |
| Archivo de versión local   | JSON dentro del proyecto, empaquetado en el build.                                                | `res://version.json`                     |
| `version_number` inicial   | Entero monótono. Si el proyecto ya trae numeración previa, continúa desde ahí.                    | `1`                                      |
| Carpeta de export          | Donde quedan el ejecutable y el `.pck`. Ahí mismo debe quedar la carpeta `ggupdater/`.            | `EXPORTS/MiApp/`                         |
| Ejecutable                 | Nombre del binario dentro de la carpeta de export, por plataforma.                                | `MiApp.x86_64` / `MiApp.exe`             |

---

# PROMPT

## Contexto

Trabajas en una app de escritorio hecha en **Godot 4** para Windows y Linux. Debes integrar el
sistema de actualizaciones **GGUpdater**. **No modifiques GGUpdater**: solo consultas su manifest y
lo lanzas cuando corresponde.

GGUpdater está centralizado en el repo `GeraldGlitch/ggupdater`:

- Cada app tiene un manifest remoto en
  `https://raw.githubusercontent.com/GeraldGlitch/ggupdater/main/manifests/<APP_ID>.json`.
- El manifest declara:
  - `version`: texto (semver) que solo se muestra en la UI.
  - `version_number`: entero no negativo y monótono. **Es el único criterio para decidir si hay actualización.**
  - `windows` / `linux`: `url` del ZIP del release y su `sha256`.
  - `delete`: lista opcional de archivos/carpetas viejas a eliminar al instalar.
- El updater se despliega junto a la app en una carpeta `ggupdater/`:

  ```text
  MiApp/
  ├── MiApp.x86_64  (o MiApp.exe)
  ├── MiApp.pck
  └── ggupdater/
      ├── ggupdater.x86_64  (o ggupdater.exe)
      └── ggupdater.pck
  ```

- GGUpdater considera `../` (la carpeta que contiene `ggupdater/`) como raíz de la app: instala ahí
  y **nunca** toca la carpeta `ggupdater/`.

## Objetivo

1. Consultar la versión remota en el manifest.
2. Si `version_number` remoto **es mayor** que el local, mostrar un popup de actualización.
3. Solo si el usuario pulsa **"Actualizar"**, lanzar GGUpdater y cerrar la app.

## Paso 0 - Detección previa

Antes de tocar código, confirma:

- `APP_ID` del proyecto (si no está claro, pregunta a Gerald).
- Ruta del archivo de versión local (por defecto `res://version.json`). Si no existe, créalo.
- Carpeta y nombre del ejecutable exportado (Windows y Linux).
- El sistema de UI/diálogos que ya use la app (para reutilizar el estilo del popup).

## Paso 1 - Archivo de versión local

Crea o actualiza `version.json` en la raíz del proyecto:

```json
{
  "version": "1.0.0",
  "version_number": 1,
  "devmessage": "0"
}
```

- `version`: texto que se muestra en la UI.
- `version_number`: entero monótono. **Nunca baja.** Se incrementa en cada release.
- `devmessage`: opcional, para mensajes del desarrollador.

> El archivo queda empaquetado dentro del PCK. En la build exportada `res://` es de solo lectura:
> no intentes escribirlo en runtime. Al actualizar, el ZIP trae la `.pck` nueva con el
> `version.json` actualizado, por eso el número local queda al día solo.

## Paso 2 - Consultar el manifest remoto

Implementa un chequeo al iniciar la app (que no bloquee ni rompa la UI si falla):

```gdscript
const GG_UPDATER_OWNER := "GeraldGlitch"
const GG_UPDATER_REPO := "ggupdater"
const GG_UPDATER_BRANCH := "main"
const APP_ID := "<APP_ID>"

func _manifest_url() -> String:
    return "https://raw.githubusercontent.com/%s/%s/%s/manifests/%s.json" % [
        GG_UPDATER_OWNER, GG_UPDATER_REPO, GG_UPDATER_BRANCH, APP_ID,
    ]

func _check_for_updates() -> void:
    var http := HTTPRequest.new()
    http.timeout = 10.0
    add_child(http)
    http.request_completed.connect(_on_update_check_completed)
    # Cache-buster: raw.githubusercontent cachea ~5 min; esto evita leer un manifest viejo.
    var url := _manifest_url() + "?t=%d" % int(Time.get_unix_time_from_system())
    var err := http.request(url)
    if err != OK:
        http.queue_free()
```

Reglas del parseo:

- Si no hay respuesta `200`, JSON inválido, no es `Dictionary` o el `app_id` no coincide: **no
  muestres nada** (fallo silencioso, la app sigue normal).
- Lee `remote.version_number` y compara con `local.version_number` de `res://version.json`.
- Hay actualización **solo si `remote.version_number > local.version_number`** (nunca `>=`).
- Si el remoto es menor, ignóralo (downgrade no permitido).

## Paso 3 - Popup de actualización

- Si hay actualización, guarda el manifest remoto en una variable y muestra un popup con:
  - Título: `remote.update_title` si existe, si no `"Actualización disponible"`.
  - Texto: `remote.update_msg` si existe, si no `"Nueva versión %s → %s"` con las versiones.
  - Botones: **"Actualizar"** (confirmar) y **"Cancelar"**.
- El manifest puede traer `update_title` / `update_msg` opcionales (GGUpdater los ignora, la app
  los usa para el texto). Añádelos al manifest si quieres personalizar el mensaje.
- No lances nada todavía: solo al pulsar "Actualizar".

Opcional (mensaje de desarrollador): si el manifest trae `devmessage` (entero), `message_title` y
`message_msg`, y su número es mayor que el `devmessage` local, muestra un popup informativo y guarda
el nuevo valor en `user://` (no en `res://`, que es de solo lectura en la build).

## Paso 4 - Lanzar GGUpdater

Al confirmar el usuario:

```gdscript
func _launch_updater(local_data: Dictionary) -> void:
    var exe_path := OS.get_executable_path()
    var app_root := exe_path.get_base_dir()

    var updater_name := "ggupdater.exe" if OS.get_name() == "Windows" else "ggupdater.x86_64"
    var updater_path := app_root.path_join("ggupdater").path_join(updater_name)
    if not FileAccess.file_exists(updater_path):
        _show_error("No se encontró GGUpdater en: %s" % updater_path)
        return

    var args := PackedStringArray([
        "--app", APP_ID,
        "--executable", exe_path.get_file(),        # relativo a la raíz de la app
        "--current-version", str(local_data.get("version", "")),
        "--current-version-number", str(int(local_data.get("version_number", 0))),
        "--pid", str(OS.get_process_id()),          # GGUpdater espera a que esta app cierre
    ])

    var pid := OS.create_process(updater_path, args, false)
    if pid <= 0:
        _show_error("No se pudo iniciar GGUpdater.")
        return

    get_tree().quit()  # cierra la app para que el updater pueda sobrescribir archivos
```

Notas críticas:

- `--executable` debe ser la ruta **relativa a la raíz de la app**. Con la estructura acordada
  (`ggupdater/` al mismo nivel que el ejecutable) basta `exe_path.get_file()`. Si el ejecutable
  estuviera en una subcarpeta, calcula su ruta relativa respecto a `app_root`.
- **Nunca uses `--version`**: es un flag reservado del motor Godot (el binario exportado lo
  intercepta, imprime su versión y sale). Usa `--current-version` y `--current-version-number`.
- `--pid` es clave: GGUpdater espera a que la app cierre antes de instalar. Además evita
  sobrescribir el ejecutable en uso.
- Tras la instalación, GGUpdater muestra `OK` y relanza solo la app con `--executable`.

## Paso 5 - Exportar y desplegar

1. Exporta la app (Linux y/o Windows) con el `version.json` ya actualizado.
2. Copia manualmente la carpeta `ggupdater/` junto al ejecutable exportado:

   ```text
   MiApp/
   ├── MiApp.x86_64  (o MiApp.exe)
   ├── MiApp.pck
   └── ggupdater/
       ├── ggupdater.x86_64  (o ggupdater.exe)
       └── ggupdater.pck
   ```

3. En Linux, asegura permiso de ejecución del updater: `chmod +x ggupdater/ggupdater.x86_64`.

## Paso 6 - Publicar una release (repo `ggupdater`)

Este paso se hace en el repo central `GeraldGlitch/ggupdater`, no en la app:

1. **Bump de versión**: sube `version` y `version_number` en el `version.json` de la app y
   re-exporta. El `version_number` del manifest debe ser **mayor** que el local instalado.
2. **Arma el ZIP** con el contenido del export en la raíz (sin la carpeta `ggupdater/`):

   ```bash
   cd "EXPORTS/MiApp"
   zip -r "/tmp/miapp-linux.zip" . -x "ggupdater/*"
   sha256sum "/tmp/miapp-linux.zip"
   ```

   Windows (PowerShell):

   ```powershell
   Compress-Archive -Path ".\*" -DestinationPath "$env:TEMP\miapp-win.zip" -Force
   Get-FileHash "$env:TEMP\miapp-win.zip" -Algorithm SHA256
   ```

   El ZIP debe tener los archivos en la **raíz**, no dentro de una carpeta contenedora.

3. **Publica el release** en `GeraldGlitch/ggupdater` con tag `<app_id>-v<version>` y adjunta los
   ZIP (ej. `miapp-linux.zip`, `miapp-win.zip`).
4. **Actualiza el manifest** `manifests/<app_id>.json` del repo `ggupdater`:

   ```json
   {
     "app_id": "<app_id>",
     "version": "1.0.1",
     "version_number": 2,
     "windows": {
       "url": "https://github.com/GeraldGlitch/ggupdater/releases/download/<app_id>-v1.0.1/<app_id>-win.zip",
       "sha256": "<sha256>"
     },
     "linux": {
       "url": "https://github.com/GeraldGlitch/ggupdater/releases/download/<app_id>-v1.0.1/<app_id>-linux.zip",
       "sha256": "<sha256>"
     },
     "delete": []
   }
   ```

5. Opcional: agrega un banner en `banners/banners.json` con el `app_id` para promocionar la release.

> No uses `releases/latest/download/`: con releases centralizados `latest` es del repo completo y
> puede devolver el ZIP de otro proyecto. Usa siempre la URL del tag exacto.

## Paso 7 - Pruebas

1. **Prueba local del updater** (sin publicar release):

   ```bash
   cd "EXPORTS/MiApp/ggupdater"
   ./ggupdater.x86_64 --app <app_id> --executable MiApp.x86_64 \
     --current-version 1.0.0 --current-version-number 1 \
     --local-update /ruta/update.zip
   ```

   El flujo debe ser: carga ZIP → verifica → extrae → ignora `ggupdater/` → instala en `../` →
   `Completed` → `OK` → relanza la app.

2. **Prueba del popup**: con el manifest remoto ya configurado, ejecuta la app con un
   `version_number` local menor que el remoto. Debe aparecer el popup y, al pulsar "Actualizar",
   abrirse GGUpdater y cerrarse la app.

3. **Prueba real completa**: corre la app, actualiza, y al pulsar `OK` en el updater verifica que la
   app vuelve a abrir con la versión nueva (el `version.json` local actualizado).

## Reglas / no hacer

- No modificar el código de GGUpdater desde el proyecto destino.
- No lanzar GGUpdater sin consentimiento del usuario.
- No comparar con `>=`: solo se actualiza si el remoto es estrictamente mayor.
- No usar `--version` (reservado por Godot).
- No incluir la carpeta `ggupdater/` dentro del ZIP de la app.
- No confiar en rutas del ZIP: GGUpdater ya valida paths y bloquea traversal.
- La app **debe cerrarse** antes de que GGUpdater instale (por eso se pasa `--pid`).
- Si `url`/`sha256` del manifest valen `PENDING`, GGUpdater falla salvo que se use
  `--local-update` (solo para desarrollo).

## Validación final (checklist para el agente)

- [ ] `version.json` local con `version` y `version_number`.
- [ ] Chequeo HTTP del manifest con fallo silencioso si no hay red.
- [ ] Comparación estricta `remote.version_number > local.version_number`.
- [ ] Popup solo si hay actualización.
- [ ] Lanzamiento de GGUpdater solo al pulsar "Actualizar".
- [ ] Args correctos: `--app`, `--executable`, `--current-version`, `--current-version-number`, `--pid`.
- [ ] La app se cierra tras lanzar el updater.
- [ ] `ggupdater/` queda junto al ejecutable exportado (Windows y Linux).

---

**# FIN DEL PROMPT**
