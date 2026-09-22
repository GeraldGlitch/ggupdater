# Prompt reutilizable: integrar actualizaciones con GGUpdater

> **Cómo usar este archivo:** rellena la tabla de configuración (o deja que el agente la detecte) y
> pasa **todo el contenido de la sección "PROMPT"** como tarea inicial a un agente dentro del
> proyecto destino. El agente implementa el chequeo de versión y el lanzamiento de GGUpdater.
> GGUpdater ya existe y **no se modifica** desde el proyecto destino.

## Configuración a rellenar

| Dato                     | Descripción                                                                                          | Ejemplo                        |
| ------------------------ | ---------------------------------------------------------------------------------------------------- | ------------------------------ |
| `APP_ID`                 | Slug del proyecto. Debe coincidir con `manifests/<app_id>.json` y con `--app`. Minúsculas, sin guiones. | `chibifx`                    |
| Archivo de versión local | JSON dentro del proyecto, empaquetado en el build.                                                   | `res://version.json`           |
| `version_number` inicial | Entero monótono. Si el proyecto ya trae numeración previa, continúa desde ahí.                       | `1`                            |
| Carpeta de export        | Donde quedan el ejecutable y la carpeta `ggupdater/`.                                                | `EXPORTS/MiApp/`               |
| Ejecutable               | Nombre del binario dentro de la carpeta de export, por plataforma.                                   | `MiApp.x86_64` / `MiApp.exe`   |

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
  - `app_id`: slug de la app. Debe coincidir con `<APP_ID>` y con `--app`.
  - `version`: texto (semver) que solo se muestra en la UI.
  - `version_number`: entero no negativo y monótono. GGUpdater lo compara: si el remoto es mayor
    instala, si es igual informa que está al día y si es menor cancela (anti-downgrade).
  - `windows` / `linux`: bloque de plataforma con `url` (enlace **directo** al asset `.zip` de esa
    plataforma) y `sha256`. Si `url`/`sha256` faltan o valen `PENDING`, la descarga se omite salvo
    en modo local y la verificación se salta (modo desarrollo).
  - `delete`: lista opcional de archivos/carpetas viejas a eliminar al instalar.
- El updater se despliega junto a la app en una carpeta `ggupdater/`:

  ```text
  MiApp/
  ├── MiApp.x86_64  (o MiApp.exe)
  └── ggupdater/
      └── ggupdater.x86_64  (o ggupdater.exe)
  ```

  **Solo se copia el ejecutable**: no hay `.pck`, assets ni carpetas adicionales. Logs, caché y
  temporales viven en los datos de usuario (`~/.local/share/ggupdater` o `%APPDATA%\ggupdater`).

- GGUpdater considera `../` (la carpeta que contiene `ggupdater/`) como raíz de la app: instala ahí
  y **nunca** toca la carpeta `ggupdater/`.

## Objetivo

1. Consultar la versión remota en el manifest.
2. Si `version_number` remoto **es mayor** que el local, mostrar un popup de actualización.
3. Solo si el usuario pulsa **"Actualizar"**, lanzar GGUpdater y cerrar la app.

La decisión de mostrar el popup es de la app; el updater vuelve a comparar `version_number` antes de
instalar y bloquea mismos números y downgrades.

## Lanzamiento

Ejecuta el binario de `ggupdater/` con estos argumentos exactos:

```text
--app <APP_ID>
--executable <nombre del ejecutable de la app>
--current-version <versión local, ej. 1.0.0>
--current-version-number <entero local>
--pid <PID actual de la app>
```

- `--pid` permite que GGUpdater espere a que la app se cierre antes de sobrescribir archivos.
- Tras lanzarlo, la app debe cerrarse (`get_tree().quit()`).
- En Linux el binario puede requerir `chmod +x`; en el export suele conservarse.

## Flujo esperado en la app

1. Leer el archivo de versión local (`version` + `version_number`).
2. Descargar el manifest remoto (o usar caché/omisión si no hay red).
3. Comparar `version_number` remoto con el local.
4. Si es mayor, mostrar popup con versión actual → nueva y botones `Actualizar` / `Cancelar`.
5. Si el usuario acepta, validar que exista `ggupdater/ggupdater.x86_64` (o `ggupdater.exe`) y lanzarlo.
6. Cerrar la app.

## Estructura de archivos esperada en el export

```text
EXPORTS/MiApp/
├── MiApp.x86_64 / MiApp.exe
├── MiApp.pck
├── version.json
└── ggupdater/
    └── ggupdater.x86_64 / ggupdater.exe
```

## Buenas prácticas

- No hardcodear la URL del manifest fuera de una constante.
- No descargar ni instalar nada desde la app: todo lo hace GGUpdater.
- Validar que los argumentos no estén vacíos antes de lanzar.
- Registrar en consola/archivo por qué se decidió actualizar o no.
- No tocar la carpeta `ggupdater/` desde la app.

## Checklist final

- [ ] `APP_ID` coincide con `manifests/<app_id>.json`.
- [ ] `--app`, `--executable`, `--current-version`, `--current-version-number` y `--pid` se pasan bien.
- [ ] El popup solo aparece si el `version_number` remoto es mayor.
- [ ] La app se cierra después de lanzar GGUpdater.
- [ ] `ggupdater/` se copia al export con el ejecutable correcto por plataforma.
