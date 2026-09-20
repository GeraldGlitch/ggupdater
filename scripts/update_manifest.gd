class_name UpdateManifest
extends RefCounted
## Representa y valida el manifest de actualización.
## Selecciona automáticamente el bloque de plataforma (windows / linux).

const PLACEHOLDER := "PENDING"

var app_id: String = ""
var version: String = ""
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
	if m.app_id.is_empty():
		m.errors.append("El manifest no contiene 'app_id'.")
	if m.version.is_empty():
		m.errors.append("El manifest no contiene 'version'.")

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
		"download_url": download_url,
		"sha256": sha256,
		"delete": Array(delete_list),
	}
