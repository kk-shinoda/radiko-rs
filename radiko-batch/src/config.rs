use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub settings: Settings,
    pub programs: Vec<ProgramConfig>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default = "default_filename_template")]
    pub filename_template: String,
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_output_dir() -> String {
    "./archive".to_string()
}

fn default_filename_template() -> String {
    "{name}_{date}".to_string()
}

fn default_format() -> String {
    "m4a".to_string()
}

#[derive(Debug, Deserialize)]
pub struct ProgramConfig {
    pub name: String,
    pub station_id: String,
    pub weekday: Weekday,
    pub start_time: String,
    pub duration_minutes: u32,
    pub output_dir: Option<String>,
    pub filename_template: Option<String>,
}

pub fn load_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TOML: &str = r#"
[settings]
output_dir = "~/radiko"
filename_template = "{name}_{date}"
format = "m4a"

[[programs]]
name = "スキモア"
station_id = "FMT"
weekday = "sat"
start_time = "00:00"
duration_minutes = 30
output_dir = "sukimoa"
filename_template = "sukimoa_{date}"

[[programs]]
name = "オールナイトニッポン"
station_id = "LFR"
weekday = "wed"
start_time = "01:00"
duration_minutes = 120
"#;

    #[test]
    fn test_parse_config() {
        let config: Config = toml::from_str(SAMPLE_TOML).unwrap();
        assert_eq!(config.settings.output_dir, "~/radiko");
        assert_eq!(config.settings.filename_template, "{name}_{date}");
        assert_eq!(config.settings.format, "m4a");
        assert_eq!(config.programs.len(), 2);
    }

    #[test]
    fn test_parse_weekday() {
        let config: Config = toml::from_str(SAMPLE_TOML).unwrap();
        assert_eq!(config.programs[0].weekday, Weekday::Sat);
        assert_eq!(config.programs[1].weekday, Weekday::Wed);
    }

    #[test]
    fn test_program_optional_fields() {
        let config: Config = toml::from_str(SAMPLE_TOML).unwrap();
        // 1つ目はオプションフィールドあり（ベースからの相対パス）
        assert_eq!(
            config.programs[0].output_dir.as_deref(),
            Some("sukimoa")
        );
        assert_eq!(
            config.programs[0].filename_template.as_deref(),
            Some("sukimoa_{date}")
        );
        // 2つ目はオプションフィールドなし
        assert!(config.programs[1].output_dir.is_none());
        assert!(config.programs[1].filename_template.is_none());
    }

    #[test]
    fn test_default_format() {
        let minimal = r#"
[settings]
output_dir = "~/radiko"

[[programs]]
name = "Test"
station_id = "TBS"
weekday = "mon"
start_time = "10:00"
duration_minutes = 60
"#;
        let config: Config = toml::from_str(minimal).unwrap();
        assert_eq!(config.settings.format, "m4a");
        assert_eq!(config.settings.filename_template, "{name}_{date}");
    }

    #[test]
    fn test_default_output_dir() {
        let minimal = r#"
[settings]

[[programs]]
name = "Test"
station_id = "TBS"
weekday = "mon"
start_time = "10:00"
duration_minutes = 60
"#;
        let config: Config = toml::from_str(minimal).unwrap();
        assert_eq!(config.settings.output_dir, "./archive");
    }

    #[test]
    fn test_load_config_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, SAMPLE_TOML).unwrap();
        let config = load_config(&path).unwrap();
        assert_eq!(config.programs.len(), 2);
        assert_eq!(config.programs[0].name, "スキモア");
    }
}
