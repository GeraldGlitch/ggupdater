# Contexto del Proyecto

- Stack: Godot 4.7 y GDScript.
- Función: updater genérico para aplicaciones desktop Windows y Linux.
- Configuración: una única build genérica; el manifest `manifests/<app_id>.json` se resuelve con el argumento `--app`. `ProjectConfig` solo define el repositorio central (owner/repo/branch).
- Manifest: cada manifest expone `app_id` y `download_url` (enlace directo al asset `.zip`, string universal u objeto `windows`/`linux`) y opcionalmente `version`, `version_number`, `sha256` y `delete`. GGUpdater elige la entrada según el SO.
- Decisión de actualización: la toma la app antes de lanzar GGUpdater. GGUpdater no compara versiones ni construye URLs.
