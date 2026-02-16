use std::path::PathBuf;

use crate::config::{ProgramConfig, Settings, Weekday};
use crate::schedule::Episode;

/// エピソードの出力パスを解決する。
/// settings.output_dir / [program.output_dir] / filename_template.format
pub fn resolve_output_path(
    episode: &Episode,
    program: &ProgramConfig,
    settings: &Settings,
) -> PathBuf {
    let base = PathBuf::from(&settings.output_dir);
    let dir = match &program.output_dir {
        Some(sub) => base.join(sub),
        None => base,
    };

    let template = program
        .filename_template
        .as_deref()
        .unwrap_or(&settings.filename_template);
    let filename = format!(
        "{}.{}",
        expand_template(template, episode, program),
        settings.format
    );

    dir.join(filename)
}

/// 出力パスにファイルが既に存在するか判定する。
pub fn is_already_recorded(path: &PathBuf) -> bool {
    path.exists()
}

fn expand_template(template: &str, episode: &Episode, program: &ProgramConfig) -> String {
    let weekday_str = match program.weekday {
        Weekday::Mon => "mon",
        Weekday::Tue => "tue",
        Weekday::Wed => "wed",
        Weekday::Thu => "thu",
        Weekday::Fri => "fri",
        Weekday::Sat => "sat",
        Weekday::Sun => "sun",
    };

    template
        .replace("{name}", &program.name)
        .replace("{date}", &episode.date.format("%Y%m%d").to_string())
        .replace(
            "{datetime}",
            &episode.start.format("%Y%m%d%H%M%S").to_string(),
        )
        .replace("{station}", &program.station_id)
        .replace("{weekday}", weekday_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    fn make_episode(date_str: &str, start_time: &str, end_time: &str) -> Episode {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
        let start = NaiveDateTime::new(
            date,
            NaiveTime::parse_from_str(start_time, "%H:%M").unwrap(),
        );
        let end = NaiveDateTime::new(
            date,
            NaiveTime::parse_from_str(end_time, "%H:%M").unwrap(),
        );
        Episode { start, end, date }
    }

    fn make_settings(output_dir: &str, template: &str, format: &str) -> Settings {
        Settings {
            output_dir: output_dir.to_string(),
            filename_template: template.to_string(),
            format: format.to_string(),
        }
    }

    fn make_program(
        name: &str,
        station_id: &str,
        weekday: Weekday,
        output_dir: Option<&str>,
        filename_template: Option<&str>,
    ) -> ProgramConfig {
        ProgramConfig {
            name: name.to_string(),
            station_id: station_id.to_string(),
            weekday,
            start_time: "00:00".to_string(),
            duration_minutes: 30,
            output_dir: output_dir.map(|s| s.to_string()),
            filename_template: filename_template.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_resolve_path_with_subdir() {
        let settings = make_settings("/tmp/radiko", "{name}_{date}", "m4a");
        let program = make_program("好きもあ", "FMT", Weekday::Sat, Some("sukimore"), None);
        let episode = make_episode("2026-02-14", "00:00", "00:30");

        let path = resolve_output_path(&episode, &program, &settings);
        assert_eq!(path, PathBuf::from("/tmp/radiko/sukimore/好きもあ_20260214.m4a"));
    }

    #[test]
    fn test_resolve_path_without_subdir() {
        let settings = make_settings("/tmp/radiko", "{name}_{date}", "m4a");
        let program = make_program("オールナイトニッポン", "LFR", Weekday::Wed, None, None);
        let episode = make_episode("2026-02-11", "01:00", "03:00");

        let path = resolve_output_path(&episode, &program, &settings);
        assert_eq!(
            path,
            PathBuf::from("/tmp/radiko/オールナイトニッポン_20260211.m4a")
        );
    }

    #[test]
    fn test_program_filename_template_overrides_settings() {
        let settings = make_settings("/tmp/radiko", "{name}_{date}", "m4a");
        let program = make_program(
            "好きもあ",
            "FMT",
            Weekday::Sat,
            Some("sukimore"),
            Some("sukimore_{date}"),
        );
        let episode = make_episode("2026-02-14", "00:00", "00:30");

        let path = resolve_output_path(&episode, &program, &settings);
        assert_eq!(
            path,
            PathBuf::from("/tmp/radiko/sukimore/sukimore_20260214.m4a")
        );
    }

    #[test]
    fn test_template_variables() {
        let settings = make_settings("/tmp", "{name}_{station}_{weekday}_{datetime}", "m4a");
        let program = make_program("テスト", "TBS", Weekday::Mon, None, None);
        let episode = make_episode("2026-02-16", "10:00", "11:00");

        let path = resolve_output_path(&episode, &program, &settings);
        assert_eq!(
            path,
            PathBuf::from("/tmp/テスト_TBS_mon_20260216100000.m4a")
        );
    }

    #[test]
    fn test_default_output_dir() {
        let settings = make_settings("./archive", "{name}_{date}", "m4a");
        let program = make_program("テスト", "TBS", Weekday::Mon, None, None);
        let episode = make_episode("2026-02-16", "10:00", "11:00");

        let path = resolve_output_path(&episode, &program, &settings);
        assert_eq!(path, PathBuf::from("./archive/テスト_20260216.m4a"));
    }

    #[test]
    fn test_is_already_recorded() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("existing.m4a");
        std::fs::write(&existing, "dummy").unwrap();
        let missing = dir.path().join("missing.m4a");

        assert!(is_already_recorded(&existing));
        assert!(!is_already_recorded(&missing));
    }
}
