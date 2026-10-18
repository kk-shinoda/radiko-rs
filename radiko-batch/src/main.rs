mod commands;
mod config;
mod output;
mod recorder;
mod schedule;

use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::NaiveTime;
use chrono_tz::Asia::Tokyo;
use clap::{Parser, Subcommand};
use radiko_rs::radiko::Radiko;

/// radiko タイムフリー一括録音ツール
#[derive(Parser)]
#[command(name = "radiko-batch", version)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// 設定ファイルのパス（サブコマンド省略時に使用する従来の一括録音モード）
    #[arg(long)]
    config: Option<PathBuf>,

    /// 録音せず対象エピソードの一覧のみ表示（従来の一括録音モードのみ）
    #[arg(long)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Command {
    /// 単発タイムフリー録音
    Once {
        /// 放送局ID（例: FMT, TBS, QRR）
        #[arg(long)]
        station_id: String,
        /// 開始日時 YYYYMMDDHHMMSS
        #[arg(long)]
        ft: String,
        /// 終了日時 YYYYMMDDHHMMSS
        #[arg(long)]
        to: String,
        /// 出力ファイルパス（.m4a）
        #[arg(long)]
        output: PathBuf,
    },
    /// 番組検索
    Search {
        /// 検索キーワード
        #[arg(long)]
        key: String,
        /// タイムフリー（過去）番組のみに絞る
        #[arg(long)]
        past: bool,
        /// 今後の番組（放送予定）のみに絞る
        #[arg(long)]
        future: bool,
        /// JSON で出力する
        #[arg(long)]
        json: bool,
    },
    /// 局の番組表
    Programs {
        /// 放送局ID
        #[arg(long)]
        station_id: String,
        /// 対象日 YYYYMMDD（省略時は週間番組表全体）
        #[arg(long)]
        date: Option<String>,
        /// JSON で出力する
        #[arg(long)]
        json: bool,
    },
    /// 現在エリアで聴ける局一覧
    Stations {
        /// JSON で出力する
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Command::Once {
            station_id,
            ft,
            to,
            output,
        }) => commands::run_once(&station_id, &ft, &to, &output).await,
        Some(Command::Search {
            key,
            past,
            future,
            json,
        }) => commands::run_search(&key, past, future, json).await,
        Some(Command::Programs {
            station_id,
            date,
            json,
        }) => commands::run_programs(&station_id, date.as_deref(), json).await,
        Some(Command::Stations { json }) => commands::run_stations(json).await,
        None => {
            let config_path = args
                .config
                .ok_or_else(|| anyhow::anyhow!("--config が必要です（サブコマンド未指定時）"))?;
            run_batch(&config_path, args.dry_run).await
        }
    }
}

/// 従来の設定ファイルベースの一括録音モード。
async fn run_batch(config_path: &Path, dry_run: bool) -> Result<()> {
    let cfg = config::load_config(config_path)?;

    let now_jst = chrono::Utc::now().with_timezone(&Tokyo).naive_local();

    // ベースディレクトリをチルダ展開
    let base_output_dir = shellexpand::tilde(&cfg.settings.output_dir).to_string();
    let settings_expanded = config::Settings {
        output_dir: base_output_dir,
        filename_template: cfg.settings.filename_template,
        format: cfg.settings.format,
    };

    let radiko = if !dry_run {
        Some(Radiko::new().await)
    } else {
        None
    };

    for program in &cfg.programs {
        let start_time = NaiveTime::parse_from_str(&program.start_time, "%H:%M")?;
        let episodes = schedule::find_episodes_in_timefree_window(
            program.weekday,
            start_time,
            program.duration_minutes,
            now_jst,
        );

        for episode in &episodes {
            let date_str = episode.date.format("%Y%m%d").to_string();
            let path = output::resolve_output_path(episode, program, &settings_expanded);

            // チルダ展開（program.output_dir に ~ がある場合）
            let path = PathBuf::from(shellexpand::tilde(&path.to_string_lossy()).to_string());

            if output::is_already_recorded(&path) {
                println!(
                    "[SKIP] {} ({}) - 録音済み: {}",
                    program.name,
                    date_str,
                    path.display()
                );
                continue;
            }

            if dry_run {
                println!(
                    "[DRY]  {} ({}) -> {}",
                    program.name,
                    date_str,
                    path.display()
                );
                continue;
            }

            println!(
                "[REC]  {} ({}) -> {}",
                program.name,
                date_str,
                path.display()
            );

            match recorder::record_episode(
                radiko.as_ref().unwrap(),
                &program.station_id,
                episode,
                &path,
            )
            .await
            {
                Ok(()) => {
                    println!("[DONE] {} ({}) {}", program.name, date_str, path.display());
                }
                Err(e) => {
                    eprintln!(
                        "[FAIL] {} ({}) {}: {}",
                        program.name,
                        date_str,
                        path.display(),
                        e
                    );
                }
            }
        }
    }

    Ok(())
}
