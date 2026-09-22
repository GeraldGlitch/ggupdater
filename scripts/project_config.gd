class_name ProjectConfig
extends RefCounted
## Configuración del repositorio central de manifests. GGUpdater es una única
## build genérica: el manifest se resuelve a partir del argumento --app en
## `manifests/<app_id>.json`, sin atar el binario a un proyecto concreto.

const GITHUB_OWNER := "GeraldGlitch"
const GITHUB_REPO := "ggupdater"
const GITHUB_BRANCH := "main"


## URL del manifest de la app indicada por --app: manifests/<app_id>.json.
static func manifest_url(app_id: String) -> String:
	return "https://raw.githubusercontent.com/%s/%s/%s/manifests/%s.json" % [
		GITHUB_OWNER, GITHUB_REPO, GITHUB_BRANCH, app_id,
	]
