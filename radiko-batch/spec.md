# radiko-batch 仕様書

## 概要

radiko のタイムフリー機能を利用し、設定ファイルに登録した番組を一括録音する CLI ツール。

## CLI

```bash
radiko-batch --config <PATH> [--dry-run]
```

| オプション | 必須 | 説明 |
|---|---|---|
| `--config <PATH>` | Yes | TOML 設定ファイルのパス |
| `--dry-run` | No | 録音せず、対象エピソードの一覧のみ表示 |

## 設定ファイル (TOML)

### `[settings]` — グローバル設定

| キー | 必須 | デフォルト | 説明 |
|---|---|---|---|
| `output_dir` | No | `"./archive"` | ベース出力ディレクトリ。`~` はホームに展開される |
| `filename_template` | No | `"{name}_{date}"` | ファイル名テンプレート（拡張子なし） |
| `format` | No | `"m4a"` | 出力フォーマット（拡張子として使用） |

### `[[programs]]` — 番組ごとの設定

| キー | 必須 | デフォルト | 説明 |
|---|---|---|---|
| `name` | Yes | — | 番組名（テンプレート変数 `{name}` に使われる） |
| `station_id` | Yes | — | radiko の放送局 ID（例: `FMT`, `LFR`, `TBS`, `QRR`） |
| `weekday` | Yes | — | 放送曜日。`mon` `tue` `wed` `thu` `fri` `sat` `sun` |
| `start_time` | Yes | — | 開始時刻 `HH:MM`（JST、実際のカレンダー日付基準） |
| `duration_minutes` | Yes | — | 放送時間（分） |
| `output_dir` | No | — | `settings.output_dir` 配下のサブディレクトリ名。省略時はベース直下 |
| `filename_template` | No | — | 番組固有のファイル名テンプレート。設定時は `settings.filename_template` を上書き |

### テンプレート変数

ファイル名テンプレート内で使用できる変数:

| 変数 | 展開例 | 説明 |
|---|---|---|
| `{date}` | `20260214` | 放送日 (YYYYMMDD) |
| `{datetime}` | `20260214000000` | 放送開始日時 (YYYYMMDDHHmmss) |
| `{station}` | `FMT` | 放送局 ID |
| `{name}` | `好きもあ` | 番組名 |
| `{weekday}` | `sat` | 放送曜日 |

## 出力パス解決

最終的なファイルパスは以下のように決まる:

```
<settings.output_dir> / [programs.output_dir] / <filename_template>.<format>
```

- `settings.output_dir`: ベースディレクトリ（デフォルト `./archive`）
- `programs.output_dir`: ベース配下のサブディレクトリ（省略時はベース直下）
- `filename_template`: `programs.filename_template` があればそれを使い、なければ `settings.filename_template` を使う
- `format`: `settings.format`（番組ごとの上書きはない）
- パス中の `~` はホームディレクトリに展開される

### 例

```toml
[settings]
output_dir = "~/radiko"
filename_template = "{name}_{date}"
format = "m4a"

[[programs]]
name = "好きもあ"
station_id = "FMT"
weekday = "sat"
start_time = "00:00"
duration_minutes = 30
output_dir = "sukimore"            # → ~/radiko/sukimore/ 配下に出力
filename_template = "sukimore_{date}"

[[programs]]
name = "オールナイトニッポン"
station_id = "LFR"
weekday = "wed"
start_time = "01:00"
duration_minutes = 120
# output_dir 省略 → ~/radiko/ 直下に出力
```

| 番組 | 放送日 | 出力パス |
|---|---|---|
| 好きもあ | 2026-02-14 | `~/radiko/sukimore/sukimore_20260214.m4a` |
| オールナイトニッポン | 2026-02-11 | `~/radiko/オールナイトニッポン_20260211.m4a` |

## 深夜番組の規約

weekday と start_time は**実際のカレンダー日付**を使う（radiko の 5 時区切りではない）。

| 番組表上の表記 | 設定値 |
|---|---|
| 火曜 25:00 | `weekday = "wed"`, `start_time = "01:00"` |
| 土曜 25:00 | `weekday = "sun"`, `start_time = "01:00"` |
| 金曜 24:30 | `weekday = "sat"`, `start_time = "00:30"` |

## スケジュール計算

実行時、各番組について過去 7 日間（radiko タイムフリーの有効期間）を走査し、録音対象エピソードを算出する。

### ルール

1. 現在時刻から 7 日前の日付〜今日の日付を日ごとに走査する
2. 該当する `weekday` の日にエピソードを生成する
3. **放送が終了しているエピソードのみ**対象にする（`start_time + duration_minutes <= now`）

### 例: 2026-02-17 (火) 12:00 に実行した場合

走査範囲: 2/10 (火) 〜 2/17 (火)

| 番組 | weekday | 該当日 | 対象? |
|---|---|---|---|
| 好きもあ | sat | 2/14 (土) | Yes — 放送済み |
| オールナイトニッポン | wed | 2/11 (水) | Yes — 放送済み |

## 録音済み判定

出力パスにファイルが既に存在する場合、そのエピソードは**スキップ**する。

## 録音パイプライン

1. radiko API でタイムフリーストリーム URL を取得
2. HLS マスタープレイリスト → メディアリストを取得
3. 放送時間を 5 分チャンクに分割し、セグメントを重複なく収集
4. 結合した m3u8 を一時ファイルに書き出し
5. ffmpeg で m4a に変換（一時ファイル名で出力）
6. 変換完了後に最終パスへリネーム（途中失敗時は不完全ファイルが残らない）

## 実行時の出力

```
[SKIP] 好きもあ (20260214) - 録音済み: ~/radiko/sukimore/sukimore_20260214.m4a
[REC]  オールナイトニッポン (20260211) -> ~/radiko/オールナイトニッポン_20260211.m4a
[DONE] オールナイトニッポン (20260211) ~/radiko/オールナイトニッポン_20260211.m4a
```

`--dry-run` 時は `[REC]` の代わりに `[DRY]` を表示し、実際の録音は行わない。

## 最小設定

`[settings]` の全フィールドにデフォルト値があるため、最小構成は以下の通り:

```toml
[settings]

[[programs]]
name = "テスト番組"
station_id = "TBS"
weekday = "mon"
start_time = "10:00"
duration_minutes = 60
```

この場合:
- 出力先: `./archive/テスト番組_20260216.m4a`
