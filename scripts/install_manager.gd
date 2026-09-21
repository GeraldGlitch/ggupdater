class_name InstallManager
extends RefCounted
## Copia los archivos extraídos hacia el root de la app (../) y aplica la lista delete.
## Todas las operaciones verifican que el destino permanezca dentro del root.

signal progress(step: String, current: int, total: int)
signal completed(installed: int, deleted: int)
signal failed(message: String)

const EXCLUDED_PATHS := [
	"GGUpdater/",
	"ggupdater/",
]


## staged_root: carpeta extraída (user://.../extracted)
## target_root: absoluto, normalmente ../ globalizado
func install(staged_root: String, target_root: String, delete_list: PackedStringArray, logger: Node = null) -> bool:
	if not DirAccess.dir_exists_absolute(staged_root):
		_fail(logger, "No existe la carpeta extraída: %s" % staged_root)
		return false
	if not DirAccess.dir_exists_absolute(target_root):
		_fail(logger, "No existe la carpeta destino de la app: %s" % target_root)
		return false

	var target_abs := target_root.simplify_path()
	var staged_abs := staged_root.simplify_path()

	var installed := _install_directory(staged_abs, staged_abs, target_abs, logger)
	if installed < 0:
		return false

	var deleted := _apply_delete_list(delete_list, target_abs, logger)

	_log(logger, "info", "Instalados %d archivos, eliminados %d elementos." % [installed, deleted])
	completed.emit(installed, deleted)
	return true


## Copia recursivamente, respetando exclusiones. Devuelve -1 si falla.
func _install_directory(root_staged: String, current_dir: String, target_root: String, logger: Node) -> int:
	var count := 0
	var dir := DirAccess.open(current_dir)
	if dir == null:
		_fail(logger, "No se pudo abrir el directorio: %s" % current_dir)
		return -1

	dir.list_dir_begin()
	var name := dir.get_next()
	while name != "":
		if name == "." or name == "..":
			name = dir.get_next()
			continue

		var absolute := current_dir.path_join(name)
		var relative := absolute.substr(root_staged.length()).trim_prefix("/")

		if PathUtils.is_excluded(relative, EXCLUDED_PATHS):
			_log(logger, "info", "Excluido durante instalación: %s" % relative)
			name = dir.get_next()
			continue

		if dir.current_is_dir():
			var sub := _install_directory(root_staged, absolute, target_root, logger)
			if sub < 0:
				dir.list_dir_end()
				return -1
			count += sub
		else:
			if not PathUtils.is_safe_target(target_root, relative):
				_fail(logger, "Ruta de destino insegura bloqueada: %s" % relative)
				dir.list_dir_end()
				return -1
			var destination := PathUtils.resolve_under(target_root, relative)
			if not _copy_file(absolute, destination, logger):
				dir.list_dir_end()
				return -1
			count += 1
			progress.emit("copying", count, 0)

		name = dir.get_next()
	dir.list_dir_end()
	return count


func _copy_file(source: String, destination: String, logger: Node) -> bool:
	if not _ensure_dir(destination.get_base_dir()):
		_fail(logger, "No se pudo crear la carpeta para: %s" % destination)
		return false

	var data := FileAccess.get_file_as_bytes(source)
	if data.is_empty() and not FileAccess.file_exists(source):
		_fail(logger, "No se pudo leer: %s" % source)
		return false

	var file := FileAccess.open(destination, FileAccess.WRITE)
	if file == null:
		_fail(logger, "No se pudo escribir (¿permisos o archivo bloqueado?): %s" % destination)
		return false
	file.store_buffer(data)
	file.close()
	return true


func _apply_delete_list(delete_list: PackedStringArray, target_root: String, logger: Node) -> int:
	var deleted := 0
	for raw_path in delete_list:
		var relative := PathUtils.sanitize_zip_path(raw_path)
		if relative.is_empty():
			continue
		if PathUtils.is_excluded(relative, EXCLUDED_PATHS):
			_log(logger, "warn", "delete bloqueado por exclusión: %s" % relative)
			continue
		if not PathUtils.is_safe_target(target_root, relative):
			_log(logger, "warn", "delete bloqueado (ruta insegura): %s" % relative)
			continue

		var absolute := PathUtils.resolve_under(target_root, relative)
		if DirAccess.dir_exists_absolute(absolute):
			if _remove_tree(absolute):
				deleted += 1
				_log(logger, "info", "Carpeta eliminada: %s" % relative)
		elif FileAccess.file_exists(absolute):
			if DirAccess.remove_absolute(absolute) == OK:
				deleted += 1
				_log(logger, "info", "Archivo eliminado: %s" % relative)
	return deleted


func _remove_tree(path: String) -> bool:
	var dir := DirAccess.open(path)
	if dir == null:
		return false
	dir.list_dir_begin()
	var name := dir.get_next()
	while name != "":
		if name != "." and name != "..":
			var child := path.path_join(name)
			if dir.current_is_dir():
				_remove_tree(child)
			else:
				DirAccess.remove_absolute(child)
		name = dir.get_next()
	dir.list_dir_end()
	return DirAccess.remove_absolute(path) == OK


func _ensure_dir(path: String) -> bool:
	if path.is_empty():
		return true
	if DirAccess.dir_exists_absolute(path):
		return true
	return DirAccess.make_dir_recursive_absolute(path) == OK


func _fail(logger: Node, message: String) -> void:
	_log(logger, "error", message)
	failed.emit(message)


func _log(logger: Node, level: String, message: String) -> void:
	if logger != null and logger.has_method(level):
		logger.call(level, message)
