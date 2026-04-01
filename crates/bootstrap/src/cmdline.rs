use std::io;

/// Reads the kernel command line from `/proc/cmdline`.
#[allow(dead_code)]
pub fn read_cmdline() -> Result<String, io::Error> {
    let content = std::fs::read_to_string("/proc/cmdline")?;
    Ok(content.trim().to_string())
}

/// Parses ocelot-specific parameters from the kernel command line.
///
/// Supports parameters like:
/// - `ocelot.root.type=<type>`
/// - `ocelot.root.device=<device>`
/// - `ocelot.console=<device>`
/// - `ocelot.log.level=<level>`
#[allow(dead_code)]
pub fn parse_cmdline(cmdline: &str) -> CmdlineParams {
    let mut params = CmdlineParams::default();

    for param in cmdline.split_whitespace() {
        if let Some(rest) = param.strip_prefix("ocelot.console=") {
            params.console = Some(rest.to_string());
        } else if let Some(rest) = param.strip_prefix("ocelot.log.level=") {
            params.log_level = Some(rest.to_string());
        } else if let Some(rest) = param.strip_prefix("ocelot.root.type=") {
            params.root_type = Some(rest.to_string());
        } else if let Some(rest) = param.strip_prefix("ocelot.root.device=") {
            params.root_device = Some(rest.to_string());
        }
    }

    params
}

#[derive(Debug, Default)]
pub struct CmdlineParams {
    pub console: Option<String>,
    pub log_level: Option<String>,
    pub root_type: Option<String>,
    pub root_device: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::parse_cmdline;

    #[test]
    fn test_parse_cmdline_basic() {
        let cmdline = "console=ttyS0 ocelot.console=ttyS1 ocelot.log.level=debug";
        let params = parse_cmdline(cmdline);
        assert_eq!(params.console, Some("ttyS1".to_string()));
        assert_eq!(params.log_level, Some("debug".to_string()));
        assert_eq!(params.root_type, None);
    }

    #[test]
    fn test_parse_cmdline_root() {
        let cmdline = "ocelot.root.type=block ocelot.root.device=/dev/vda2";
        let params = parse_cmdline(cmdline);
        assert_eq!(params.root_type, Some("block".to_string()));
        assert_eq!(params.root_device, Some("/dev/vda2".to_string()));
    }

    #[test]
    fn test_parse_cmdline_empty() {
        let params = parse_cmdline("");
        assert_eq!(params.console, None);
        assert_eq!(params.log_level, None);
    }
}
