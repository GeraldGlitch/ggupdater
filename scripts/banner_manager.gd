class_name BannerManager
extends Node
## Carrusel de banners remotos con caché local.
## - Descarga un manifest de banners (lista de {app_id, url, sha256?}).
## - Descarga imágenes remotas y las guarda en caché.
## - Ordena: primero el banner cuyo app_id coincide, luego el resto.
## - Si no hay conexión usa la caché; si tampoco hay, mantiene el logo local.

signal banners_ready(textures: Array)
signal banner_changed(texture: Texture2D)

## Configuración placeholder: se completará cuando existan URLs reales.
const BANNERS_MANIFEST_URL := ""
const CACHE_DIR := "user://ggupdater/cache/banners/"
const LOCAL_BANNERS_DIR := "res://assets/banners/"
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


## Intenta cargar banners. Siempre termina emitiendo banners_ready (posiblemente vacío).
func load_banners() -> void:
	var manifest_url := BANNERS_MANIFEST_URL
	if manifest_url.is_empty() or manifest_url == UpdateManifest.PLACEHOLDER:
		_log("warn", "BANNERS_MANIFEST_URL no configurada; se usará caché si existe.")
		_emit_from_cache()
		return

	var downloader := DownloadManager.new()
	var text := downloader.download_text(manifest_url, _logger)
	var entries := _parse_manifest(text)
	if entries.is_empty():
		_log("warn", "Manifest de banners vacío o inválido; usando caché.")
		_emit_from_cache()
		return

	_ordered_by_app(entries)
	for entry in entries:
		var local := _cache_path_for(entry)
		if not FileAccess.file_exists(local):
			var dl := DownloadManager.new()
			var ok := dl.download_to_file(String(entry.get("url", "")), local, _logger)
			if not ok:
				continue

		var tex := _load_texture(local)
		if tex != null:
			_textures.append(tex)

	if _textures.is_empty():
		_emit_from_cache()
		return

	_log("info", "Carrusel cargado con %d banners." % _textures.size())
	banners_ready.emit(_textures)
	_start_rotation()


## Carga banners desde la caché (offline / sin manifest). Si la caché está
## vacía, intenta banners locales en res://assets/banners/ (útil en preview).
func _emit_from_cache() -> void:
	_textures.clear()
	var dir := DirAccess.open(CACHE_DIR)
	if dir != null:
		dir.list_dir_begin()
		var name := dir.get_next()
		while name != "":
			if not dir.current_is_dir() and _is_supported(name):
				var tex := _load_texture(CACHE_DIR + name)
				if tex != null:
					_textures.append(tex)
			name = dir.get_next()
		dir.list_dir_end()

	if _textures.is_empty():
		_load_local_banners()

	if _textures.is_empty():
		_log("warn", "Sin caché ni banners locales; se mantiene el developer logo.")
	else:
		_log("info", "Cargados %d banners desde caché/local." % _textures.size())
	banners_ready.emit(_textures)
	if not _textures.is_empty():
		_start_rotation()


func _start_rotation() -> void:
	if _textures.size() > 1 and _timer != null:
		_timer.start()


## Carga imágenes locales de preview desde res://assets/banners/.
func _load_local_banners() -> void:
	if not DirAccess.dir_exists_absolute(LOCAL_BANNERS_DIR):
		return
	var dir := DirAccess.open(LOCAL_BANNERS_DIR)
	if dir == null:
		return
	dir.list_dir_begin()
	var name := dir.get_next()
	while name != "":
		if not dir.current_is_dir() and _is_supported(name):
			var tex := _load_texture(LOCAL_BANNERS_DIR + name)
			if tex != null:
				_textures.append(tex)
		name = dir.get_next()
	dir.list_dir_end()
	if not _textures.is_empty():
		_log("info", "Cargados %d banners locales de preview." % _textures.size())


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


func _cache_path_for(entry: Dictionary) -> String:
	var url := String(entry.get("url", ""))
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
