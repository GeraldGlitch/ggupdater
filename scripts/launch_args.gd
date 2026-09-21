class_name LaunchArgs
extends RefCounted
## Parseo y validación de argumentos de línea de comandos.

var app_id: String = ""
var executable: String = ""
var current_version: String = ""
var pid: int = -1
var target_root: String = ""
var local_update: String = ""
var platform: String = ""

var valid: bool = false
var errors: PackedStringArray = []

## Modo preview: se lanzó sin --app. No actualiza nada, solo muestra la UI
## y el carrusel de banners (útil para previsualizar desde el editor).
var preview_mode: bool = false


## Parsea los argumentos del proceso actual.
static func parse() -> LaunchArgs:
	var args := LaunchArgs.new()
	args._parse_array(OS.get_cmdline_args() + OS.get_cmdline_user_args())
	return args


func _parse_array(argv: PackedStringArray) -> void:
	var raw := {}
	var i := 0
	while i < argv.size():
		var token := argv[i]
		if token.begins_with("--"):
			var key := token.substr(2)
			var value := ""
			if key.contains("="):
				var parts := key.split("=", true, 1)
				key = parts[0]
				value = parts[1]
			elif i + 1 < argv.size() and not argv[i + 1].begins_with("--"):
				i += 1
				value = argv[i]
			raw[key] = value
		i += 1

	app_id = String(raw.get("app", "")).strip_edges()
	executable = String(raw.get("executable", "")).strip_edges()
	current_version = String(raw.get("current-version", "")).strip_edges()
	local_update = String(raw.get("local-update", "")).strip_edges()

	if raw.has("pid"):
		var pid_text := String(raw["pid"]).strip_edges()
		if pid_text.is_valid_int():
			pid = int(pid_text)
		else:
			errors.append("El argumento --pid no es un entero válido: '%s'" % pid_text)

	platform = "windows" if OS.get_name() == "Windows" else ("linux" if OS.get_name() == "Linux" else OS.get_name().to_lower())
	target_root = PathUtils.get_target_root()

	_validate()


func _validate() -> void:
	# Si no hay --app, entra en modo preview (no actualiza, solo muestra UI/banners).
	preview_mode = app_id.is_empty()

	if preview_mode:
		errors = PackedStringArray()
		valid = false
		return

	if executable.is_empty():
		errors.append("Falta el argumento obligatorio --executable <archivo>.")
	if current_version.is_empty():
		errors.append("Falta el argumento obligatorio --current-version <x.y.z>.")
	errors = _dedupe(errors)
	valid = errors.is_empty()


static func _dedupe(source: PackedStringArray) -> PackedStringArray:
	var seen := {}
	var out := PackedStringArray()
	for item in source:
		if not seen.has(item):
			seen[item] = true
			out.append(item)
	return out


func to_dictionary() -> Dictionary:
	return {
		"app_id": app_id,
		"executable": executable,
		"current_version": current_version,
		"pid": pid,
		"target_root": target_root,
		"local_update": local_update,
		"platform": platform,
		"valid": valid,
		"preview_mode": preview_mode,
	}
