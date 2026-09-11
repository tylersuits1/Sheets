use std::process::Command;

const REPO: &str = "tylersuits1/Sheets";

#[derive(Debug, serde::Deserialize)]
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

/// Splits curl's `<body>\n<http_status>` output (see `fetch_latest_release`)
/// and turns a non-200 status into a specific message. GitHub's
/// `/releases/latest` 404s both for a private repo AND for a public repo
/// whose only release is still a draft — either way, "not out yet" is the
/// accurate read, not a parse failure.
fn parse_curl_output(output: &str) -> Result<Release, String> {
    let (body, status) = output.rsplit_once('\n').ok_or_else(|| "unexpected response from GitHub".to_string())?;
    match status.trim() {
        "200" => serde_json::from_str(body).map_err(|_| "unexpected response from GitHub".to_string()),
        "404" => Err("no public release found yet — the repo may still be private, or nothing's been published".to_string()),
        other => Err(format!("GitHub returned an unexpected status ({other})")),
    }
}

/// Shells out to `curl` for one GitHub API call rather than pulling in an
/// HTTP client crate — `git` is already a runtime dependency the same way
/// (see `theme_store::install_theme_from_git`'s predecessor).
fn fetch_latest_release() -> Result<Release, String> {
    let output = Command::new("curl")
        .args([
            "-sSL",
            "--max-time",
            "10",
            "-w",
            "\n%{http_code}",
            &format!("https://api.github.com/repos/{REPO}/releases/latest"),
        ])
        .output()
        .map_err(|e| format!("failed to run curl: {e}"))?;
    if !output.status.success() {
        return Err("couldn't reach GitHub".to_string());
    }
    parse_curl_output(&String::from_utf8_lossy(&output.stdout))
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

    #[test]
    fn a_404_reports_a_clear_not_yet_available_message() {
        let output = "{\"message\": \"Not Found\"}\n404";
        let err = parse_curl_output(output).unwrap_err();
        assert!(err.contains("no public release found yet"), "unexpected error: {err}");
    }

    #[test]
    fn a_200_parses_the_release() {
        let output = "{\"tag_name\": \"v0.2.0\", \"html_url\": \"https://example.com/releases/v0.2.0\"}\n200";
        let release = parse_curl_output(output).unwrap();
        assert_eq!(release.tag_name, "v0.2.0");
        assert_eq!(release.html_url, "https://example.com/releases/v0.2.0");
    }

    #[test]
    fn an_unexpected_status_is_reported_with_the_code() {
        let output = "{}\n500";
        let err = parse_curl_output(output).unwrap_err();
        assert!(err.contains("500"), "unexpected error: {err}");
    }
}
