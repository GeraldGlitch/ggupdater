class_name BannerManager
extends Node
## Carrusel de banners.
## - Muestra primero un banner local precargado (assets/banners/GG.png) para
##   tener contenido incluso sin internet.
## - Luego descarga los banners del manifest del repo y los añade, omitiendo
##   el banner local (GG.png) porque ya está embebido.
## - Cachea las imágenes remotas; si no hay conexión usa la caché.

signal banners_ready(textures: Array)
signal banner_changed(texture: Texture2D)

## Configuración: manifest e imágenes de banners hospedados en el repo.
const BANNERS_MANIFEST_URL := "https://raw.githubusercontent.com/GeraldGlitch/ggupdater/main/banners/banners.json"
const BANNERS_BASE_URL := "https://raw.githubusercontent.com/GeraldGlitch/ggupdater/main/banners/"

## Banner local embebido en el ejecutable. Se muestra siempre primero y no se
## descarga del repo (se omite si aparece en el manifest).
const LOCAL_BANNERS_DIR := "res://assets/banners/"
const LOCAL_BANNER_FILE := "GG.png"

const CACHE_DIR := "user://ggupdater/cache/banners/"
const ROTATION_INTERVAL := 6.0

const SUPPORTED_EXTENSIONS := ["webp", "png", "jpg", "jpeg"]

var _app_id: String = ""
var _textures: Array[Texture2D] = []
var _index: int = 0
var _timer: Timer = null
var _logger: Node = null


func setup(app_id: String, logger: Node) -> void:
	_app_id = app_id
	_logger = logger
	_ensure_dir(CACHE_DIR)

	_timer = Timer.new()
	_timer.wait_time = ROTATION_INTERVAL
	_timer.autostart = false
	_timer.timeout.connect(_advance)
	add_child(_timer)


## Carga los banners: primero el local precargado, luego los remotos.
## Siempre termina emitiendo banners_ready (posiblemente solo con el local).
func load_banners() -> void:
	_textures.clear()

	# 1) Banner local precargado (siempre disponible, sin internet).
	var local_tex := _load_local_banner()
	if local_tex != null:
		_textures.append(local_tex)
		_log("info", "Banner local precargado: %s" % LOCAL_BANNER_FILE)

	# 2) Banners remotos del manifest.
	var manifest_url := BANNERS_MANIFEST_URL
	if manifest_url.is_empty() or manifest_url == UpdateManifest.PLACEHOLDER:
		_log("warn", "BANNERS_MANIFEST_URL no configurada; usando caché si existe.")
		_append_from_cache()
		_finish()
		return

	var downloader := DownloadManager.new()
	add_child(downloader)
	var text: String = await downloader.download_text(manifest_url, _logger)
	downloader.queue_free()
	var entries := _parse_manifest(text)
	if entries.is_empty():
		_log("warn", "Manifest de banners vacío o inaccesible; usando caché.")
		_append_from_cache()
		_finish()
		return

	_ordered_by_app(entries)
	var loaded := 0
	for entry in entries:
		if _is_local_entry(entry):
			_log("info", "Se omite banner local del manifest: %s" % String(entry.get("url", "")))
			continue

		var url := _resolve_entry_url(entry)
		if url.is_empty():
			continue
		var local := _cache_path_for(entry, url)
		if not FileAccess.file_exists(local):
			var dl := DownloadManager.new()
			add_child(dl)
			if not dl.download_to_file(url, local, _logger):
				dl.queue_free()
				continue
			# Espera a que termine esta imagen antes de la siguiente.
			await dl.completed
			dl.queue_free()

		var tex := _load_texture(local)
		if tex != null:
			_textures.append(tex)
			loaded += 1

	_log("info", "Carrusel listo: 1 local + %d remotos." % loaded)
	_finish()


## Carga el banner local precargado como recurso importado (funciona en build).
func _load_local_banner() -> Texture2D:
	var resource_path := LOCAL_BANNERS_DIR + LOCAL_BANNER_FILE
	var tex := load(resource_path) as Texture2D
	if tex != null:
		return tex
	# Fallback: decodificar el buffer si el recurso no está importado.
	var abs_path := ProjectSettings.globalize_path(resource_path)
	if FileAccess.file_exists(abs_path):
		return _load_texture(abs_path)
	_log("warn", "No se encontró el banner local: %s" % resource_path)
	return null


## True si la entrada del manifest corresponde al banner local (no descargar).
func _is_local_entry(entry: Dictionary) -> bool:
	var raw := String(entry.get("url", "")).replace("\\", "/").get_file()
	return raw.to_lower() == LOCAL_BANNER_FILE.to_lower()


## Añade banners desde la caché sin borrar los ya cargados (p. ej. el local).
func _append_from_cache() -> void:
	var dir := DirAccess.open(CACHE_DIR)
	if dir == null:
		return
	var added := 0
	dir.list_dir_begin()
	var name := dir.get_next()
	while name != "":
		if not dir.current_is_dir() and _is_supported(name):
			var tex := _load_texture(CACHE_DIR + name)
			if tex != null:
				_textures.append(tex)
				added += 1
		name = dir.get_next()
	dir.list_dir_end()
	if added == 0:
		_log("warn", "Sin banners en caché.")
	else:
		_log("info", "Añadidos %d banners desde caché." % added)


func _finish() -> void:
	_log("info", "Carrusel con %d banners." % _textures.size())
	banners_ready.emit(_textures)
	if not _textures.is_empty():
		_start_rotation()


func _start_rotation() -> void:
	if _textures.size() > 1 and _timer != null:
		_timer.start()



func _advance() -> void:
	_index = (_index + 1) % _textures.size()
	banner_changed.emit(_textures[_index])


func stop() -> void:
	if _timer != null:
		_timer.stop()


func get_textures() -> Array:
	return _textures


func _ordered_by_app(entries: Array) -> void:
	entries.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		var a_match := String(a.get("app_id", "")) == _app_id
		var b_match := String(b.get("app_id", "")) == _app_id
		return a_match and not b_match
	)


func _parse_manifest(text: String) -> Array:
	if text.is_empty():
		return []
	var parsed: Variant = JSON.parse_string(text)
	if parsed == null:
		return []
	if parsed is Array:
		return parsed
	if parsed is Dictionary:
		var list: Variant = parsed.get("banners", [])
		if list is Array:
			return list
	return []


## Resuelve la URL del banner: si el manifest trae una ruta relativa (ej.
## "promo.png"), la une a BANNERS_BASE_URL. Si es absoluta, se usa tal cual.
func _resolve_entry_url(entry: Dictionary) -> String:
	var raw := String(entry.get("url", "")).strip_edges()
	if raw.is_empty():
		return ""
	if raw.begins_with("http://") or raw.begins_with("https://"):
		return raw
	return BANNERS_BASE_URL + raw.trim_prefix("/")


func _cache_path_for(entry: Dictionary, url: String) -> String:
	var file_name := url.get_file()
	if file_name.is_empty():
		file_name = "banner_%d" % abs(url.hash())
	var app_part := String(entry.get("app_id", "generic"))
	if app_part.is_empty():
		app_part = "generic"
	return CACHE_DIR + ("%s_%s" % [app_part, file_name]).replace(" ", "_")


func _load_texture(path: String) -> Texture2D:
	var image := Image.new()
	var extension := path.get_extension().to_lower()
	var err := ERR_FILE_UNRECOGNIZED
	match extension:
		"png":
			err = image.load_png_from_buffer(FileAccess.get_file_as_bytes(path))
		"jpg", "jpeg":
			err = image.load_jpg_from_buffer(FileAccess.get_file_as_bytes(path))
		"webp":
			err = image.load_webp_from_buffer(FileAccess.get_file_as_bytes(path))
		_:
			err = image.load(path)
	if err != OK:
		_log("warn", "No se pudo decodificar banner: %s" % path)
		return null
	return ImageTexture.create_from_image(image)


func _is_supported(file_name: String) -> bool:
	return file_name.get_extension().to_lower() in SUPPORTED_EXTENSIONS


func _ensure_dir(path: String) -> void:
	if not DirAccess.dir_exists_absolute(path):
		DirAccess.make_dir_recursive_absolute(path)


func _log(level: String, message: String) -> void:
	if _logger != null and _logger.has_method(level):
		_logger.call(level, message)
