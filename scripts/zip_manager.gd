class_name ZipManager
extends RefCounted
## Extracción segura de ZIP usando ZIPReader.
## Nunca escribe fuera del directorio de extracción y aplica anti path-traversal.

signal progress(entry_index: int, entry_total: int)
signal completed(extracted_root: String)
signal failed(message: String)


## Extrae zip_path a output_dir. Devuelve true si arrancó sin errores.
func extract(zip_path: String, output_dir: String, logger: Node = null) -> bool:
	if not FileAccess.file_exists(zip_path):
		_fail(logger, "El archivo ZIP no existe: %s" % zip_path)
		return false

	var reader := ZIPReader.new()
	if reader.open(zip_path) != OK:
		_fail(logger, "No se pudo abrir el ZIP (archivo inválido o corrupto).")
		return false

	var files := reader.get_files()
	var total := files.size()
	if total == 0:
		reader.close()
		_fail(logger, "El ZIP no contiene archivos.")
		return false

	if not _ensure_dir(output_dir):
		reader.close()
		_fail(logger, "No se pudo crear el directorio de extracción: %s" % output_dir)
		return false

	var output_abs := output_dir.simplify_path()
	var extracted := 0

	for i in total:
		var entry := files[i]
		var relative := PathUtils.sanitize_zip_path(entry)

		if relative.is_empty():
			continue
		# ZIPReader usa "/" final para directorios.
		var is_dir := relative.ends_with("/")
		relative = relative.trim_suffix("/")

		if not PathUtils.is_safe_relative(relative):
			reader.close()
			_fail(logger, "Entrada ZIP insegura bloqueada: %s" % entry)
			return false

		var destination := PathUtils.resolve_under(output_abs, relative)
		if not PathUtils.is_inside(output_abs, destination):
			reader.close()
			_fail(logger, "Entrada ZIP escapó del directorio destino: %s" % entry)
			return false

		if is_dir:
			_ensure_dir(destination)
			continue

		if not _ensure_dir(destination.get_base_dir()):
			reader.close()
			_fail(logger, "No se pudo crear directorio para: %s" % relative)
			return false

		var data := reader.read_file(entry)
		var file := FileAccess.open(destination, FileAccess.WRITE)
		if file == null:
			reader.close()
			_fail(logger, "No se pudo escribir el archivo extraído: %s" % relative)
			return false
		file.store_buffer(data)
		file.close()

		extracted += 1
		progress.emit(i + 1, total)

	reader.close()
	_log(logger, "info", "Extraídos %d archivos a %s" % [extracted, output_abs])
	completed.emit(output_abs)
	return true


func _fail(logger: Node, message: String) -> void:
	_log(logger, "error", message)
	failed.emit(message)


func _ensure_dir(path: String) -> bool:
	if path.is_empty():
		return true
	if DirAccess.dir_exists_absolute(path):
		return true
	return DirAccess.make_dir_recursive_absolute(path) == OK


func _log(logger: Node, level: String, message: String) -> void:
	if logger != null and logger.has_method(level):
		logger.call(level, message)
