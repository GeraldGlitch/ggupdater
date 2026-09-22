pub const GITHUB_OWNER: &str = "GeraldGlitch";
pub const GITHUB_REPO: &str = "ggupdater";
pub const GITHUB_BRANCH: &str = "main";

pub const APP_ID: &str = "chibifx";

pub fn manifest_url() -> String {
    format!(
        "https://raw.githubusercontent.com/{GITHUB_OWNER}/{GITHUB_REPO}/{GITHUB_BRANCH}/manifests/{APP_ID}.json"
    )
}
