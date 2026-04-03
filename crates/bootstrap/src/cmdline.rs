use std::io;

/// Reads the kernel command line from `/proc/cmdline`.
fn read_cmdline() -> Result<String, io::Error> {
    let content = std::fs::read_to_string("/proc/cmdline")?;
    Ok(content.trim().to_string())
}

/// Parses the `ocelot.config` parameter from the kernel command line.
fn parse_config_param(cmdline: &str) -> Option<String> {
    cmdline
        .split_whitespace()
        .find_map(|param| param.strip_prefix("ocelot.config="))
        .map(str::to_string)
}

/// Reads the config file path from kernel command line.
///
/// Returns the value of `ocelot.config=<path>` parameter if present,
/// or `None` if not found or if cmdline cannot be read.
#[must_use]
pub fn get_config_path() -> Option<String> {
    let cmdline = read_cmdline().ok()?;
    parse_config_param(&cmdline)
}

#[cfg(test)]
mod tests {
    use super::parse_config_param;

    #[test]
    fn test_parse_config_param() {
        let cmdline = "console=ttyS0 ocelot.config=/path/to/config.yaml";
        assert_eq!(parse_config_param(cmdline), Some("/path/to/config.yaml".to_string()));
    }

    #[test]
    fn test_parse_config_param_not_present() {
        let cmdline = "console=ttyS0 ocelot.log.level=debug";
        assert_eq!(parse_config_param(cmdline), None);
    }

    #[test]
    fn test_parse_config_param_empty() {
        assert_eq!(parse_config_param(""), None);
    }
}
