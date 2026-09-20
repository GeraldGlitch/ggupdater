class_name AppRelauncher
extends RefCounted
## Lanza de nuevo el ejecutable de la app tras completar la actualización.
## En Linux asegura permisos de ejecución; en Windows no altera nada.

const EXCLUDED_PATHS := [
	"GGUpdater/",
]


## Intenta lanzar executable dentro de target_root. Devuelve true si lo envió.
func relaunch(target_root: String, executable: String, logger: Node = null) -> bool:
	if PathUtils.is_excluded(executable, EXCLUDED_PATHS):
		_log(logger, "error", "Por seguridad no se relanza un ejecutable dentro de GGUpdater/.")
		return false
	if not PathUtils.is_safe_target(target_root, executable):
		_log(logger, "error", "Ruta de ejecutable insegura: %s" % executable)
		return false

	var absolute := PathUtils.resolve_under(target_root, executable)
	if not FileAccess.file_exists(absolute):
		_log(logger, "error", "El ejecutable no existe: %s" % absolute)
		return false

	if OS.get_name() != "Windows":
		_ensure_executable(absolute, logger)

	var pid := OS.create_process(absolute, [], false)
	if pid <= 0:
		_log(logger, "error", "No se pudo lanzar el ejecutable: %s" % absolute)
		return false

	_log(logger, "info", "Aplicación relanzada (pid %d): %s" % [pid, absolute])
	return true


## Asegura chmod +x. Godot 4.7 no expone una API nativa de permisos,
## así que en Unix se usa 'chmod' del sistema; en Windows no se hace nada.
func _ensure_executable(path: String, logger: Node) -> void:
	if OS.get_name() == "Windows":
		return
	var output: Array = []
	var code := OS.execute("chmod", ["+x", path], output, true)
	if code == 0:
		_log(logger, "info", "Permiso de ejecución restaurado en: %s" % path)
	else:
		_log(logger, "warn", "No se pudo marcar como ejecutable: %s (código %d)" % [path, code])


func _log(logger: Node, level: String, message: String) -> void:
	if logger != null and logger.has_method(level):
		logger.call(level, message)
