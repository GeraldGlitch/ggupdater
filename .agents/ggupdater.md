Trabaja dentro de esta carpeta `GGUpdater/` y construye un proyecto Godot 4 funcional que actúe como updater genérico reutilizable para varias aplicaciones desktop.

## Objetivo

GGUpdater debe ser completamente independiente de las apps que actualizará. Más adelante cada app lo ejecutará desde una carpeta:

```text
App/
├── App.exe / App.x86_64
├── ...
└── GGUpdater/
    ├── GGUpdater.exe / GGUpdater.x86_64
    └── GGUpdater.pck
```

Por lo tanto, GGUpdater debe considerar `../` como la carpeta raíz de la aplicación a actualizar.

Debe funcionar tanto en Windows como Linux.

## Argumentos

Preparar soporte para recibir argumentos similares a:

```text
--app blackcatpos
--executable BlackCatPOS.exe
--version 1.0.0
--pid 1234
```

`--pid` puede ser opcional.

Guardar internamente:

* `app_id`
* `executable`
* `current_version`
* `target_root = ../`
* plataforma actual

Validar argumentos y mostrar error claro si faltan datos esenciales.

## Flujo de estados

Implementar una máquina de estados:

```text
STARTING
LOADING_MANIFEST
LOADING_BANNERS
DOWNLOADING
VERIFYING
EXTRACTING
INSTALLING
COMPLETED
ERROR
```

La UI debe reaccionar al estado actual.

## UI

Crear una ventana sencilla de updater.

Al iniciar mostrar:

```text
[Developer Logo]

Starting updater...
```

El developer logo debe estar incluido localmente dentro del proyecto para que siempre pueda mostrarse sin internet.

Luego mostrar:

* área grande para carrusel de banners
* texto indicando estado actual
* barra de progreso
* porcentaje
* versión actual → nueva versión cuando esté disponible
* botón `OK` solamente al terminar
* mensaje de error + botón para cerrar/reintentar cuando corresponda

No sobrecargar visualmente la interfaz.

## Carrusel

Preparar sistema de carrusel remoto.

Las URLs todavía NO existen, así que usar constantes/configuración placeholder:

```gdscript
const MANIFEST_BASE_URL = ""
const BANNERS_MANIFEST_URL = ""
```

El sistema debe quedar preparado para:

1. Descargar manifest de banners.
2. Descargar imágenes remotas.
3. Guardarlas en caché.
4. Mostrar primero el banner cuyo `app_id` coincida con la app actual.
5. Mostrar después los demás banners.
6. Cambiar automáticamente de imagen cada varios segundos.

Guardar caché en:

```text
user://ggupdater/cache/
```

Si no hay conexión:

* usar imágenes en caché
* si tampoco hay caché, mantener el developer logo

Soportar WebP/PNG/JPG.

## Manifest de actualización

Preparar una estructura esperada como:

```json
{
  "app_id": "blackcatpos",
  "version": "1.5.0",
  "windows": {
    "url": "PENDING",
    "sha256": "PENDING"
  },
  "linux": {
    "url": "PENDING",
    "sha256": "PENDING"
  },
  "delete": []
}
```

GGUpdater debe seleccionar automáticamente `windows` o `linux`.

No conectar todavía URLs reales.

## Descarga

Implementar descarga HTTP del ZIP mostrando progreso real.

Guardar temporalmente en:

```text
user://ggupdater/temp/update.zip
```

La barra de progreso debe representar bytes descargados / tamaño total cuando el servidor lo permita.

## Verificación

Después de descargar:

1. calcular SHA-256
2. comparar con el manifest
3. si no coincide, cancelar instalación
4. borrar archivo corrupto
5. mostrar error

Si el hash todavía está vacío o marcado como `PENDING`, permitir temporalmente saltar la validación para facilitar desarrollo, pero estructurar el código para activarla posteriormente sin cambios grandes.

## Extracción

Usar `ZIPReader`.

Nunca extraer directamente sobre `../`.

Primero extraer en:

```text
user://ggupdater/temp/extracted/
```

Manejar correctamente directorios y archivos anidados.

## Instalación

Después de extraer correctamente, copiar los archivos desde `extracted/` hacia:

```text
../
```

Debe:

* crear carpetas nuevas
* sobrescribir archivos existentes
* copiar archivos nuevos
* conservar estructura de carpetas

IMPORTANTE:

Nunca reemplazar:

```text
GGUpdater/
```

Cualquier archivo cuyo path relativo empiece por:

```text
GGUpdater/
```

debe ignorarse durante la instalación.

Preparar además una lista configurable de exclusiones, por ejemplo:

```gdscript
const EXCLUDED_PATHS = [
    "GGUpdater/"
]
```

Más adelante cada app podrá agregar otras carpetas protegidas.

## Archivos eliminados

Soportar el array:

```json
"delete": []
```

Si contiene paths, eliminar esos archivos/carpetas de `../` durante la actualización.

Nunca permitir que esta lista elimine:

* `GGUpdater/`
* paths fuera de `../`

Sanitizar paths para impedir `../`, paths absolutos o traversal.

## Seguridad de paths

Toda operación de copiar, borrar o reemplazar debe verificar que el destino final permanezca dentro de la carpeta raíz de la aplicación.

No confiar directamente en paths provenientes del ZIP o JSON.

Bloquear path traversal como:

```text
../../
../
C:\
/
```

## Esperar cierre de la app

Preparar soporte para `--pid`.

Si se proporciona PID:

* esperar a que ese proceso termine antes de instalar

Si no se proporciona:

* esperar un pequeño intervalo y continuar

No intentar sobrescribir ejecutables mientras la app original siga abierta.

## Linux

Después de actualizar, asegurar que el ejecutable principal tenga permiso de ejecución.

Usar el mecanismo apropiado disponible desde Godot/OS para restaurar `chmod +x` o conservar permisos de forma segura.

No romper Windows al implementar esta parte.

## Limpieza

Después de una actualización exitosa borrar:

```text
user://ggupdater/temp/update.zip
user://ggupdater/temp/extracted/
```

No borrar:

```text
user://ggupdater/cache/
```

Limpiar también temporales viejos al iniciar.

## Finalización

Al completar mostrar:

```text
✓ Actualización completada

1.4.0 → 1.5.0

[ OK ]
```

Al presionar `OK`:

1. ejecutar el archivo recibido mediante `--executable` ubicado en `../`
2. cerrar GGUpdater

No relanzar la app hasta que el usuario pulse `OK`.

## Errores

Manejar como mínimo:

* manifest inaccesible
* sin internet
* download fallido
* ZIP inválido
* SHA-256 incorrecto
* error de extracción
* permisos insuficientes
* archivo bloqueado
* ejecutable destino inexistente
* argumentos inválidos

Los errores deben mostrarse en la UI y también registrarse.

## Logging

Crear logging sencillo en:

```text
user://ggupdater/logs/
```

Registrar:

* fecha/hora
* app_id
* versión actual
* versión destino
* plataforma
* pasos realizados
* archivos relevantes
* errores

No guardar secretos ni contenido sensible.

## Arquitectura

Separar responsabilidades en scripts/clases, evitando meter toda la lógica en un solo script.

Una estructura sugerida:

```text
scripts/
├── updater_controller.gd
├── update_manifest.gd
├── download_manager.gd
├── zip_manager.gd
├── install_manager.gd
├── banner_manager.gd
├── path_utils.gd
└── logger.gd
```

Puedes modificar esta estructura si encuentras una organización mejor.

## Modo de desarrollo local

Implementar también un modo opcional:

```text
--local-update /ruta/update.zip
```

Este modo debe saltarse la descarga remota y usar un ZIP local.

Esto permitirá probar todo el proceso antes de crear el repo y las URLs.

El flujo local debe seguir siendo:

```text
GGUpdater
→ carga ZIP local
→ extrae
→ ignora GGUpdater/
→ instala en ../
→ limpia temporales
→ muestra Completed
→ OK
→ relanza app
```

## Restricciones

* Godot 4.
* No usar librerías externas salvo que sean estrictamente necesarias.
* Priorizar APIs nativas de Godot.
* Compatible con Windows y Linux.
* No implementar todavía URLs reales.
* No asumir ningún `app_id` específico.
* No hardcodear BlackCatPOS, FITO, ClassroomHUB, etc.
* GGUpdater debe ser totalmente genérico.
* No tocar archivos fuera de `../`.
* No actualizar GGUpdater a sí mismo por ahora.

Al finalizar, deja el proyecto ejecutable y documenta brevemente en un README:

* argumentos soportados
* estructura esperada
* cómo probar con `--local-update`
* dónde colocar URLs futuras
* cómo exportar GGUpdater para Windows y Linux.
