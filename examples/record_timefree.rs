use clap::Parser;
use radiko_rs::radiko::Radiko;
use std::collections::HashSet;
use std::process::Command;

/// radikoタイムフリー録音ツール
#[derive(Parser)]
#[command(name = "record_timefree")]
struct Args {
    /// 放送局ID（例: FMT, TBS, QRR）
    #[arg(long)]
    station_id: String,

    /// 開始日時 YYYYMMDDHHmmss形式（例: 20260215000000）
    #[arg(long)]
    ft: String,

    /// 終了日時 YYYYMMDDHHmmss形式（例: 20260215003000）
    #[arg(long)]
    to: String,

    /// 出力ファイルパス
    #[arg(short, long)]
    output: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let radiko = Radiko::new().await;
    let auth_token = radiko.auth_token().await;

    let client = reqwest::Client::new();

    // ft〜toの範囲を5分ずつチャンクに分けてセグメントを収集
    let ft_dt = chrono::NaiveDateTime::parse_from_str(&args.ft, "%Y%m%d%H%M%S")?;
    let to_dt = chrono::NaiveDateTime::parse_from_str(&args.to, "%Y%m%d%H%M%S")?;
    let chunk_duration = chrono::Duration::seconds(300);

    let mut all_segments: Vec<(String, String)> = Vec::new(); // (extinf, url)
    let mut seen_urls = HashSet::new();
    let mut header_lines = String::new();
    let mut current_ft = ft_dt;

    while current_ft < to_dt {
        let current_to = (current_ft + chunk_duration).min(to_dt);
        let chunk_ft = current_ft.format("%Y%m%d%H%M%S").to_string();
        let chunk_to = current_to.format("%Y%m%d%H%M%S").to_string();

        let chunk_url = radiko
            .timefree_stream_url(&args.station_id, &chunk_ft, &chunk_to)
            .await?;
        println!("Fetching chunk: {} - {}", chunk_ft, chunk_to);

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
                    "Warning: no medialist URL for chunk {} - {}",
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
        "Total unique segments: {} (~{} seconds)",
        all_segments.len(),
        all_segments.len() * 5
    );

    // メディアリストを組み立てて保存
    let mut full_medialist = header_lines;
    for (extinf, url) in &all_segments {
        full_medialist.push_str(extinf);
        full_medialist.push('\n');
        full_medialist.push_str(url);
        full_medialist.push('\n');
    }
    full_medialist.push_str("#EXT-X-ENDLIST\n");

    let medialist_path = "/tmp/radiko_medialist.m3u8";
    std::fs::write(medialist_path, &full_medialist)?;

    let status = Command::new("ffmpeg")
        .args([
            "-loglevel",
            "info",
            "-protocol_whitelist",
            "file,http,https,tcp,tls,crypto",
            "-allowed_extensions",
            "ALL",
            "-i",
            medialist_path,
            "-acodec",
            "copy",
            "-vn",
            "-bsf:a",
            "aac_adtstoasc",
            "-y",
            &args.output,
        ])
        .status()?;

    if status.success() {
        println!("Recording saved to {}", args.output);
    } else {
        eprintln!("ffmpeg exited with: {}", status);
    }

    Ok(())
}
