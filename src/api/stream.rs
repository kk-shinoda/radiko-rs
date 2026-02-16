use std::{borrow::Cow, convert::TryFrom, io::Write, sync::Arc};

use anyhow::{Result, anyhow};
use hls_m3u8::MasterPlaylist;
use tempfile::NamedTempFile;

use super::{auth::RadikoAuthManager, endpoint::RadikoEndpoint};

pub struct RadikoStream {
    inner: Arc<RadikoStreamRef>,
}

struct RadikoStreamRef {
    auth_manager: Arc<RadikoAuthManager>,
}

impl RadikoStream {
    pub fn new(radiko_auth_manager: Arc<RadikoAuthManager>) -> Self {
        Self {
            inner: Arc::new(RadikoStreamRef {
                auth_manager: radiko_auth_manager.clone(),
            }),
        }
    }

    pub fn stream_url(&self, station_id: &str) -> String {
        let lsid = &self.inner.auth_manager.lsid().to_string();
        if self.inner.auth_manager.area_free() {
            RadikoEndpoint::area_free_playlist_create_url_endpoint(station_id, &lsid)
        } else {
            RadikoEndpoint::playlist_create_url_endpoint(station_id, &lsid)
        }
    }

    #[allow(dead_code)]
    pub async fn get_hls_master_playlist_content(&self, station_id: &str) -> Result<Cow<str>> {
        let master_playlist_res = self
            .inner
            .auth_manager
            .http_client()
            .get(&self.stream_url(station_id))
            .send()
            .await?;

        if !master_playlist_res.status().is_success() {
            return Err(anyhow!(
                "get hls master playlist error: {:#?}, client_info: {:#?}",
                master_playlist_res.text().await?,
                self.inner.auth_manager.http_client()
            ));
        }

        Ok(master_playlist_res.text().await?.into())
    }

    #[allow(dead_code)]
    pub fn extract_medialist_url(&self, master_playlist_content: &str) -> Result<Cow<str>> {
        let master_playlist = match MasterPlaylist::try_from(master_playlist_content) {
            Ok(master_playlist) => master_playlist,
            Err(err) => panic!(
                "extract_medialist_url error: {:#?},master_playlist_content: {:#?}",
                err, master_playlist_content
            ),
        };
        Ok(master_playlist
            .variant_streams
            .iter()
            .next()
            .ok_or_else(|| anyhow!("No stream found in master playlist"))
            .and_then(|stream| match stream {
                hls_m3u8::tags::VariantStream::ExtXStreamInf { uri, .. } => Ok(uri.to_string()),
                _ => Err(anyhow!("Invalid stream type")),
            })?
            .into())
    }

    #[allow(dead_code)]
    pub async fn download_playlist_to_tempfile(&self, station_id: &str) -> Result<NamedTempFile> {
        let playlist_content = self
            .inner
            .auth_manager
            .http_client()
            .get(&self.stream_url(station_id))
            .send()
            .await?
            .bytes()
            .await?;

        let mut temp_file = NamedTempFile::with_suffix(".m3u8")?;
        temp_file.write_all(&playlist_content)?;
        temp_file.flush()?;

        Ok(temp_file)
    }
}

#[cfg(test)]
mod tests {
    use crate::api::auth::RadikoAuthManager;
    use crate::utils::load_env;
    use crate::{api::stream::RadikoStream, radiko::Radiko};
    use std::{env, process::Stdio};

    use anyhow::Result;

    use tokio::{
        io::{AsyncBufReadExt, BufReader},
        process::Command,
    };

    /// タイムフリーURLから実際にオーディオセグメントをダウンロードし、
    /// 有効なAACデータであることを検証する統合テスト。
    ///
    /// 環境変数 RADIKO_TEST_STATION_ID で放送局を指定可能（デフォルト: FMT）。
    /// FMTのエリア外で実行する場合は受信可能な放送局IDを指定すること。
    #[tokio::test]
    async fn timefree_stream_url_should_return_downloadable_audio() -> Result<()> {
        let radiko = Radiko::new().await;
        let token = radiko.auth_token().await;
        let station_id =
            env::var("RADIKO_TEST_STATION_ID").unwrap_or_else(|_| "FMT".to_string());

        // 2日前の00:00〜00:30を対象（タイムフリーは過去1週間まで有効）
        let two_days_ago = chrono::Local::now() - chrono::Duration::days(2);
        let base_date = two_days_ago.date_naive();
        let ft = base_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .format("%Y%m%d%H%M%S")
            .to_string();
        let to = base_date
            .and_hms_opt(0, 30, 0)
            .unwrap()
            .format("%Y%m%d%H%M%S")
            .to_string();

        // タイムフリーURL取得 → マスタープレイリスト → メディアプレイリスト → セグメント
        let timefree_url = radiko
            .timefree_stream_url(&station_id, &ft, &to)
            .await?;

        let client = reqwest::Client::new();
        let master = client
            .get(&timefree_url)
            .header("X-Radiko-Authtoken", &token)
            .send()
            .await?
            .text()
            .await?;

        let media_url = master
            .lines()
            .find(|l| l.starts_with("https://"))
            .expect("マスタープレイリストにメディアURLが含まれていない");

        let media = client
            .get(media_url)
            .header("X-Radiko-Authtoken", &token)
            .send()
            .await?
            .text()
            .await?;

        let segment_url = media
            .lines()
            .find(|l| l.starts_with("https://"))
            .expect("メディアプレイリストにセグメントURLが含まれていない");

        let segment_bytes = client.get(segment_url).send().await?.bytes().await?;

        // tempfileに保存して検証（スコープ抜けで自動削除）
        let temp_file = tempfile::NamedTempFile::with_suffix(".aac")?;
        std::fs::write(temp_file.path(), &segment_bytes)?;

        let file_size = temp_file.path().metadata()?.len();
        assert!(
            file_size > 100,
            "音声ファイルが小さすぎる: {} bytes",
            file_size
        );

        // AACデータの検証: ID3タグ(0x49 0x44 0x33) または ADTSヘッダ(0xFF 0xF?)
        assert!(
            segment_bytes.starts_with(b"ID3")
                || (segment_bytes[0] == 0xFF && segment_bytes[1] & 0xF0 == 0xF0),
            "有効なAACオーディオデータではない (先頭2バイト: {:02X} {:02X})",
            segment_bytes[0],
            segment_bytes[1]
        );

        Ok(())
    }

    #[tokio::test]
    async fn hls_m3u8_playground() -> Result<()> {
        let station_id = "TBS";
        let radiko_stream = RadikoStream::new(RadikoAuthManager::new().await.into());

        let master_playlist_content = radiko_stream
            .get_hls_master_playlist_content(station_id)
            .await?;
        let segment_uri = radiko_stream.extract_medialist_url(&master_playlist_content)?;

        println!("parsed_uri: {}", segment_uri);

        Ok(())
    }

    #[tokio::test]
    async fn stream_url_test() -> Result<()> {
        let radiko = Radiko::new().await;
        let available_stations = radiko
            .stations_from_area_id(&radiko.area_id().await)
            .await?;
        let station_id = available_stations.data.get(0).unwrap().id.clone();

        run_ffmpeg_command_stream(radiko, &station_id).await?;

        Ok(())
    }

    #[tokio::test]
    async fn area_free_stream_url_test() -> Result<()> {
        load_env();
        let mail = env::var("mail").expect("failed mail from dotenv");
        let pass = env::var("pass").expect("failed pass from dotenv");
        let station_id = "MBS";
        let radiko = Radiko::new_area_free(&mail, &pass).await;

        println!("station_id: {}", station_id);

        run_ffmpeg_command_stream(radiko, station_id).await?;

        Ok(())
    }

    async fn run_ffmpeg_command_stream(radiko: Radiko, station_id: &str) -> Result<()> {
        let strem_url = radiko.stream_url(station_id).await;
        let token = radiko.auth_token().await.to_string();

        let cmd = Command::new("ffmpeg")
            .args([
                "-loglevel",
                "debug",
                "-protocol_whitelist",
                "file,http,https,tcp,tls,crypto",
                "-headers",
                &format!("X-Radiko-Authtoken: {}\r\n", token),
                "-allowed_extensions",
                "ALL",
                "-seekable",
                "0",
                "-http_seekable",
                "0",
                "-f",
                "hls",
                "-i",
                &strem_url,
                "-reconnect",
                "3",
                "-reconnect_at_eof",
                "1",
                "-reconnect_streamed",
                "1",
                "-reconnect_delay_max",
                "5",
                "-timeout",
                "10000000",
                "-http_persistent",
                "1",
                "-acodec",
                "copy",
                "-vn",
                "-bsf:a",
                "aac_adtstoasc",
                "-t",
                "10",
                "-y",
                "output.aac",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn();

        match cmd {
            Ok(mut child) => {
                let process_id = child.id().unwrap_or(0);
                let stream_id = process_id;

                println!(
                    "FFmpeg process started for stream {} with PID {}",
                    stream_id, process_id
                );

                // stderrをログ出力用に監視
                if let Some(stderr) = child.stderr.take() {
                    let reader = BufReader::new(stderr);
                    let mut lines = reader.lines();
                    let log_stream_id = stream_id.clone();

                    tokio::spawn(async move {
                        while let Ok(Some(line)) = lines.next_line().await {
                            if line.contains("error") || line.contains("Error") {
                                println!("FFmpeg {}: {}", log_stream_id, line);
                            } else if line.contains("frame=") || line.contains("time=") {
                                // プログレス情報は詳細レベルで
                                println!("FFmpeg {}: {}", log_stream_id, line);
                            } else {
                                println!("FFmpeg {}: {}", log_stream_id, line);
                            }
                        }
                    });
                }

                // プロセス終了を待機
                match child.wait().await {
                    Ok(status) => {
                        if status.success() {
                            println!("FFmpeg process for exited successfully");
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!(
                                "FFmpeg process for exited with error: {}",
                                status
                            ))
                        }
                    }
                    Err(e) => Err(anyhow::anyhow!(
                        "Error waiting for FFmpeg process {}: {}",
                        stream_id,
                        e
                    )),
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to start ffmpeg for stream : {}", e)),
        }
    }
}
