class_name ProjectConfig
extends RefCounted
## Configuración del proyecto destino, fijada en tiempo de exportación.
## Los releases de todas las apps se centralizan en el repo GGUpdater, y cada
## proyecto tiene su manifest en `manifests/<app_id>.json` dentro de ese repo.
## Cambia APP_ID por el slug del proyecto (minúsculas, sin guiones) y reexporta.

const GITHUB_OWNER := "GeraldGlitch"
const GITHUB_REPO := "ggupdater"
const GITHUB_BRANCH := "main"

## Única variable a cambiar por proyecto. Debe coincidir EXACTAMENTE con el
## valor de --app y con el app_id del manifest.
const APP_ID := "chibifx"


## URL del manifest del proyecto: manifests/<app_id>.json en el repo central.
static func manifest_url() -> String:
	return "https://raw.githubusercontent.com/%s/%s/%s/manifests/%s.json" % [
		GITHUB_OWNER, GITHUB_REPO, GITHUB_BRANCH, APP_ID,
	]
