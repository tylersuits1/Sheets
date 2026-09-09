use font_kit::source::SystemSource;

/// Every font family installed on this machine — Core Text on macOS,
/// fontconfig on Linux (via font-kit), the same registries Font Book /
/// `fc-list` read from.
pub fn list_font_families() -> Result<Vec<String>, String> {
    let mut families = SystemSource::new().all_families().map_err(|e| e.to_string())?;
    families.sort();
    families.dedup();
    Ok(families)
}
