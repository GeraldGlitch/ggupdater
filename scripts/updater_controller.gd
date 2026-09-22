class_name UpdaterController
extends Node
## Máquina de estados principal del updater.
## Orquesta manifest, banners, descarga, verificación, extracción e instalación
## y notifica a la UI mediante la señal `state_changed`.

signal state_changed(state: int, data: Dictionary)
signal progress_changed(ratio: float, received_bytes: int, total_bytes: int)
signal versions_changed(current: String, target: String)

enum State {
	STARTING,
	LOADING_MANIFEST,
	LOADING_BANNERS,
	DOWNLOADING,
	VERIFYING,
	EXTRACTING,
	INSTALLING,
	COMPLETED,
	ERROR,
	PREVIEW,
}

const TEMP_DIR := "user://ggupdater/temp/"
const ZIP_PATH := "user://ggupdater/temp/update.zip"
const EXTRACT_DIR := "user://ggupdater/temp/extracted/"

var args: LaunchArgs
var logger: Node
var banner_manager: BannerManager

var _state: State = State.STARTING
var _manifest: UpdateManifest = null
var _download: DownloadManager = null
var _last_error: String = ""


func setup(launch_args: LaunchArgs, log_node: Node, banners: BannerManager) -> void:
	args = launch_args
	logger = log_node
	banner_manager = banners


func start() -> void:
	_cleanup_old_temp()
	_go(State.STARTING, {"message": "Starting updater..."})
	versions_changed.emit(args.current_version, "")

	if args.preview_mode:
		_start_preview()
		return

	if not args.valid:
		_error("Argumentos inválidos:\n" + "\n".join(args.errors))
		return

	_load_banners_then_manifest()


## Modo preview (sin --app): solo carga banners y deja la UI informativa.
func _start_preview() -> void:
	_log("info", "Modo preview: sin --app, no se actualizará ninguna aplicación.")
	_go(State.LOADING_BANNERS, {"message": "Cargando banners..."})
	if banner_manager != null:
		banner_manager.banners_ready.connect(_on_preview_banners_ready, CONNECT_ONE_SHOT)
		banner_manager.load_banners()
	else:
		_on_preview_banners_ready([])


func _on_preview_banners_ready(textures: Array) -> void:
	var message := "Modo preview (sin --app)"
	if textures.is_empty():
		message += "\nNo hay banners en caché. Configura BANNERS_MANIFEST_URL."
	else:
		message += "\n%d banners cargados." % textures.size()
	_go(State.PREVIEW, {"message": message})


## En modo real los banners se cargan en paralelo; el manifest no espera a que
## terminen. La UI se rellena con `banners_partial_ready` y recibe el carrusel
## completo en `banners_ready`.
func _load_banners_then_manifest() -> void:
	if banner_manager != null:
		banner_manager.load_banners()
	_proceed_manifest()


# ---------------------------------------------------------------- manifest ----

func _proceed_manifest() -> void:
	_go(State.LOADING_MANIFEST, {"message": "Cargando manifest..."})
	var manifest: UpdateManifest = await _fetch_manifest()
	if manifest == null:
		return

	if manifest.app_id != args.app_id:
		_error("El manifest pertenece a '%s' pero la app es '%s'." % [manifest.app_id, args.app_id])
		return

	_manifest = manifest
	versions_changed.emit(args.current_version, _manifest.version)

	if _manifest.is_url_pending() and args.local_update.is_empty():
		_error("El manifest no contiene 'download_url' válido. La instalación no se modificó.")
		return
	await _start_download()


## Devuelve un UpdateManifest o null (ya reportó error). En modo local crea un manifest sintético.
func _fetch_manifest() -> UpdateManifest:
	if not args.local_update.is_empty():
		# Modo local: no hay manifest remoto, se usa un stub con la versión destino desconocida.
		var stub := UpdateManifest.new()
		stub.app_id = args.app_id
		stub.version = args.current_version
		stub.version_number = args.current_version_number
		stub.download_url = UpdateManifest.PLACEHOLDER
		stub.sha256 = UpdateManifest.PLACEHOLDER
		stub.loaded = true
		_log("info", "Modo local: se omite manifest remoto.")
		return stub

	var url := ProjectConfig.manifest_url(args.app_id)

	var downloader := DownloadManager.new()
	add_child(downloader)
	var text: String = await downloader.download_text(url, logger)
	if text.is_empty():
		_error("Manifest inaccesible o sin conexión: %s" % url)
		return null

	var parsed: Variant = JSON.parse_string(text)
	if not (parsed is Dictionary):
		_error("El manifest no es un JSON válido.")
		return null

	var manifest := UpdateManifest.from_dictionary(parsed, args.platform)
	if not manifest.loaded:
		_error("Manifest inválido:\n" + "\n".join(manifest.errors))
		return null
	_log("info", "Manifest cargado: %s -> %s (%d)" % [manifest.app_id, manifest.version, manifest.version_number])
	return manifest


# ---------------------------------------------------------------- download ----

func _start_download() -> void:
	_ensure_dir(TEMP_DIR)
	_purge_dir(TEMP_DIR)
	_go(State.DOWNLOADING, {"message": "Esperando cierre de la aplicación..."})
	progress_changed.emit(0.0, 0, 0)

	# La app es quien decide si hay actualización; aquí solo se espera a que
	# cierre antes de descargar e instalar.
	var waiter := ProcessWaiter.new()
	await waiter.wait_for_exit(args.pid, logger)

	_go(State.DOWNLOADING, {"message": "Descargando actualización..."})

	if not args.local_update.is_empty():
		_use_local_zip()
		return

	_download = DownloadManager.new()
	add_child(_download)
	_download.progress.connect(_on_download_progress)
	_download.completed.connect(_on_download_completed)
	_download.failed.connect(func(msg: String) -> void: _error("Descarga fallida: " + msg))
	_download.download_to_file(_manifest.download_url, ZIP_PATH, logger)


func _use_local_zip() -> void:
	var local_path := args.local_update
	if not FileAccess.file_exists(local_path):
		_error("El ZIP local no existe: %s" % local_path)
		return
	# Globaliza rutas res:// (editor) o user://; deja intactas las absolutas.
	var source := ProjectSettings.globalize_path(local_path)
	var err := DirAccess.copy_absolute(source, ProjectSettings.globalize_path(ZIP_PATH))
	if err != OK:
		_error("No se pudo copiar el ZIP local (error %d)." % err)
		return
	_log("info", "ZIP local copiado a temporales: %s" % local_path)
	_on_download_completed(ZIP_PATH)


func _on_download_progress(received: int, total: int) -> void:
	var ratio := 0.0
	if total > 0:
		ratio = clampf(float(received) / float(total), 0.0, 1.0)
	progress_changed.emit(ratio, received, total)


func _on_download_completed(_path: String) -> void:
	_verify()


# ----------------------------------------------------------------- verify ----

func _verify() -> void:
	_go(State.VERIFYING, {"message": "Verificando integridad..."})
	progress_changed.emit(1.0, 0, 0)
	var expected := UpdateManifest.PLACEHOLDER
	if args.local_update.is_empty() and _manifest != null:
		expected = _manifest.sha256
		if _manifest.is_sha_pending():
			_log("warn", "Sin sha256 para '%s'; se omite la verificación de integridad." % args.platform)
	else:
		_log("info", "Modo local: se omite la verificación de integridad.")

	var verifier := UpdateVerifier.new()
	if not verifier.verify(ZIP_PATH, expected, true, logger):
		_error("La verificación falló. La actualización se canceló.")
		return
	_extract()


# ---------------------------------------------------------------- extract ----

func _extract() -> void:
	_go(State.EXTRACTING, {"message": "Extrayendo archivos..."})
	if DirAccess.dir_exists_absolute(EXTRACT_DIR):
		_remove_tree(EXTRACT_DIR)
	_ensure_dir(EXTRACT_DIR)

	var zip := ZipManager.new()
	zip.progress.connect(func(idx: int, total: int) -> void:
		if total > 0:
			progress_changed.emit(float(idx) / float(total), idx, total)
	)
	zip.failed.connect(func(msg: String) -> void: _error("Error de extracción: " + msg))
	if not zip.extract(ZIP_PATH, EXTRACT_DIR, logger):
		return
	_install()


# ---------------------------------------------------------------- install ----

func _install() -> void:
	_go(State.INSTALLING, {"message": "Instalando actualización..."})
	progress_changed.emit(0.0, 0, 0)

	var installer := InstallManager.new()
	installer.progress.connect(func(_step: String, current: int, total: int) -> void:
		if total > 0:
			progress_changed.emit(float(current) / float(total), current, total)
	)
	installer.failed.connect(func(msg: String) -> void: _error("Error de instalación: " + msg))

	var delete_list := _manifest.delete_list if _manifest != null else PackedStringArray()
	if not installer.install(EXTRACT_DIR, args.target_root, delete_list, logger):
		return

	_cleanup_after_success()
	var target_version := _manifest.version if _manifest != null else args.current_version
	_go(State.COMPLETED, {
		"message": "Actualización completada",
		"current": args.current_version,
		"target": target_version,
	})


# ---------------------------------------------------------------- estados ----

func _go(state: State, data: Dictionary) -> void:
	_state = state
	_log("info", "Estado -> %s %s" % [_state_name(state), JSON.stringify(data)])
	state_changed.emit(state, data)


func _error(message: String) -> void:
	_last_error = message
	_log("error", message)
	_go(State.ERROR, {"message": message})


func get_last_error() -> String:
	return _last_error


func get_state() -> State:
	return _state


func _state_name(state: State) -> String:
	return State.keys()[state]


# ---------------------------------------------------------------- limpieza ----

func _cleanup_after_success() -> void:
	_delete_file(ProjectSettings.globalize_path(ZIP_PATH))
	_remove_tree(ProjectSettings.globalize_path(EXTRACT_DIR))
	_log("info", "Temporales limpiados.")


func _cleanup_old_temp() -> void:
	if DirAccess.dir_exists_absolute(TEMP_DIR):
		_remove_tree(ProjectSettings.globalize_path(TEMP_DIR))
	_ensure_dir(TEMP_DIR)
	_log("info", "Temporales antiguos limpiados al iniciar.")


# ---------------------------------------------------------------- helpers ----

func _ensure_dir(path: String) -> void:
	var absolute := ProjectSettings.globalize_path(path)
	if not DirAccess.dir_exists_absolute(absolute):
		DirAccess.make_dir_recursive_absolute(absolute)


func _delete_file(absolute: String) -> void:
	if FileAccess.file_exists(absolute):
		DirAccess.remove_absolute(absolute)


func _purge_dir(path: String) -> void:
	var absolute := ProjectSettings.globalize_path(path)
	if DirAccess.dir_exists_absolute(absolute):
		_remove_tree(absolute)
	_ensure_dir(path)


func _remove_tree(absolute: String) -> void:
	var dir := DirAccess.open(absolute)
	if dir == null:
		return
	dir.list_dir_begin()
	var name := dir.get_next()
	while name != "":
		if name != "." and name != "..":
			var child := absolute.path_join(name)
			if dir.current_is_dir():
				_remove_tree(child)
			else:
				DirAccess.remove_absolute(child)
		name = dir.get_next()
	dir.list_dir_end()
	DirAccess.remove_absolute(absolute)


func _log(level: String, message: String) -> void:
	if logger != null and logger.has_method(level):
		logger.call(level, message)
