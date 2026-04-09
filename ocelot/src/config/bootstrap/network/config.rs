use serde::Deserialize;

/// `NetworkConfig`: Network config (serialization type).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkConfig {
    #[serde(default)]
    pub mode: NetworkMode,
    #[serde(default)]
    pub interfaces: Vec<InterfaceConfig>,
}

/// `NetworkMode`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NetworkMode {
    #[default]
    Dhcp,
    Static,
}

/// `InterfaceConfig`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterfaceConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub netmask: Option<String>,
    #[serde(default)]
    pub gateway: Option<String>,
}

impl From<NetworkConfig> for ocelot_bootstrap::NetworkConfig {
    fn from(config: NetworkConfig) -> Self {
        Self {
            mode: match config.mode {
                NetworkMode::Dhcp => ocelot_bootstrap::NetworkMode::Dhcp,
                NetworkMode::Static => ocelot_bootstrap::NetworkMode::Static,
            },
            interfaces: config.interfaces.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<InterfaceConfig> for ocelot_bootstrap::InterfaceConfig {
    fn from(config: InterfaceConfig) -> Self {
        Self {
            name: config.name,
            address: config.address,
            netmask: config.netmask,
            gateway: config.gateway,
        }
    }
}
