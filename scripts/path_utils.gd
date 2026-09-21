class_name PathUtils
extends RefCounted
## Utilidades de normalización y seguridad de paths.
## Todas las comprobaciones son globales (funciones estáticas) para poder
## usarse desde cualquier manager sin instanciar.

const FORBIDDEN_SEGMENTS := ["..", "."]


## Devuelve el root absoluto de la aplicación a actualizar (siempre ../).
## En el editor se deriva de res://; en el binario exportado, del ejecutable.
static func get_target_root() -> String:
	if OS.has_feature("editor"):
		return ProjectSettings.globalize_path("res://../").simplify_path()
	return OS.get_executable_path().get_base_dir().path_join("..").simplify_path()


## Une root + relative respetando separadores y devuelve un path absoluto simplificado.
static func resolve_under(root: String, relative: String) -> String:
	var root_abs := root.simplify_path().trim_suffix("/")
	var rel := relative.replace("\\", "/").trim_prefix("/")
	return (root_abs + "/" + rel).simplify_path()


## True si candidate queda dentro de root (o es igual a root).
static func is_inside(root: String, candidate: String) -> bool:
	var root_abs := root.simplify_path().trim_suffix("/")
	var cand := candidate.simplify_path()
	if cand == root_abs:
		return true
	return cand.begins_with(root_abs + "/")


## Rechaza paths absolutos, con traversal, unidades Windows o NUL.
static func is_safe_relative(relative: String) -> bool:
	if relative.is_empty():
		return false
	if relative.contains(char(0)):
		return false
	var normalized := relative.replace("\\", "/")
	if normalized.begins_with("/"):
		return false
	# Unidad Windows tipo C: o cualquier "X:".
	if normalized.length() >= 2 and normalized[1] == ":":
		return false
	for segment in normalized.split("/", false):
		if segment in FORBIDDEN_SEGMENTS:
			return false
	return true


## Comprueba que relative sea seguro y quede dentro de root.
static func is_safe_target(root: String, relative: String) -> bool:
	if not is_safe_relative(relative):
		return false
	return is_inside(root, resolve_under(root, relative))


## True si relative está protegido por la lista de exclusiones.
static func is_excluded(relative: String, excluded: Array) -> bool:
	var normalized := relative.replace("\\", "/").trim_prefix("/")
	for entry in excluded:
		var prefix := String(entry).replace("\\", "/").trim_prefix("/")
		if normalized == prefix or normalized.begins_with(prefix):
			return true
	return false


## Normaliza un path venido del ZIP para usarlo como path relativo seguro.
static func sanitize_zip_path(entry_path: String) -> String:
	var normalized := entry_path.replace("\\", "/")
	while normalized.begins_with("./"):
		normalized = normalized.substr(2)
	normalized = normalized.trim_prefix("/")
	return normalized
