use crate::powmon::*;
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::fs;

#[derive(Debug)]
pub struct HexU8(pub u8);

impl<'de> Deserialize<'de> for HexU8 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let value =
            u8::from_str_radix(s.trim_start_matches("0x"), 16).map_err(serde::de::Error::custom)?;
        Ok(HexU8(value))
    }
}

impl fmt::Display for HexU8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02X}", self.0)
    }
}

#[derive(Debug, Deserialize)]
struct RootConfig {
    #[serde(default)]
    powmon: Powmon,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Powmon {
    address: HexU8,
    verbose: bool,
    i2c_speed_hz: u32,
    reset_on_open: bool,
    timeout: u64,
    chip: Chipsetup,
}

impl Default for Powmon {
    fn default() -> Self {
        Powmon {
            address: HexU8(0x4A),
            verbose: false,
            i2c_speed_hz: 400_000,
            reset_on_open: false,
            timeout: 10,
            chip: Chipsetup::new(),
        }
    }
}

/**
 * Reads and parses the powmon section of the config file shared with iocc.
 * The file path is provided by iocc through the IOCC_CONFIG environment variable.
 * Missing or invalid files fall back to the default configuration.
 */
pub fn read_config() -> Powmon {
    let Some(path) = std::env::var_os("IOCC_CONFIG") else {
        eprintln!("Warning: IOCC_CONFIG not set; using default configuration");
        return Powmon::default();
    };

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Warning: cannot read {}: {e}; using default configuration",
                std::path::Path::new(&path).display()
            );
            return Powmon::default();
        }
    };

    match serde_yaml::from_str::<RootConfig>(&content) {
        Ok(cfg) => cfg.powmon,
        Err(e) => {
            eprintln!("Warning: invalid powmon configuration: {e}; using default configuration");
            Powmon::default()
        }
    }
}

/**
 * getters for Powmon structure
 */
impl Powmon {
    pub fn get_address(&self) -> u8 {
        self.address.0
    }
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }
    pub fn get_speed(&self) -> u32 {
        self.i2c_speed_hz
    }
    pub fn is_reset_on_open(&self) -> bool {
        self.reset_on_open
    }
    pub fn get_timeout(&self) -> u64 {
        self.timeout
    }
    pub fn get_chip_config(&self) -> Chipsetup {
        self.chip
    }
}
