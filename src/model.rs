//! Pure, host-tested plugin facts: the supported game version, the plugin's
//! identity and data paths, and the install line.

pub const SUPPORTED_VERSION: &str = "13.0.5";
pub const PLUGIN: &str = "ssbu-cpu-pilot";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DATA_DIR: &str = "sd:/ultimate/ssbu-cpu-pilot";
pub const EVENTS_LOG: &str = "sd:/ultimate/ssbu-cpu-pilot/events.log";
pub const CONFIG_INI: &str = "sd:/ultimate/ssbu-cpu-pilot/config.ini";

/// True only for the exact supported display version.
pub fn version_supported(display: &str) -> bool {
    display.trim_end_matches('\0').trim() == SUPPORTED_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version() {
        assert!(version_supported("13.0.5"));
        assert!(version_supported("13.0.5\0\0"));
        assert!(!version_supported("13.0.4"));
        assert!(!version_supported("13.0.50"));
        assert!(!version_supported(""));
    }

    #[test]
    fn paths_share_the_data_dir() {
        assert!(EVENTS_LOG.starts_with(DATA_DIR));
        assert!(CONFIG_INI.starts_with(DATA_DIR));
        assert_eq!(PLUGIN, "ssbu-cpu-pilot");
    }
}
