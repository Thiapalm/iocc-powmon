use crate::powmon::Chipsetup;

/// Flag the plugin answers to describe itself to the iocc TUI
pub const MANIFEST_FLAG: &str = "--iocc-manifest";

/// Panels and actions the `iocc tui` shows for this plugin
pub const JSON: &str = r#"{
  "protocol": 1,
  "title": "Power monitor (INA226)",
  "version": "0.3.0",
  "description": "INA226 voltage, current and power monitor through the MCP2221",
  "views": [
    {"id": "live", "label": "Live readings", "kind": "metrics", "stream": true,
     "device": "shared", "command": ["watch", "--json", "--interval-ms", "200"]},
    {"id": "status", "label": "Status", "kind": "table", "device": "shared",
     "refresh_ms": 5000, "command": ["status", "--json"]}
  ],
  "actions": [
    {"id": "mode", "label": "Set mode", "description": "change the operating mode",
     "args": [{"name": "mode", "label": "Mode", "type": "choice",
               "options": ["PowerDown", "ShuntVoltageTriggered", "BusVoltageTriggered",
                           "ShuntAndBusTriggered", "PowerDown2", "ShuntVoltageContinuous",
                           "BusVoltageContinuous", "ShuntAndBusContinuous"]}],
     "device": "shared", "command": ["mode", "{mode}"]},
    {"id": "calibrate", "label": "Calibrate", "description": "set the maximum expected current",
     "args": [{"name": "max", "label": "Max current (A)", "type": "number", "default": "5.0"}],
     "command": ["calibrate", "--max", "{max}"]},
    {"id": "reset", "label": "Reset chip", "confirm": true, "command": ["reset"]}
  ]
}"#;

/**
 * JSON table with the decoded configuration, for the `status --json` view
 */
pub fn status_table(setup: &Chipsetup, address: u8, configuration: u16) -> String {
    let rows: Vec<[String; 2]> = setup
        .describe(address, configuration)
        .into_iter()
        .map(|(name, value)| [name, value])
        .collect();
    serde_json::json!({
        "columns": ["Field", "Value"],
        "rows": rows,
    })
    .to_string()
}

/**
 * JSON metrics with the four readings, for the `read --json` view
 */
pub fn metrics(volts: f64, amps: f64, watts: f64, shunt: f64) -> String {
    serde_json::json!({
        "metrics": [
            {"name": "Bus voltage", "value": volts, "unit": "V"},
            {"name": "Current", "value": amps, "unit": "A"},
            {"name": "Power", "value": watts, "unit": "W"},
            {"name": "Shunt", "value": shunt, "unit": "V"},
        ]
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_valid_for_the_tui_protocol() {
        let value: serde_json::Value = serde_json::from_str(JSON).unwrap();
        assert_eq!(value["protocol"], 1);
        assert_eq!(value["views"].as_array().unwrap().len(), 2);
        assert_eq!(value["actions"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn status_and_metrics_parse() {
        let table: serde_json::Value =
            serde_json::from_str(&status_table(&Chipsetup::new(), 0x4A, 0x4527)).unwrap();
        assert_eq!(table["columns"][0], "Field");
        assert!(table["rows"].as_array().unwrap().len() >= 5);

        let m: serde_json::Value = serde_json::from_str(&metrics(12.0, 1.0, 12.0, 0.0)).unwrap();
        assert_eq!(m["metrics"].as_array().unwrap().len(), 4);
    }
}
