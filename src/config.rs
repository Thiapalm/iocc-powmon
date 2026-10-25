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
    read_config_with(true)
}

/**
 * Same as `read_config`, but silent when the file is missing or unreadable.
 * Used to fill the TUI manifest defaults, where a warning would only be noise.
 */
pub fn read_config_quiet() -> Powmon {
    read_config_with(false)
}

fn read_config_with(warn: bool) -> Powmon {
    let Some(path) = std::env::var_os("IOCC_CONFIG") else {
        if warn {
            eprintln!("Warning: IOCC_CONFIG not set; using default configuration");
        }
        return Powmon::default();
    };

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            if warn {
                eprintln!(
                    "Warning: cannot read {}: {e}; using default configuration",
                    std::path::Path::new(&path).display()
                );
            }
            return Powmon::default();
        }
    };

    match serde_yaml::from_str::<RootConfig>(&content) {
        Ok(cfg) => cfg.powmon,
        Err(e) => {
            if warn {
                eprintln!(
                    "Warning: invalid powmon configuration: {e}; using default configuration"
                );
            }
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

/**
 * Persists one `powmon:` -> `chip:` entry (`mode`, `maxcurrent`, ...) in the
 * shared config.yaml, so the next run starts with the value that is in use now.
 *
 * Only that line is rewritten: comments, indentation and every other section
 * are kept byte for byte. Returns the file that was written.
 */
pub fn save_chip_entry(key: &str, value: &str) -> Result<std::path::PathBuf, String> {
    let Some(path) = std::env::var_os("IOCC_CONFIG") else {
        return Err("IOCC_CONFIG not set, cannot save the configuration".into());
    };
    let path = std::path::PathBuf::from(path);
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let updated = replace_chip_entry(&text, key, value)?;
    fs::write(&path, updated).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

/**
 * Rewrites `key: value` inside the `powmon.chip` block. When the entry does not
 * exist yet it is inserted at the end of the block, which keeps hand written
 * files working. Fails when there is no `powmon.chip` section at all.
 */
fn replace_chip_entry(text: &str, key: &str, value: &str) -> Result<String, String> {
    let indent_of = |line: &str| line.len() - line.trim_start().len();
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let mut in_powmon = false;
    // (indent of `chip:`, index of the last content line of the block)
    let mut chip: Option<(usize, usize)> = None;
    // Last `powmon.chip` block seen, kept after the block ends for insertion
    let mut chip_block: Option<(usize, usize)> = None;
    // (index, inline comment) of the entry to rewrite, once found
    let mut found: Option<(usize, String)> = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = indent_of(line);
        if indent == 0 {
            in_powmon = trimmed.starts_with("powmon:");
            chip = None;
            continue;
        }
        if !in_powmon {
            continue;
        }
        match chip {
            None => {
                if trimmed.starts_with("chip:") {
                    chip = Some((indent, i));
                    chip_block = Some((indent, i));
                }
            }
            Some((chip_indent, _)) if indent <= chip_indent => chip = None,
            Some((chip_indent, _)) => {
                if let Some(rest) = trimmed.strip_prefix(&format!("{key}:")) {
                    // Keep the inline comment as it was (e.g. `#in Amps`)
                    let comment = rest
                        .find('#')
                        .map(|at| format!(" {}", &rest[at..]))
                        .unwrap_or_default();
                    found = Some((i, comment));
                    break;
                }
                chip = Some((chip_indent, i));
                chip_block = chip;
            }
        }
    }

    if let Some((i, comment)) = found {
        let indent = indent_of(&lines[i]);
        lines[i] = format!("{}{key}: {value}{comment}", " ".repeat(indent));
        return Ok(join_lines(lines, text));
    }
    let Some((chip_indent, last)) = chip_block else {
        return Err(format!(
            "no powmon.chip section in config.yaml, {key} not saved"
        ));
    };
    let entry = format!("{}{key}: {value}", " ".repeat(chip_indent + 2));
    lines.insert(last + 1, entry);
    Ok(join_lines(lines, text))
}

fn join_lines(lines: Vec<String>, original: &str) -> String {
    let mut content = lines.join("\n");
    if original.ends_with('\n') {
        content.push('\n');
    }
    content
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "iocc:\n  verbose: true\npowmon:\n  address: 0x4A\n  chip:\n    # keep me\n    mode: BusVoltageContinuous # old\n    average: _16\n    maxcurrent: 5.0 #in Amps\nont:\n  port: a\n";

    #[test]
    fn replaces_only_the_requested_entry() {
        let text = replace_chip_entry(SAMPLE, "mode", "ShuntAndBusContinuous").unwrap();
        assert!(text.contains("mode: ShuntAndBusContinuous # old"));
        assert!(text.contains("maxcurrent: 5.0 #in Amps"));
        assert!(text.contains("# keep me") && text.contains("ont:") && text.contains("verbose"));

        let text = replace_chip_entry(SAMPLE, "maxcurrent", "2").unwrap();
        assert!(text.contains("maxcurrent: 2 #in Amps"));
        assert!(text.contains("mode: BusVoltageContinuous # old"));
    }

    #[test]
    fn inserts_a_missing_entry_at_the_end_of_the_block() {
        let text = replace_chip_entry(SAMPLE, "vbusct", "_2_116_ms").unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let added = lines.iter().position(|l| *l == "    vbusct: _2_116_ms");
        assert_eq!(added, Some(9), "{text}");
        assert_eq!(lines[10], "ont:");
    }

    #[test]
    fn refuses_a_file_without_the_powmon_chip_section() {
        let err = replace_chip_entry("iocc:\n  verbose: true\n", "mode", "PowerDown").unwrap_err();
        assert!(err.contains("no powmon.chip section"), "{err}");
    }

    #[test]
    fn save_chip_entry_writes_the_shared_file() {
        let dir = std::env::temp_dir().join(format!("iocc-powmon-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.yaml");
        fs::write(&path, SAMPLE).unwrap();
        // SAFETY: only this test touches IOCC_CONFIG
        unsafe { std::env::set_var("IOCC_CONFIG", &path) };
        let saved = save_chip_entry("maxcurrent", "2").unwrap();
        let text = fs::read_to_string(&path).unwrap();
        unsafe { std::env::remove_var("IOCC_CONFIG") };
        assert_eq!(saved, path);
        assert!(text.contains("maxcurrent: 2 #in Amps"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn saved_file_reads_back_as_the_new_value() {
        let mut text = replace_chip_entry(SAMPLE, "maxcurrent", "2").unwrap();
        text = replace_chip_entry(&text, "mode", "ShuntAndBusContinuous").unwrap();
        let root: RootConfig = serde_yaml::from_str(&text).unwrap();
        assert_eq!(root.powmon.get_chip_config().maxcurrent, 2.0);
        assert_eq!(
            root.powmon.get_chip_config().mode,
            InaMode::ShuntAndBusContinuous
        );
    }
}
