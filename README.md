# ds4map

`ds4map` は、DUALSHOCK 4 の入力レポートを読み取り、CLI 上でリアルタイム表示しつつ、必要に応じて `arm9` などの format 向け出力データを生成・送信できる Rust 製のアプリケーションです。

## 主な機能

- DS4 の入力状態をリアルタイムに表示
- `graphic` / `raw` / `compact` / `none` の monitor モード
- シリアルポートへの出力
- `--port` 指定時のシリアル受信データ表示（バイト列と ASCII 文字列）
- `--log-file` 指定時のフレーム単位ログ出力
- `--monitor none` でのバックグラウンド実行
- `status` / `stop` による実行状態の確認と停止

## 動作に必要なもの

- `make`
- Rust ツールチェーン（`make init` で自動インストール可能）
- 接続された DUALSHOCK 4
- macOS では、`arm9` 用のモード切り替え音再生に `afplay` を利用可能な場合があります

## セットアップ

このプロジェクトは、リポジトリを `git clone` してローカルでビルドする運用がいちばん簡単です。

### 1. リポジトリを clone する

```bash
git clone <REPO_URL>
cd ds4map
```

### 2. 初期化する

リポジトリのルートで次を実行してください。

```bash
make init
```

`make init` は次をまとめて実行します。

- `cargo` が見つからない場合は `rustup` で Rust をインストール
- `Cargo.lock` に従って依存関係を取得
- ローカルビルド

すでに構築済みの環境でも、同じコマンドを再実行して構いません。

### 3. 実行する

```bash
./target/debug/ds4 run
```

Cargo 経由でも実行できます。

```bash
cargo run -- run
```

ビルド済みバイナリを直接使うこともできます。

```bash
./target/debug/ds4 run
```

### 4. ソースコードを変更したあとにリポジトリ内のバイナリを更新する

リポジトリ内のバイナリを使う場合は、次を実行してください。

```bash
make build
```

### 5. `ds4` コマンドをグローバルインストールする

グローバルに `ds4` コマンドとして入れたい場合だけ、次を実行してください。

```bash
make install-global
```

ソースコード変更後にグローバルの `ds4` コマンドへ反映したいときも、同じ `make install-global` を使えます。

## ビルド

```bash
cargo build
```

Cargo 経由で実行:

```bash
cargo run -- run
```

ビルド済みバイナリを直接実行:

```bash
./target/debug/ds4 run
```

## クイックスタート

まずは help が出ることを確認してください。

```bash
./target/debug/ds4 --help
```

まだ `make init` をしていない場合は、こちらでも確認できます。

```bash
cargo run -- --help
```

最初によく使うコマンド:

```bash
./target/debug/ds4 devices
./target/debug/ds4 ports
./target/debug/ds4 run
```

グローバルの `ds4` コマンドを入れたいとき:

```bash
make install-global
```

## コマンド一覧

以下は `./target/debug/ds4` を使うか、`make install-global` 後は `ds4` に読み替えてください。

### 接続中の DS4 を表示

```bash
ds4 devices
```

### 出力先候補のシリアルポートを表示

```bash
ds4 ports
```

### モニタを起動

```bash
ds4 run
```

### 実行中の状態を表示

```bash
ds4 status
```

### 実行中の処理を停止

```bash
ds4 stop
```

### help を表示

```bash
ds4 --help
ds4 help run
```

## `run` のオプション

```text
-m, --monitor <graphic|raw|compact|none>
-f, --format <arm9>
-p, --port <PORT>
-b, --baud <BAUD_RATE>
-h, --help
```

### monitor モード

- `graphic`: コントローラ全体をグラフィカルに表示
- `raw`: 生の HID レポートを表示
- `compact`: compact 8 バイトのレポートを表示
- `none`: モニタ表示なし。バックグラウンド実行専用

### よく使う例

通常のグラフィック表示:

```bash
ds4 run
```

生 HID レポート表示:

```bash
ds4 run -m raw
```

compact レポート表示:

```bash
ds4 run -m compact
```

`arm9` 出力を画面下部にプレビュー表示する:

```bash
ds4 run -f arm9
```

`arm9` 出力をシリアルポートへ送信しつつ、graphic 下部に受信データも表示する:

```bash
ds4 run -f arm9 -p /dev/ttyUSB0 -b 115200
```

HID と送受信データをログファイルへ追記する:

```bash
ds4 run -f arm9 -p /dev/ttyUSB0 -b 115200 --log-file logs/ds4.log
```

モニタ表示なしでバックグラウンド実行する:

```bash
ds4 run -m none -f arm9 -p /dev/ttyUSB0 -b 115200
```

## 構成

アプリケーションは大きく次の 4 層に分かれています。

- `src/app`: CLI の入口、コマンド分岐、runtime 管理
- `src/input`: DS4 HID の探索・受信、compact 形式への変換
- `src/output`: シリアル出力と format ごとの出力生成
- `src/ui`: ターミナル上の monitor 描画

format 実装は次の配下にあります。

- `src/output/formats/`

現在の `arm9` 実装は次のように分割しています。

- `src/output/formats/arm9/mod.rs`
- `src/output/formats/arm9/encoder.rs`
- `src/output/formats/arm9/sound.rs`

## 新しい format の追加方法

新しい format を追加するときは、CLI 側を大きく触らずに、`src/output/formats/<name>/` に実装を追加する方針です。

### 1. format 用ディレクトリを作る

例:

```text
src/output/formats/my_format/
  mod.rs
  encoder.rs
```

`mod.rs` では、`Box<dyn OutputDriver>` を返す `create_driver()` を公開します。

### 2. `OutputDriver` を実装する

共通 trait は `src/output/formats/mod.rs` にあります。

```rust
pub trait OutputDriver {
    fn format_name(&self) -> &'static str;
    fn encode(&mut self, compact_report: &CompactReport) -> Result<Vec<u8>, String>;
}
```

各 driver は次を担当します。

- 必要な内部状態を保持する
- `CompactReport` を format 固有の bytes に変換する
- プレビュー表示やシリアル送信に使う `Vec<u8>` を返す

### 3. format を登録する

`src/output/formats/mod.rs` を更新して、次を追加します。

- `OutputFormat` の enum variant
- `OUTPUT_FORMATS` への登録

この登録により、以下が有効になります。

- `--format <name>` の解釈
- 画面表示用の format 名
- driver の生成

### 4. 動作確認する

追加後は、最低限これを実行してください。

```bash
cargo fmt
cargo check
cargo test
```

## runtime の挙動

- `ds4 run -m none ...` はバックグラウンドで起動します
- `ds4 status` で現在の実行状態を確認できます
- `ds4 stop` で停止要求を送れます
- フォアグラウンドの monitor は `Ctrl-C` で停止でき、終了時に元の端末画面へ戻ります

## ドキュメント

追加の技術資料は `docs/` にまとめています。

- [DS4_HID_REPORT_SPEC.md](docs/DS4_HID_REPORT_SPEC.md)
- [COMPACT_SPEC.md](docs/COMPACT_SPEC.md)
- [ARM9_ACV6_PACKET_AND_CAN.md](docs/ARM9_ACV6_PACKET_AND_CAN.md)
