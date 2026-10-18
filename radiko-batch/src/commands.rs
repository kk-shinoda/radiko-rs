use std::path::Path;

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use radiko_rs::models::program::Program;
use radiko_rs::models::search::{Filter, SearchCondition};
use radiko_rs::radiko::Radiko;
use serde::Serialize;
use serde_json::json;

use crate::recorder;
use crate::schedule::Episode;

const FT_FORMAT: &str = "%Y%m%d%H%M%S";

/// benry などの呼び出し元向けに整形した番組情報。
#[derive(Debug, Serialize)]
pub struct ProgramJson {
    pub station_id: String,
    pub title: String,
    pub ft: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

impl From<&Program> for ProgramJson {
    fn from(p: &Program) -> Self {
        // info が空なら description を使う
        let info = non_empty(&p.info).or_else(|| non_empty(&p.description));
        ProgramJson {
            station_id: p.station_id.clone(),
            title: p.title.clone(),
            ft: p.start_time.format(FT_FORMAT).to_string(),
            to: p.end_time.format(FT_FORMAT).to_string(),
            performer: non_empty(&p.performer),
            info,
            url: non_empty(&p.program_url),
        }
    }
}

/// `radiko-batch once` — 単発タイムフリー録音。
pub async fn run_once(station_id: &str, ft: &str, to: &str, output: &Path) -> Result<()> {
    let result = run_once_inner(station_id, ft, to, output).await;

    match result {
        Ok(duration_sec) => {
            println!(
                "{}",
                json!({
                    "ok": true,
                    "output": output.to_string_lossy(),
                    "duration_sec": duration_sec,
                })
            );
            Ok(())
        }
        Err(e) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "error": e.to_string(),
                })
            );
            std::process::exit(1);
        }
    }
}

async fn run_once_inner(station_id: &str, ft: &str, to: &str, output: &Path) -> Result<i64> {
    let start = NaiveDateTime::parse_from_str(ft, FT_FORMAT)
        .with_context(|| format!("--ft の形式が不正です: {}", ft))?;
    let end = NaiveDateTime::parse_from_str(to, FT_FORMAT)
        .with_context(|| format!("--to の形式が不正です: {}", to))?;
    anyhow::ensure!(start < end, "--ft は --to より前である必要があります");

    let episode = Episode {
        start,
        end,
        date: start.date(),
    };

    eprintln!(
        "[REC] {} {} -> {} -> {}",
        station_id,
        ft,
        to,
        output.display()
    );
    let radiko = Radiko::new().await;
    recorder::record_episode(&radiko, station_id, &episode, output).await?;
    eprintln!("[DONE] {}", output.display());

    Ok((end - start).num_seconds())
}

/// `radiko-batch search` — 番組検索。
pub async fn run_search(key: &str, past: bool, future: bool, json_output: bool) -> Result<()> {
    anyhow::ensure!(
        !(past && future),
        "--past と --future は同時に指定できません"
    );

    let filter = if past {
        Some(Filter::TimeFree)
    } else if future {
        Some(Filter::Live)
    } else {
        Some(Filter::All)
    };

    let condition = SearchCondition {
        key: vec![key.to_string()],
        filter,
        ..Default::default()
    };

    let radiko = Radiko::new().await;
    let programs = radiko.find_program(&condition).await?;

    print_programs(&programs.data, json_output);
    Ok(())
}

/// `radiko-batch programs` — 局の番組表。
pub async fn run_programs(station_id: &str, date: Option<&str>, json_output: bool) -> Result<()> {
    let radiko = Radiko::new().await;
    let programs = radiko.weekly_programs_from_station(station_id).await?;

    let filtered: Vec<Program> = match date {
        Some(date_str) => {
            eprintln!(
                "[INFO] --date は週間番組表から該当日のみを絞り込みます（指定日が週間表の範囲外の場合は結果が空になります）"
            );
            programs
                .data
                .into_iter()
                .filter(|p| p.start_time.format("%Y%m%d").to_string() == date_str)
                .collect()
        }
        None => programs.data,
    };

    print_programs(&filtered, json_output);
    Ok(())
}

/// `radiko-batch stations` — 現在エリアの局一覧。
pub async fn run_stations(json_output: bool) -> Result<()> {
    let radiko = Radiko::new().await;
    let area_id = radiko.area_id().await;
    let stations = radiko.stations_from_area_id(&area_id).await?;

    if json_output {
        println!("{}", serde_json::to_string(&stations.data)?);
    } else {
        for s in &stations.data {
            println!("{} {}", s.id, s.name);
        }
    }
    Ok(())
}

fn print_programs(programs: &[Program], json_output: bool) {
    if json_output {
        let items: Vec<ProgramJson> = programs.iter().map(ProgramJson::from).collect();
        match serde_json::to_string(&items) {
            Ok(s) => println!("{}", s),
            Err(e) => eprintln!("[FAIL] JSON出力に失敗しました: {}", e),
        }
    } else {
        for p in programs {
            println!(
                "{} {}-{} {}",
                p.station_id,
                p.start_time.format(FT_FORMAT),
                p.end_time.format(FT_FORMAT),
                p.title
            );
        }
    }
}
