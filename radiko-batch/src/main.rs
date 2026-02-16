mod config;
mod output;
mod recorder;
mod schedule;

use std::path::PathBuf;

use anyhow::Result;
use chrono::NaiveTime;
use chrono_tz::Asia::Tokyo;
use clap::Parser;
use radiko_rs::radiko::Radiko;

/// radiko タイムフリー一括録音ツール
#[derive(Parser)]
#[command(name = "radiko-batch")]
struct Args {
    /// 設定ファイルのパス
    #[arg(long)]
    config: PathBuf,

    /// 録音せず対象エピソードの一覧のみ表示
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let cfg = config::load_config(&args.config)?;

    let now_jst = chrono::Utc::now()
        .with_timezone(&Tokyo)
        .naive_local();

    // ベースディレクトリをチルダ展開
    let base_output_dir = shellexpand::tilde(&cfg.settings.output_dir).to_string();
    let settings_expanded = config::Settings {
        output_dir: base_output_dir,
        filename_template: cfg.settings.filename_template,
        format: cfg.settings.format,
    };

    let radiko = if !args.dry_run {
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

            if args.dry_run {
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
                    println!(
                        "[DONE] {} ({}) {}",
                        program.name,
                        date_str,
                        path.display()
                    );
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
