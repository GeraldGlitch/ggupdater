class_name UpdateManifest
extends RefCounted
## Representa y valida el manifest de actualización.
## `download_url` puede ser un string universal o un objeto por plataforma
## (windows / linux); GGUpdater selecciona la del SO actual.

const PLACEHOLDER := "PENDING"

var app_id: String = ""
var version: String = ""
var version_number: int = -1
var download_url: String = ""
var sha256: String = ""
var delete_list: PackedStringArray = PackedStringArray()

var loaded: bool = false
var errors: PackedStringArray = []


static func from_dictionary(data: Dictionary, platform: String) -> UpdateManifest:
	var m := UpdateManifest.new()
	if data.is_empty():
		m.errors.append("El manifest está vacío.")
		return m

	m.app_id = String(data.get("app_id", "")).strip_edges()
	m.version = String(data.get("version", "")).strip_edges()
	m.version_number = parse_version_number(data.get("version_number", null))
	m.download_url = resolve_platform_value(data.get("download_url", null), platform)
	m.sha256 = resolve_platform_value(data.get("sha256", null), platform)
	if m.app_id.is_empty():
		m.errors.append("El manifest no contiene 'app_id'.")
	if m.download_url.is_empty():
		m.errors.append("El manifest no contiene 'download_url' para la plataforma '%s'." % platform)

	var raw_delete: Variant = data.get("delete", [])
	if raw_delete is Array:
		for item in raw_delete:
			var value := String(item).strip_edges()
			if not value.is_empty():
				m.delete_list.append(value)

	m.loaded = m.errors.is_empty()
	return m


## Acepta un string (universal) o un objeto con claves por plataforma.
## Devuelve el valor de la plataforma actual o "" si no existe.
static func resolve_platform_value(value: Variant, platform: String) -> String:
	if value is String:
		return String(value).strip_edges()
	if value is Dictionary:
		var block: Dictionary = value
		if block.has(platform):
			return String(block[platform]).strip_edges()
	return ""


## Convierte un valor de manifest a entero de versión. JSON parsea los números
## como float, por eso se aceptan int, float entero y string numérico.
## Devuelve -1 si el valor no es un entero no negativo válido.
static func parse_version_number(value: Variant) -> int:
	if value is int:
		return value if value >= 0 else -1
	if value is float:
		return int(value) if value >= 0.0 and is_equal_approx(value, roundf(value)) else -1
	if value is String:
		var text: String = value.strip_edges()
		if text.is_valid_int():
			return int(text) if int(text) >= 0 else -1
	return -1


## True si la URL es un placeholder o está vacía (aún sin configurar).
func is_url_pending() -> bool:
	return download_url.is_empty() or download_url == PLACEHOLDER


## True si el hash es placeholder o está vacío: se permite saltar validación en dev.
func is_sha_pending() -> bool:
	return sha256.is_empty() or sha256 == PLACEHOLDER
