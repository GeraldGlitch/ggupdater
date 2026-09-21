class_name DownloadManager
extends Node
## Descarga HTTP con progreso real usando HTTPRequest (API nativa recomendada).
## Maneja SSL, redirecciones y timeout. Sirve para archivos y para texto.

signal progress(received_bytes: int, total_bytes: int)
signal completed(path: String)
signal failed(message: String)

const MAX_REDIRECTS := 5
const TIMEOUT := 30.0
const USER_AGENT := "GGUpdater"

var _http: HTTPRequest = null
var _target_path: String = ""
var _text_mode: bool = false
var _text_result: String = ""
var _logger: Node = null
var _active: bool = false


## Crea el nodo HTTPRequest si no existe y conecta sus señales.
func _ensure_http() -> void:
	if _http != null and is_instance_valid(_http):
		return
	_http = HTTPRequest.new()
	_http.max_redirects = MAX_REDIRECTS
	_http.timeout = TIMEOUT
	add_child(_http)
	_http.request_completed.connect(_on_completed)


## Descarga un archivo a un path absoluto. Devuelve true si arrancó.
## Al terminar emite `completed` o `failed`.
func download_to_file(url: String, destination: String, logger: Node = null) -> bool:
	if _active:
		failed.emit("Ya hay una descarga en curso.")
		return false
	if url.is_empty() or url == UpdateManifest.PLACEHOLDER:
		failed.emit("URL de descarga no configurada.")
		return false
	if not _ensure_dir(destination.get_base_dir()):
		failed.emit("No se pudo crear el directorio de destino: %s" % destination.get_base_dir())
		return false

	_logger = logger
	_target_path = destination
	_text_mode = false
	_ensure_http()
	var headers := PackedStringArray(["User-Agent: %s" % USER_AGENT, "Accept: */*"])
	var err := _http.request(url, headers)
	if err != OK:
		failed.emit("No se pudo iniciar la descarga (error %d)." % err)
		return false
	_active = true
	return true


## Descarga texto (manifest) de forma asíncrona. Debe usarse con `await`.
## Devuelve el texto o "" si falla.
func download_text(url: String, logger: Node = null) -> String:
	if _active:
		_log(logger, "warn", "Ya hay una descarga en curso.")
		return ""
	if url.is_empty() or url == UpdateManifest.PLACEHOLDER:
		_log(logger, "warn", "URL de manifest vacía o placeholder.")
		return ""

	_logger = logger
	_text_mode = true
	_target_path = ""
	_text_result = ""
	_ensure_http()
	var headers := PackedStringArray(["User-Agent: %s" % USER_AGENT, "Accept: application/json, text/plain, */*"])
	var err := _http.request(url, headers)
	if err != OK:
		_log(logger, "error", "No se pudo iniciar la descarga de texto (error %d)." % err)
		return ""
	_active = true
	# Espera sin bloquear el hilo: el callback de HTTPRequest necesita el bucle activo.
	await _http.request_completed
	return _text_result


func _on_completed(result: int, response_code: int, _headers: PackedStringArray, body: PackedByteArray) -> void:
	_active = false

	if result != HTTPRequest.RESULT_SUCCESS:
		var reason := _result_name(result)
		if _text_mode:
			_log(_logger, "error", "Manifest no se pudo descargar: %s." % reason)
		else:
			_delete_file(_target_path)
			_log(_logger, "error", "Descarga falló: %s." % reason)
			failed.emit("Descarga falló: %s." % reason)
		return

	if response_code >= 400:
		if _text_mode:
			_log(_logger, "error", "Manifest respondió HTTP %d." % response_code)
		else:
			_delete_file(_target_path)
			_log(_logger, "error", "Descarga falló con HTTP %d." % response_code)
			failed.emit("El servidor respondió HTTP %d." % response_code)
		return

	if _text_mode:
		_text_result = body.get_string_from_utf8()
		_log(_logger, "info", "Manifest descargado (%d bytes)." % body.size())
		completed.emit("")
		return

	var file := FileAccess.open(_target_path, FileAccess.WRITE)
	if file == null:
		failed.emit("No se pudo escribir el archivo: %s" % _target_path)
		return
	file.store_buffer(body)
	file.close()
	progress.emit(body.size(), body.size())
	_log(_logger, "info", "Descargados %d bytes a %s" % [body.size(), _target_path])
	completed.emit(_target_path)


## Cancela la descarga en curso.
func cancel() -> void:
	if _http != null and is_instance_valid(_http) and _active:
		_http.cancel_request()
	_active = false


func _result_name(result: int) -> String:
	match result:
		HTTPRequest.RESULT_CANT_CONNECT: return "no se pudo conectar"
		HTTPRequest.RESULT_CANT_RESOLVE: return "no se pudo resolver el host"
		HTTPRequest.RESULT_CONNECTION_ERROR: return "error de conexión"
		HTTPRequest.RESULT_TLS_HANDSHAKE_ERROR: return "error TLS"
		HTTPRequest.RESULT_NO_RESPONSE: return "sin respuesta"
		HTTPRequest.RESULT_TIMEOUT: return "timeout"
		_: return "resultado %d" % result


func _ensure_dir(path: String) -> bool:
	if path.is_empty():
		return true
	if DirAccess.dir_exists_absolute(path):
		return true
	return DirAccess.make_dir_recursive_absolute(path) == OK


func _delete_file(path: String) -> void:
	if not path.is_empty() and FileAccess.file_exists(path):
		DirAccess.remove_absolute(path)


func _log(logger: Node, level: String, message: String) -> void:
	if logger != null and logger.has_method(level):
		logger.call(level, message)
