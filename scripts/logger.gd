extends Node
## Logger sencillo con salida a consola y archivo rotado por día.
## Nunca registra secretos ni contenido sensible.

const LOG_DIR := "user://ggupdater/logs/"
const PREFIX := "[GGUpdater]"

var _context: Dictionary = {}
var _log_path: String = ""


func configure(app_id: String, current_version: String, target_version: String) -> void:
	_context = {
		"app_id": app_id,
		"current_version": current_version,
		"target_version": target_version,
		"platform": OS.get_name(),
	}
	_ensure_dir()
	_log_path = LOG_DIR + "ggupdater_%s.log" % Time.get_datetime_string_from_system().replace(":", "-").substr(0, 10)


func _ensure_dir() -> void:
	if not DirAccess.dir_exists_absolute(LOG_DIR):
		DirAccess.make_dir_recursive_absolute(LOG_DIR)


func info(message: String) -> void:
	_write("INFO", message)


func warn(message: String) -> void:
	_write("WARN", message)
	push_warning(message)


func error(message: String) -> void:
	_write("ERROR", message)
	push_error(message)


func _write(level: String, message: String) -> void:
	if _log_path.is_empty():
		_ensure_dir()
		_log_path = LOG_DIR + "ggupdater_%s.log" % Time.get_datetime_string_from_system().replace(":", "-").substr(0, 10)
	var stamp := Time.get_datetime_string_from_system()
	var line := "%s %s %s" % [stamp, level, message]
	if not _context.is_empty():
		line += " | " + JSON.stringify(_context)
	print(line)
	var file := FileAccess.open(_log_path, FileAccess.READ_WRITE)
	if file == null:
		return
	file.seek_end()
	file.store_line(line)
	file.close()
