class_name DownloadManager
extends RefCounted
## Descarga HTTP con progreso real (bytes/total) hacia un archivo local.
## También sirve para descargar texto (manifests) a memoria.

signal progress(received_bytes: int, total_bytes: int)
signal completed(path: String)
signal failed(message: String)

const CHUNK_SIZE := 131072  # 128 KiB

var _http: HTTPClient = null
var _active: bool = false
var _downloaded: int = 0


## Descarga un archivo a un path absoluto. Devuelve true si arrancó.
func download_to_file(url: String, destination: String, logger: Node = null) -> bool:
	if _active:
		_log(logger, "error", "Ya hay una descarga en curso.")
		failed.emit("Ya hay una descarga en curso.")
		return false
	if url.is_empty() or url == UpdateManifest.PLACEHOLDER:
		failed.emit("URL de descarga no configurada.")
		return false

	var parsed := _parse_url(url)
	if parsed.is_empty():
		failed.emit("URL inválida: %s" % url)
		return false

	if not _ensure_dir(destination.get_base_dir()):
		failed.emit("No se pudo crear el directorio de destino: %s" % destination.get_base_dir())
		return false

	_http = HTTPClient.new()
	var err := _http.connect_to_host(parsed.host, parsed.port, _tls_options(parsed.tls))
	if err != OK:
		failed.emit("No se pudo conectar al host %s (error %d)." % [parsed.host, err])
		return false

	_downloaded = 0
	_active = true
	_run_loop(url, parsed, destination, logger)
	return true


## Descarga texto (manifest) a memoria. Devuelve el String o "" en error.
func download_text(url: String, logger: Node = null) -> String:
	if url.is_empty() or url == UpdateManifest.PLACEHOLDER:
		_log(logger, "warn", "URL de manifest vacía o placeholder.")
		return ""

	var parsed := _parse_url(url)
	if parsed.is_empty():
		_log(logger, "error", "URL de manifest inválida: %s" % url)
		return ""

	var http := HTTPClient.new()
	if http.connect_to_host(parsed.host, parsed.port, _tls_options(parsed.tls)) != OK:
		_log(logger, "error", "No se pudo conectar para manifest: %s" % parsed.host)
		return ""

	var body := PackedByteArray()
	var requested := false
	var response_code := 0
	var done := false
	var total := -1
	var deadline := Time.get_ticks_msec() + 30000
	var last_progress := Time.get_ticks_msec()

	while not done and Time.get_ticks_msec() < deadline:
		http.poll()
		match http.get_status():
			HTTPClient.STATUS_CONNECTED:
				if not requested:
					http.request(HTTPClient.METHOD_GET, parsed.path, PackedStringArray(["User-Agent: GGUpdater"]))
					requested = true
			HTTPClient.STATUS_BODY:
				if response_code == 0 and http.has_response():
					response_code = http.get_response_code()
					total = http.get_response_body_length()
				var chunk := http.read_response_body_chunk()
				if chunk.size() > 0:
					body.append_array(chunk)
					last_progress = Time.get_ticks_msec()
				if total >= 0 and body.size() >= total:
					done = true
			HTTPClient.STATUS_DISCONNECTED:
				if requested:
					if response_code == 0 and http.has_response():
						response_code = http.get_response_code()
					done = true
			HTTPClient.STATUS_CONNECTION_ERROR:
				# Algunos servidores cierran la conexión tras el body sin Content-Length.
				if requested and not body.is_empty():
					done = true
				else:
					_log(logger, "error", "Error de conexión descargando manifest: %s" % url)
					return ""
			_:
				pass
		if requested and not done and Time.get_ticks_msec() - last_progress > 5000:
			done = true
		OS.delay_msec(5)

	if not done:
		_log(logger, "error", "Timeout descargando manifest: %s" % url)
		return ""
	if response_code != 0 and response_code != 200:
		_log(logger, "error", "Manifest respondió HTTP %d." % response_code)
		return ""
	return body.get_string_from_utf8()


func _run_loop(url: String, parsed: Dictionary, destination: String, logger: Node) -> void:
	var file := FileAccess.open(destination, FileAccess.WRITE)
	if file == null:
		_active = false
		failed.emit("No se pudo abrir el archivo de destino: %s" % destination)
		return

	var requested := false
	var total := -1
	var response_code := 0
	var success := false
	var finished := false
	var last_progress := Time.get_ticks_msec()

	while _active and not finished:
		_http.poll()
		match _http.get_status():
			HTTPClient.STATUS_RESOLVING, HTTPClient.STATUS_CONNECTING:
				pass
			HTTPClient.STATUS_CONNECTED:
				if not requested:
					var headers := PackedStringArray(["User-Agent: GGUpdater", "Accept: */*"])
					_http.request(HTTPClient.METHOD_GET, parsed.path, headers)
					requested = true
			HTTPClient.STATUS_REQUESTING:
				pass
			HTTPClient.STATUS_BODY:
				if _http.has_response() and response_code == 0:
					response_code = _http.get_response_code()
					total = _http.get_response_body_length()
					if response_code >= 400:
						success = false
						finished = true
						last_progress = Time.get_ticks_msec()
						continue
				var chunk := _http.read_response_body_chunk()
				if chunk.size() > 0:
					file.store_buffer(chunk)
					_downloaded += chunk.size()
					last_progress = Time.get_ticks_msec()
					progress.emit(_downloaded, total)
				if total >= 0 and _downloaded >= total:
					success = true
					finished = true
			HTTPClient.STATUS_DISCONNECTED:
				if requested:
					if _http.has_response() and response_code == 0:
						response_code = _http.get_response_code()
					success = response_code == 0 or response_code == 200
					finished = true
			HTTPClient.STATUS_CONNECTION_ERROR:
				if requested and _downloaded > 0:
					success = true
					finished = true
				else:
					_log(logger, "error", "Error de conexión durante la descarga: %s" % url)
					finished = true
			_:
				pass
		if not finished and Time.get_ticks_msec() - last_progress > 15000:
			_log(logger, "error", "Timeout durante la descarga: %s" % url)
			finished = true
		OS.delay_msec(5)

	file.close()
	_active = false

	if response_code >= 400:
		_delete_file(destination)
		_log(logger, "error", "Descarga falló con HTTP %d: %s" % [response_code, url])
		failed.emit("El servidor respondió HTTP %d." % response_code)
		return
	if not success:
		_delete_file(destination)
		_log(logger, "error", "Descarga interrumpida: %s" % url)
		failed.emit("La descarga se interrumpió.")
		return

	_log(logger, "info", "Descargados %d bytes a %s" % [_downloaded, destination])
	completed.emit(destination)


func cancel() -> void:
	_active = false


func get_downloaded_bytes() -> int:
	return _downloaded


func _parse_url(url: String) -> Dictionary:
	var tls := url.begins_with("https://")
	if not (url.begins_with("http://") or tls):
		return {}
	var rest := url.substr(8 if tls else 7)
	var slash := rest.find("/")
	var host_port := rest if slash == -1 else rest.substr(0, slash)
	var path := "/" if slash == -1 else rest.substr(slash)
	var host := host_port
	var port := 443 if tls else 80
	if host_port.contains(":"):
		var parts := host_port.split(":", true, 1)
		host = parts[0]
		port = int(parts[1])
	return {"host": host, "port": port, "path": path, "tls": tls}


func _tls_options(tls: bool) -> TLSOptions:
	return TLSOptions.client() if tls else null


func _ensure_dir(path: String) -> bool:
	if path.is_empty():
		return true
	if DirAccess.dir_exists_absolute(path):
		return true
	return DirAccess.make_dir_recursive_absolute(path) == OK


func _delete_file(path: String) -> void:
	if FileAccess.file_exists(path):
		DirAccess.remove_absolute(path)


func _log(logger: Node, level: String, message: String) -> void:
	if logger != null and logger.has_method(level):
		logger.call(level, message)
