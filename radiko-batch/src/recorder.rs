use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use chrono::TimeDelta;
use radiko_rs::radiko::Radiko;

use crate::schedule::Episode;

/// 一時ファイルに録音し、完了後に最終パスへリネームする。
/// 途中失敗時は不完全ファイルが残らないようにする。
pub async fn record_episode(
    radiko: &Radiko,
    station_id: &str,
    episode: &Episode,
    output_path: &Path,
) -> Result<()> {
    // 出力ディレクトリを作成
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // 一時ファイルパス
    let tmp_path = output_path.with_extension("tmp.m4a");

    let auth_token = radiko.auth_token().await;
    let client = reqwest::Client::new();

    // 5分チャンクでセグメントを収集
    let chunk_duration = TimeDelta::seconds(300);
    let mut all_segments: Vec<(String, String)> = Vec::new();
    let mut seen_urls = HashSet::new();
    let mut header_lines = String::new();
    let mut current_ft = episode.start;

    while current_ft < episode.end {
        let current_to = (current_ft + chunk_duration).min(episode.end);
        let chunk_ft = current_ft.format("%Y%m%d%H%M%S").to_string();
        let chunk_to = current_to.format("%Y%m%d%H%M%S").to_string();

        let chunk_url = radiko
            .timefree_stream_url(station_id, &chunk_ft, &chunk_to)
            .await?;
        println!("  Fetching chunk: {} - {}", chunk_ft, chunk_to);

        // マスタープレイリスト取得
        let master = client
            .get(&chunk_url)
            .header("X-Radiko-Authtoken", auth_token.as_str())
            .send()
            .await?
            .text()
            .await?;

        let medialist_url = match master.lines().find(|l| l.starts_with("https://")) {
            Some(url) => url.to_string(),
            None => {
                eprintln!(
                    "  Warning: no medialist URL for chunk {} - {}",
                    chunk_ft, chunk_to
                );
                current_ft = current_to;
                continue;
            }
        };

        // メディアリスト取得
        let medialist = client
            .get(&medialist_url)
            .header("X-Radiko-Authtoken", auth_token.as_str())
            .send()
            .await?
            .text()
            .await?;

        // 最初のチャンクからヘッダーを保存
        if header_lines.is_empty() {
            for line in medialist.lines() {
                if line.starts_with("#EXTM3U")
                    || line.starts_with("#EXT-X-VERSION")
                    || line.starts_with("#EXT-X-TARGETDURATION")
                    || line.starts_with("#EXT-X-MEDIA-SEQUENCE")
                {
                    header_lines.push_str(line);
                    header_lines.push('\n');
                }
            }
        }

        // セグメントを抽出（重複除去）
        let lines: Vec<&str> = medialist.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            if lines[i].starts_with("#EXTINF") && i + 1 < lines.len() {
                let url = lines[i + 1].to_string();
                if url.starts_with("https://") && seen_urls.insert(url.clone()) {
                    all_segments.push((lines[i].to_string(), url));
                }
                i += 2;
            } else {
                i += 1;
            }
        }

        current_ft = current_to;
    }

    println!(
        "  Total unique segments: {} (~{} seconds)",
        all_segments.len(),
        all_segments.len() * 5
    );

    // メディアリストを組み立てて一時ファイルに保存
    let mut full_medialist = header_lines;
    for (extinf, url) in &all_segments {
        full_medialist.push_str(extinf);
        full_medialist.push('\n');
        full_medialist.push_str(url);
        full_medialist.push('\n');
    }
    full_medialist.push_str("#EXT-X-ENDLIST\n");

    let m3u8_path = tmp_path.with_extension("m3u8");
    std::fs::write(&m3u8_path, &full_medialist)?;

    // ffmpeg で変換
    let status = Command::new("ffmpeg")
        .args([
            "-loglevel",
            "warning",
            "-protocol_whitelist",
            "file,http,https,tcp,tls,crypto",
            "-allowed_extensions",
            "ALL",
            "-i",
            &m3u8_path.to_string_lossy(),
            "-acodec",
            "copy",
            "-vn",
            "-bsf:a",
            "aac_adtstoasc",
            "-y",
            &tmp_path.to_string_lossy(),
        ])
        .status()?;

    // m3u8 一時ファイルを削除
    let _ = std::fs::remove_file(&m3u8_path);

    if !status.success() {
        let _ = std::fs::remove_file(&tmp_path);
        bail!("ffmpeg exited with: {}", status);
    }

    // 完了後に最終パスへリネーム
    std::fs::rename(&tmp_path, output_path).with_context(|| {
        format!(
            "Failed to rename {} -> {}",
            tmp_path.display(),
            output_path.display()
        )
    })?;

    Ok(())
}
