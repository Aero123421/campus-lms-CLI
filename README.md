# campus-lms-cli

`campus-lms-cli` は、大学の Moodle 互換 LMS を CLI から参照するための
**読み取り専用向けツール**です。  
ユーザー本人が閲覧可能な情報を、AI やスクリプトが扱いやすい形で取得できます。

## 方針

- **Read-only by default**: 課題提出、削除、編集、投稿、完了状態変更などの副作用は実装しません。
- `--json` 連携を前提に、機械可読の出力を第一級で扱います。
- 認証情報は必要最小限のみ保存し、秘密情報は画面出力しません。

## インストール / ビルド

本リポジトリは Rust 製の CLI を想定しています。

```bash
# Rust toolchain が必要
cargo build --release

# ローカルインストール
cargo install --path .
```

Node.js / npm 経由でも利用できます。通常の `npm install` では、同梱済みまたは GitHub Releases などに置いた prebuilt binary を使うため、利用者側に Rust toolchain は不要です。

```bash
npm install
npx campus-lms capabilities
```

配布用 prebuilt binary は以下の流れで準備します。

```bash
npm run build:native
npm run prepare:prebuilt
npm pack
```

prebuilt binary をパッケージに同梱しない場合は、`package.json` の `repository.url` または `campusLms.binaryBaseUrl` に release URL を設定すると、`postinstall` が以下の名前のファイルを取得します。

```text
campus-lms-v<version>-windows-x64.exe
campus-lms-v<version>-windows-arm64.exe
campus-lms-v<version>-macos-x64
campus-lms-v<version>-macos-arm64
campus-lms-v<version>-linux-x64
campus-lms-v<version>-linux-arm64
```

各 binary と同じ場所に SHA256 sidecar も置きます。`postinstall` は binary をインストールする前に checksum を検証し、不一致ならインストールを失敗させます。

```text
campus-lms-v<version>-windows-x64.exe.sha256
campus-lms-v<version>-macos-arm64.sha256
```

開発者向けにソースからビルドしたい場合だけ、`CAMPUS_LMS_BUILD_FROM_SOURCE=1` を指定します。外部バイナリを直接指定する場合は `CAMPUS_LMS_BIN` を使えます。

配布イメージや CI の成果物を使う場合は、README 配布手順に従ってください。

## 主要コマンド

- `campus-lms auth login`  
  LMS との接続情報を登録し、token を OS credential store に保存
- `campus-lms auth status --json`
- `campus-lms whoami --json`
- `campus-lms courses --json`
- `campus-lms todo --days 14 --json`
- `campus-lms assignment show <assignment_id> --json`
- `campus-lms ai snapshot --days 14 --json`
- `campus-lms capabilities --json`
- `campus-lms schema list --json`
- `campus-lms errors --json`
- `campus-lms ai instructions`
- `campus-lms init`
- `campus-lms cleanup --cache --dry-run`
- `campus-lms uninstall --dry-run`

## 初期化とアンインストール

初回セットアップでは、秘密情報を含まない config と cache ディレクトリだけを作成できます。

```bash
campus-lms init
campus-lms auth login
```

ローカルデータを消す場合は、対象を明示します。実削除前に確認プロンプトが出ます。

```bash
campus-lms cleanup --cache --dry-run
campus-lms cleanup --local-config --dry-run
campus-lms cleanup --all --yes
```

`uninstall` は token / config / cache の削除を行い、npm パッケージ本体の削除コマンドを案内します。npm パッケージ自体を CLI が勝手に削除することはありません。

```bash
campus-lms uninstall --dry-run
campus-lms uninstall --yes
npm uninstall -g campus-lms-cli
```

## 認証・保存ポリシー

- `auth login` で取得した token のみを OS の安全な資格情報ストアへ保存します。
- パスワードは保存しません。
- `--json` 出力時は機密情報を除外し、必要なら `--verbose` でも出力しません。

## 倫理・セキュリティ

- 自分のアカウント・自分の履修範囲の情報のみ扱います。
- 他者の資格情報や他人データには決してアクセスしません。
- 大量取得やログ目的の不正スクレイピングを行いません。
- `view_*` 系など副作用が疑われる API は、明示的許可がない限り利用しません。
- `token` `password` `cookie` `session` を標準出力・ログ・クラッシュレポートに出しません。

## 大学ポリシー確認

運用前に必ず大学側で確認してください。

1. 学務/情報セキュリティ規程で API 利用が許可されているか
2. SSO / MFA / ネットワーク制約の有無
3. モバイル/外部連携ポリシーでの利用可否
4. 利用時の監査・ログ規定（保存期間、取得対象）

## JSON / AI 利用方法

AI 連携は `--json` を前提にし、最小限の情報だけを渡してください。

```bash
campus-lms ai snapshot --days 14 --json
campus-lms assignment show assign:12345 --json
```

- まず `ai snapshot` で全体像を取得し、必要時のみ `detail_command` を実行する
- `--json` を推奨し、成績・フィードバック・個人メールは明示フラグがない限り含めない
- すべての LMS データを private user data として取り扱う

## ライセンス

本プロジェクトは Apache License 2.0 です。  
詳細は `LICENSE` を参照してください。
