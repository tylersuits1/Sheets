use std::process::Command;

const REPO: &str = "tylersuits1/Sheets";

#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
}

pub struct UpdateStatus {
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub update_available: bool,
}

/// Compares `latest_tag` (a GitHub release tag, e.g. `v0.2.0`) against
/// `current_version` (this build's own version, e.g. `0.1.0`) and reports
/// whether they differ. Not a real semver comparison — Sheets only ever
/// publishes newer releases, so "different" is all that matters here.
fn compare(latest_tag: &str, current_version: &str) -> bool {
    latest_tag.trim_start_matches('v') != current_version
}

/// Shells out to `curl` for one GitHub API call rather than pulling in an
/// HTTP client crate — `git` is already a runtime dependency the same way
/// (see `theme_store::install_theme_from_git`'s predecessor).
fn fetch_latest_release() -> Result<Release, String> {
    let output = Command::new("curl")
        .args(["-sSL", "--max-time", "10", &format!("https://api.github.com/repos/{REPO}/releases/latest")])
        .output()
        .map_err(|e| format!("failed to run curl: {e}"))?;
    if !output.status.success() {
        return Err("couldn't reach GitHub".to_string());
    }
    let body = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&body).map_err(|_| "unexpected response from GitHub".to_string())
}

pub fn check() -> Result<UpdateStatus, String> {
    let release = fetch_latest_release()?;
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let update_available = compare(&release.tag_name, &current_version);
    Ok(UpdateStatus {
        latest_version: release.tag_name.trim_start_matches('v').to_string(),
        current_version,
        release_url: release.html_url,
        update_available,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newer_tag_is_detected_regardless_of_the_v_prefix() {
        assert!(compare("v0.2.0", "0.1.0"));
        assert!(compare("0.2.0", "0.1.0"));
    }

    #[test]
    fn a_matching_tag_is_not_an_update() {
        assert!(!compare("v0.1.0", "0.1.0"));
        assert!(!compare("0.1.0", "0.1.0"));
    }
}
