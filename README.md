# WIP

# radiko-rs

日本のインターネットラジオサービス「radiko」へのアクセスを提供する非公式Rustライブラリです。

## 特徴

- 2段階認証フローによるradiko APIの認証
- 認証トークンを用いたHLSストリーミングURLへのアクセス
- 番組情報と番組表の取得
- 番組検索機能
- タイムゾーン対応の日時処理

## インストール

`Cargo.toml`に以下を追加してください：

```toml
[dependencies]
radiko-rs = "0.1.0"
```

## 使用方法

```rust
use radiko_rs::{RadikoClient, RadikoAuthManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 認証済みクライアントを作成
    let auth_manager = RadikoAuthManager::new().await?;
    let client = RadikoClient::new(auth_manager).await?;
    
    // radiko APIにアクセス
    // ...
    
    Ok(())
}
```

## 認証について

このライブラリはradikoの2段階認証プロセスを自動的に処理します：

1. `auth1` APIからauthトークンとキー情報を取得
2. `playerCommon.js`から認証キーを抽出
3. `auth2` APIに部分キーを送信して認証を完了
4. `X-Radiko-Authtoken`ヘッダー付きのHTTPクライアントを作成

## タイムフリー録音

`examples/record_timefree.rs` でタイムフリー番組の録音ができます（要ffmpeg）。

```bash
cargo run --example record_timefree -- \
  --station-id FMT \
  --ft 20260215000000 \
  --to 20260215003000 \
  -o output.m4a
```

| 引数 | 説明 | 例 |
|---|---|---|
| `--station-id` | 放送局ID | FMT, TBS, QRR |
| `--ft` | 開始日時 (YYYYMMDDHHmmss) | 20260215000000 |
| `--to` | 終了日時 (YYYYMMDDHHmmss) | 20260215003000 |
| `-o` / `--output` | 出力ファイルパス | output.m4a |

## radiko-batch: タイムフリー一括録音ツール

設定ファイルに番組を登録しておき、コマンド1つでタイムフリー期間内かつ未録音のエピソードを自動録音する CLI ツールです（要 ffmpeg）。

### セットアップ

```bash
# サンプル設定をコピーして編集
cp radiko-batch/sample_config.toml ~/.config/radiko-batch/config.toml
```

### 設定例

```toml
[settings]
output_dir = "~/radiko"        # ベース出力ディレクトリ（省略時: ./archive）
filename_template = "{name}_{date}"
format = "m4a"

[[programs]]
name = "すきもあ"
station_id = "FMT"
weekday = "sat"
start_time = "00:00"
duration_minutes = 30
output_dir = "sukimore"        # → ~/radiko/sukimore/ 配下に出力
filename_template = "sukimore_{date}"

[[programs]]
name = "オールナイトニッポン"
station_id = "LFR"
weekday = "wed"                # 火曜深夜25時 = 水曜01:00
start_time = "01:00"
duration_minutes = 120
```

### 実行

```bash
# ドライラン（録音対象の確認のみ）
cargo run -p radiko-batch -- --config .config/radiko-batch/config.toml --dry-run

# 録音実行
cargo run -p radiko-batch -- --config .config/radiko-batch/config.toml
```

### cron で定期実行

```bash
# 毎日 12:00 に実行（タイムフリー窓は過去7日間）
0 12 * * * cd /path/to/radiko-rs && cargo run -p radiko-batch -- --config ~/.config/radiko-batch/config.toml
```

詳細な仕様は [radiko-batch/spec.md](radiko-batch/spec.md) を参照してください。

### サブコマンド（外部ツールからの呼び出し用）

設定ファイルを使った一括録音のほかに、単発操作用のサブコマンドがある。いずれも進捗は stderr、結果は stdout への JSON 1行で出力する（`--json` 指定時）。

```bash
# 単発タイムフリー録音
radiko-batch once --station-id LFR --ft 20261018204000 --to 20261018210000 --output ~/radiko/test.m4a
# -> stdout: {"ok":true,"output":"...","duration_sec":1200}
#    失敗時: {"ok":false,"error":"..."} かつ非ゼロ終了

# 番組検索
radiko-batch search --key "オールナイトニッポン" --past --json

# 局の番組表（省略時は週間番組表、--date で当日分のみ絞り込み）
radiko-batch programs --station-id LFR --json
radiko-batch programs --station-id LFR --date 20261018 --json

# 現在エリアで聴ける局一覧
radiko-batch stations --json
```

#### ffmpeg のパス

launchd 等、PATH に Homebrew (`/opt/homebrew/bin`) が含まれない環境から実行する場合は `RADIKO_FFMPEG` 環境変数で ffmpeg の絶対パスを指定する。未設定時は PATH 上の `ffmpeg` を使用する。

```bash
RADIKO_FFMPEG=/opt/homebrew/bin/ffmpeg radiko-batch once --station-id LFR --ft ... --to ... --output ...
```

## テスト

```bash
cargo test
```

### テスト実行時の注意

- テストの多くは実際のradiko APIへのネットワークアクセスを行います
- タイムフリー関連テストはデフォルトで放送局 **FMT**（TOKYO FM）を使用します
- FMTのエリア外から実行する場合は、受信可能な放送局IDを環境変数で指定してください：

```bash
RADIKO_TEST_STATION_ID=MBS cargo test timefree
```

## ライセンス

このプロジェクトは以下のいずれかのライセンスで提供されます：

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) または http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) または http://opensource.org/licenses/MIT)

お好みの方をお選びください。

## 免責事項

このライブラリは教育目的および個人利用のみを目的としています。radikoの利用規約を尊重し、責任を持ってこのライブラリをご利用ください。