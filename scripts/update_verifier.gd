class_name UpdateVerifier
extends RefCounted
## Verificación SHA-256 del ZIP descargado contra el manifest.
## Si el hash es placeholder/vacío permite saltar la validación (modo dev),
## pero la estructura queda lista para activarla sin cambios.

signal verified(path: String, skipped: bool)
signal failed(message: String)


## Devuelve true si la verificación pasó (o fue saltada). En error borra el archivo.
func verify(zip_path: String, expected_sha: String, delete_on_error: bool, logger: Node = null) -> bool:
	if not FileAccess.file_exists(zip_path):
		_fail(logger, "No existe el archivo a verificar: %s" % zip_path)
		return false

	if expected_sha.is_empty() or expected_sha == UpdateManifest.PLACEHOLDER:
		_log(logger, "warn", "SHA-256 PENDING; se omite validación (modo desarrollo).")
		verified.emit(zip_path, true)
		return true

	var actual := _sha256(zip_path)
	if actual.is_empty():
		_fail(logger, "No se pudo calcular el SHA-256 de: %s" % zip_path)
		return false

	if actual.to_lower() != expected_sha.to_lower():
		if delete_on_error and FileAccess.file_exists(zip_path):
			DirAccess.remove_absolute(zip_path)
		_fail(logger, "SHA-256 incorrecto. Esperado=%s Obtenido=%s (archivo eliminado)." % [expected_sha, actual])
		return false

	_log(logger, "info", "SHA-256 verificado correctamente.")
	verified.emit(zip_path, false)
	return true


## Calcula SHA-256 leyendo en bloques. Devuelve "" si falla.
func _sha256(path: String) -> String:
	var ctx := HashingContext.new()
	if ctx.start(HashingContext.HASH_SHA256) != OK:
		return ""
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		return ""
	const CHUNK := 1 << 20
	while not file.eof_reached():
		var chunk := file.get_buffer(CHUNK)
		if chunk.size() > 0:
			if ctx.update(chunk) != OK:
				file.close()
				return ""
	file.close()
	return ctx.finish().hex_encode()


func _fail(logger: Node, message: String) -> void:
	_log(logger, "error", message)
	failed.emit(message)


func _log(logger: Node, level: String, message: String) -> void:
	if logger != null and logger.has_method(level):
		logger.call(level, message)
