pub const GITHUB_OWNER: &str = "GeraldGlitch";
pub const GITHUB_REPO: &str = "ggupdater";
pub const GITHUB_BRANCH: &str = "main";

/// URL del manifest del proyecto, resuelto con el `--app` recibido.
/// Un mismo binario sirve para todas las apps; el manifest se elige por app_id.
pub fn manifest_url(app_id: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/{GITHUB_OWNER}/{GITHUB_REPO}/{GITHUB_BRANCH}/manifests/{app_id}.json"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_manifest_url_from_app_id() {
        let url = manifest_url("blackcatpos");
        assert!(url.contains("GeraldGlitch/ggupdater/main"));
        assert!(url.ends_with("/manifests/blackcatpos.json"));
    }
}
