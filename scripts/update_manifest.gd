class_name UpdateManifest
extends RefCounted
## Representa y valida el manifest de actualización.
## Selecciona automáticamente el bloque de plataforma (windows / linux).

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
	if m.app_id.is_empty():
		m.errors.append("El manifest no contiene 'app_id'.")
	if m.version.is_empty():
		m.errors.append("El manifest no contiene 'version'.")
	if m.version_number < 0:
		m.errors.append("El manifest debe contener 'version_number' como entero no negativo.")

	var block: Dictionary = data.get(platform, {})
	if block.is_empty():
		m.errors.append("El manifest no contiene bloque para la plataforma '%s'." % platform)
	else:
		m.download_url = String(block.get("url", "")).strip_edges()
		m.sha256 = String(block.get("sha256", "")).strip_edges()

	var raw_delete: Variant = data.get("delete", [])
	if raw_delete is Array:
		for item in raw_delete:
			var value := String(item).strip_edges()
			if not value.is_empty():
				m.delete_list.append(value)

	m.loaded = m.errors.is_empty()
	return m


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


func to_dictionary() -> Dictionary:
	return {
		"app_id": app_id,
		"version": version,
		"version_number": version_number,
		"download_url": download_url,
		"sha256": sha256,
		"delete": Array(delete_list),
	}
