# Flujo de UI

- Inicio: valida argumentos o muestra modo preview.
- Actualización: carga banners en paralelo, carga manifest, descarga, verifica, extrae e instala.
- Final: muestra éxito (`OK` relanza la app) o error (`Cerrar`).

# Mapa de código

- `src/main.rs`: arranque, canal de eventos y ventana eframe.
- `src/updater.rs`: máquina de estados en hilo aparte.
- `src/banners.rs`: carrusel local + remoto con caché.
- `src/ui.rs`: presentación egui (colores, progreso, botones, indicadores).
- `src/download.rs`, `src/verify.rs`, `src/zip_extract.rs`, `src/install.rs`:
  descarga HTTP, SHA-256, extracción segura e instalación en `../`.
- `src/args.rs`, `src/config.rs`, `src/manifest.rs`, `src/path_utils.rs`, `src/process_wait.rs`,
  `src/relaunch.rs`, `src/logger.rs`, `src/assets_image.rs`: soporte.
