# Contexto del Proyecto

- Stack: Godot 4.7 y GDScript.
- Función: updater genérico para aplicaciones desktop Windows y Linux.
- Configuración: cada build define `ProjectConfig.APP_ID`; los manifests remotos viven en `manifests/<app_id>.json`.
- Versiones: `version` se usa para UI y `version_number` entero monótono decide actualizaciones.
