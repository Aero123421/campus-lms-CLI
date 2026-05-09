# campus-lms-cli 仕様書

Version: 0.1.0-draft  
Last updated: 2026-05-08  
Primary language: Rust  
Target OS: Windows / macOS / Linux  
License recommendation: Apache-2.0  
Status: MVP implementation ready

---

## 1. 概要

`campus-lms-cli` は、大学の Moodle 系 LMS をブラウザではなく CLI から扱うためのツールである。
主目的は、ユーザー本人が閲覧権限を持つ範囲で、履修コース、課題、締切、小テスト、提出状態、課題本文、添付ファイル情報などを取得し、AI エージェントやスクリプトが安全に利用できる安定した JSON インターフェースを提供することである。

本ツールは、Moodle サーバーに対して **read-only / API-first / JSON-first** でアクセスする。
デフォルトでは課題提出、削除、編集、既読化、完了状態変更、フォーラム投稿、メッセージ送信などの副作用を持つ操作は実装しない。

---

## 2. プロダクト名と商標方針

### 2.1 推奨プロダクト名

正式名称は以下を推奨する。

```text
campus-lms-cli
```

実行ファイル名は短くする。

```text
campus-lms
```

### 2.2 避ける名前

以下のように、Moodle の商標をプロダクト名・パッケージ名の主語として使う名前は避ける。

```text
moodle-cli
moodlectl
moodle-ai-cli
moodle-assignment-cli
```

### 2.3 README 上の説明例

Moodle との互換性を説明するときは、以下のような表現にする。

```text
A read-only CLI integration for Moodle™ LMS-compatible university LMS sites.
```

### 2.4 理由

Moodle の商標・ロゴは Moodle Pty Ltd または関連会社により管理されている。
Moodle 連携であることを説明する用途と、プロダクト名として Moodle を使う用途は分けて考える。

---

## 3. 目的と非目的

### 3.1 目的

- 大学 Moodle の課題・締切・提出状態を CLI から確認できる。
- AI エージェントが `--json` 出力を読んで、ユーザーの学習 ToDo を把握できる。
- 課題詳細、本文、添付ファイルメタデータ、提出状態を取得できる。
- ローカルキャッシュにより Moodle サーバーへの負荷を抑える。
- 認証情報は OS の安全な資格情報ストアに保存する。
- すべての機械向け出力はスキーマバージョン付き JSON とする。
- help / capabilities / schema / errors を充実させ、AI が使い方を理解しやすくする。

### 3.2 非目的

MVP では以下を行わない。

- 課題の自動提出。
- 小テストの自動受験。
- Moodle 上のデータ変更。
- フォーラム投稿。
- メッセージ送信。
- 成績・フィードバックを AI snapshot にデフォルトで含めること。
- 他人のアカウントや他人の履修情報へのアクセス。
- SSO / MFA / アクセス制御の迂回。
- 教材・課題文・PDF の再配布。
- ブラウザ操作の自動化を第一手段にすること。

---

## 4. 倫理・セキュリティ方針

### 4.1 許容される利用

本ツールが想定する利用は以下である。

- ユーザー本人の正規アカウントでログインする。
- ユーザー本人が Moodle 上で閲覧できる情報だけを取得する。
- 課題、締切、提出状態、コース情報を確認する。
- AI に「次に何をやるべきか」を判断させるために、必要最小限の情報を渡す。
- Moodle サーバーへのリクエスト頻度を抑える。
- 大学の利用規約、情報システムポリシー、授業運営方針に従う。

### 4.2 禁止・非推奨の利用

以下はサポートしない。

- 他人の username / password / token / cookie を利用する。
- SSO, MFA, CAS, Shibboleth などの認証を迂回する。
- 大学の許可なく大量スクレイピングする。
- 教材や課題文を第三者に配布する。
- 課題提出、答案作成、提出確定を AI に自動実行させる。
- `view_*` 系 API を無自覚に呼び、閲覧ログや完了状態を変更する。
- token, password, session, cookie を標準出力・ログ・クラッシュレポートに出す。

### 4.3 AI 向け安全原則

AI エージェントに使わせる場合は、以下を原則とする。

```text
- まず campus-lms ai snapshot --days 14 --json を使う。
- detail_command がある場合のみ、必要な詳細を追加取得する。
- grades はユーザーが明示したときだけ取得する。
- 提出、削除、編集、投稿、完了状態変更は行わない。
- すべての Moodle データを private user data として扱う。
```

---

## 5. 技術スタック

### 5.1 言語

Rust をメイン言語とする。

理由:

- Windows / macOS / Linux 向けに単一バイナリ配布しやすい。
- Python / Node.js ランタイムなしで軽量に動作する。
- CLI 引数、JSON スキーマ、エラー型を静的型で管理しやすい。
- 認証情報やトークンの扱いを明確に分離しやすい。
- AI 向けの安定 I/O を作りやすい。

### 5.2 主要 crate 案

```toml
[package]
name = "campus-lms-cli"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"

[dependencies]
anyhow = "1"
thiserror = "1"
clap = { version = "4", features = ["derive"] }
clap_complete = "4"
reqwest = { version = "0.12", default-features = false, features = ["blocking", "json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
rpassword = "7"
keyring = "3"
directories = "5"
time = { version = "0.3", features = ["serde", "formatting", "parsing"] }
scraper = "0.19"
html2text = "0.12"
url = "2"
sha2 = "0.10"
```

### 5.3 blocking / async 方針

MVP では `reqwest::blocking` を使う。

理由:

- CLI の 1 回の実行で行う API 呼び出しは少数である。
- 実装が簡潔になる。
- エラー処理が読みやすい。
- 将来、添付ファイル大量取得や並列取得が必要になった場合に async 化する。

---

## 6. 全体アーキテクチャ

```text
src/
  main.rs
  cli.rs
  output.rs
  error.rs
  config.rs
  auth.rs
  keychain.rs
  cache.rs
  moodle/
    mod.rs
    client.rs
    params.rs
    models.rs
    courses.rs
    assignments.rs
    calendar.rs
    quizzes.rs
    grades.rs
    files.rs
  ai/
    mod.rs
    snapshot.rs
    instructions.rs
  schema/
    mod.rs
    json_schema.rs
  docs/
    capabilities.rs
    errors.rs
```

### 6.1 責務

| ファイル | 責務 |
|---|---|
| `main.rs` | エントリポイント |
| `cli.rs` | clap によるコマンド定義 |
| `output.rs` | JSON / text 出力の統一 |
| `error.rs` | エラー型、exit code、JSON error response |
| `config.rs` | 設定ファイル読み書き |
| `auth.rs` | login / logout / status |
| `keychain.rs` | OS credential store との接続 |
| `cache.rs` | ローカルキャッシュ |
| `moodle/client.rs` | Moodle Web Service client |
| `moodle/params.rs` | Moodle REST の配列パラメータ flatten |
| `moodle/models.rs` | Moodle API response 型 |
| `ai/snapshot.rs` | AI 向け集約データ生成 |
| `schema/json_schema.rs` | JSON Schema 出力 |
| `docs/capabilities.rs` | capabilities 出力 |

---

## 7. Moodle API 方針

### 7.1 API-first

Moodle へのアクセスは、原則として Moodle Web Services / External Services を使う。
HTML スクレイピングやブラウザ自動操作は MVP では実装しない。

### 7.2 REST endpoint

Moodle REST Web Service は以下の endpoint を想定する。

```text
POST {base_url}/webservice/rest/server.php
```

共通パラメータ:

```text
wstoken=<token>
wsfunction=<function name>
moodlewsrestformat=json
```

### 7.3 token endpoint

username / password でログインする場合、以下を使う。

```text
POST {base_url}/login/token.php
```

パラメータ:

```text
username=<username>
password=<password>
service=moodle_mobile_app
```

注意:

- 大学 Moodle が SSO / MFA / CAS / Shibboleth を使っている場合、この方式は使えないことがある。
- `/login/token.php` が無効な場合や Mobile Web Services が無効な場合は失敗する。
- password は保存しない。取得した token だけを OS credential store に保存する。

### 7.4 MVP で使う候補 API function

| 目的 | function |
|---|---|
| 接続確認・ユーザー情報 | `core_webservice_get_site_info` |
| 履修コース一覧 | `core_enrol_get_users_courses` |
| コース内活動・教材 | `core_course_get_contents` |
| 課題一覧 | `mod_assign_get_assignments` |
| 課題提出状態 | `mod_assign_get_submission_status` |
| カレンダー upcoming | `core_calendar_get_calendar_upcoming_view` |
| 期限順 action events | `core_calendar_get_action_events_by_timesort` |
| コース別 action events | `core_calendar_get_action_events_by_courses` |
| 小テスト一覧 | `mod_quiz_get_quizzes_by_courses` |
| 成績 overview | `gradereport_overview_get_course_grades` |
| 成績 item | `gradereport_user_get_grade_items` |
| 完了状況 | `core_completion_get_activities_completion_status` |

### 7.5 MVP で避ける API function

以下のような、閲覧ログ・完了状態・イベント発火に関係し得る関数は MVP では呼ばない。

```text
mod_assign_view_assign
mod_quiz_view_quiz
mod_resource_view_resource
gradereport_user_view_grade_report
```

将来的に必要になった場合は、以下を満たすこと。

- `--allow-view-event` のような明示フラグを要求する。
- help に副作用を明記する。
- `--dry-run` では呼ばない。
- JSON 出力の `warnings` に副作用を明記する。

---

## 8. 認証仕様

### 8.1 コマンド

```bash
campus-lms auth login
campus-lms auth logout
campus-lms auth status --json
```

### 8.2 login flow

```text
1. campus-lms auth login を実行する。
2. base_url を入力する。
3. username を入力する。
4. password を非表示入力する。
5. /login/token.php に username/password/service を送る。
6. token を受け取る。
7. password は即破棄する。
8. token を OS credential store に保存する。
9. base_url, username, profile name など非秘密情報を config.toml に保存する。
```

### 8.3 保存する情報

#### OS credential store

保存する秘密情報:

```text
token
```

保存キー:

```text
service = campus-lms:<base_url>
account = <username>
secret  = <token>
```

#### config.toml

保存する非秘密情報:

```toml
[profile.default]
base_url = "https://lms.example.ac.jp"
username = "student@example.ac.jp"
service = "moodle_mobile_app"
cache_ttl_seconds = 300
```

禁止:

```toml
password = "..." # 禁止
token = "..."    # 禁止
cookie = "..."   # 禁止
```

### 8.4 password 保存について

MVP では password 保存を実装しない。
ユーザー体験としては username/password で login するが、端末に保存されるのは Moodle token のみとする。

将来的に password 保存を実装する場合も、以下を必須にする。

```text
- --save-password の明示指定が必要。
- OS credential store 以外には保存しない。
- 初回実行時に危険性を明示する。
- config.toml, env, log, stdout には絶対に出さない。
- auth logout で削除する。
```

### 8.5 HTTPS 強制

`auth login` は原則として HTTPS の `base_url` のみ許可する。

例外:

```bash
campus-lms auth login --allow-insecure-localhost
```

この例外は以下に限定する。

```text
http://localhost
http://127.0.0.1
```

---

## 9. 設定仕様

### 9.1 設定ファイルパス

| OS | 例 |
|---|---|
| macOS | `~/Library/Application Support/campus-lms/config.toml` |
| Linux | `~/.config/campus-lms/config.toml` |
| Windows | `%APPDATA%\\campus-lms\\config.toml` |

Rust では `directories` crate を使い、OS ごとの標準パスに保存する。

### 9.2 設定例

```toml
active_profile = "default"

[profile.default]
base_url = "https://lms.example.ac.jp"
username = "student@example.ac.jp"
service = "moodle_mobile_app"
cache_ttl_seconds = 300
cache_retention_seconds = 2592000

[privacy]
include_grades_in_ai_snapshot = false
include_feedback_in_ai_snapshot = false

[output]
timezone = "Asia/Tokyo"
```

### 9.3 複数大学・複数アカウント

将来的に profile をサポートする。

```bash
campus-lms --profile default todo --json
campus-lms --profile grad-school todo --json
```

MVP では `default` profile のみでもよい。

---

## 10. コマンド仕様

### 10.1 グローバルオプション

```bash
campus-lms [GLOBAL_OPTIONS] <COMMAND>
```

| option | 説明 |
|---|---|
| `--profile <NAME>` | 利用する profile |
| `--config <PATH>` | 設定ファイルパスを明示 |
| `--json` | JSON 出力 |
| `--no-color` | 互換用予約オプション。現状の出力は色を使わない |
| `--verbose` | 詳細ログ。ただし秘密情報は出さない |
| `--warning-details <N\|all>` | warning 詳細件数。通常は0、`--verbose` は all |
| `--quiet` | 最小出力 |
| `--version` | バージョン表示 |
| `--help` | help |

注意:

- `--json` 時、stdout には JSON のみ出す。
- progress, debug, warning は stderr に出す。
- `--json` 時のエラーも JSON で stdout に出すか、運用上の一貫性を重視して stderr に JSON を出すか、実装前に統一する。
- 本仕様では **通常結果は stdout、エラー JSON は stderr** を推奨する。

---

## 11. MVP コマンド一覧

### 11.1 auth login

```bash
campus-lms auth login
```

説明:

- 対話的に base_url, username, password を入力する。
- `/login/token.php` で token を取得する。
- token を OS credential store に保存する。

オプション:

| option | 説明 |
|---|---|
| `--base-url <URL>` | 入力を省略して指定 |
| `--username <USER>` | 入力を省略して指定 |
| `--service <NAME>` | service shortname。default: `moodle_mobile_app` |
| `--allow-insecure-localhost` | localhost の HTTP を許可 |

### 11.2 auth logout

```bash
campus-lms auth logout
```

説明:

- OS credential store から token を削除する。
- 必要に応じて config も削除する。

オプション:

| option | 説明 |
|---|---|
| `--keep-config` | token だけ削除し config は残す |

### 11.3 auth status

```bash
campus-lms auth status --json
```

JSON 例:

```json
{
  "schema_version": "campus-lms.auth_status.v1",
  "generated_at": "2026-05-08T10:30:00+09:00",
  "authenticated": true,
  "profile": "default",
  "base_url": "https://lms.example.ac.jp",
  "username": "student@example.ac.jp",
  "token_available": true,
  "warnings": []
}
```

### 11.4 whoami

```bash
campus-lms whoami --json
```

説明:

- `core_webservice_get_site_info` を呼び、接続確認とユーザー情報を取得する。

JSON 例:

```json
{
  "schema_version": "campus-lms.whoami.v1",
  "generated_at": "2026-05-08T10:30:00+09:00",
  "user": {
    "id": "user:123",
    "username": "student@example.ac.jp",
    "fullname": null,
    "site_name": "Example University Moodle"
  },
  "warnings": []
}
```

### 11.5 courses

```bash
campus-lms courses --json
```

説明:

- 履修中・閲覧可能なコース一覧を取得する。

オプション:

| option | 説明 |
|---|---|
| `--refresh` | キャッシュを無視して取得 |
| `--offline` | キャッシュのみ使用 |

JSON 例:

```json
{
  "schema_version": "campus-lms.courses.v1",
  "generated_at": "2026-05-08T10:30:00+09:00",
  "cache": {
    "used": true,
    "fetched_at": "2026-05-08T10:25:00+09:00",
    "ttl_seconds": 3600
  },
  "courses": [
    {
      "id": "course:101",
      "moodle_id": 101,
      "short_name": "INFO101",
      "full_name": "情報理論",
      "visible": true,
      "url": "https://lms.example.ac.jp/course/view.php?id=101"
    }
  ],
  "warnings": []
}
```

### 11.6 todo

```bash
campus-lms todo --days 14 --json
```

説明:

- 直近 N 日の課題、小テスト、カレンダーイベントを取得し、未対応タスクとして整形する。

オプション:

| option | 説明 |
|---|---|
| `--days <DAYS>` | 何日先まで見るか。default: 14 |
| `--max-items <N>` | 最大件数 |
| `--refresh` | キャッシュを無視 |
| `--offline` | キャッシュのみ |
| `--include-submitted` | 提出済みも含める |
| `--course <COURSE_ID>` | コースで絞る |

JSON 例:

```json
{
  "schema_version": "campus-lms.todo.v1",
  "generated_at": "2026-05-08T10:30:00+09:00",
  "range": {
    "from": "2026-05-08",
    "to": "2026-05-22",
    "timezone": "Asia/Tokyo"
  },
  "cache": {
    "used": false,
    "fetched_at": "2026-05-08T10:30:00+09:00",
    "ttl_seconds": 300
  },
  "items": [
    {
      "id": "assign:12345",
      "type": "assignment",
      "course_id": "course:101",
      "course_name": "情報理論",
      "title": "レポート2",
      "due_at": "2026-05-12T23:59:00+09:00",
      "due_in_seconds": 393540,
      "status": "not_submitted",
      "priority_hint": "high",
      "url": "https://lms.example.ac.jp/mod/assign/view.php?id=12345",
      "detail_command": "campus-lms assignment show assign:12345 --json"
    }
  ],
  "warnings": []
}
```

### 11.7 assignment show

```bash
campus-lms assignment show assign:12345 --json
```

説明:

- 課題詳細、締切、提出状態、課題本文、添付ファイルメタデータを取得する。
- 課題を提出しない。
- 完了状態を変更しない。

オプション:

| option | 説明 |
|---|---|
| `--max-chars <N>` | 本文の最大文字数。default: 8000 |
| `--include-html` | HTML 本文も含める |
| `--refresh` | キャッシュを無視 |
| `--offline` | キャッシュのみ |

JSON 例:

```json
{
  "schema_version": "campus-lms.assignment.v1",
  "generated_at": "2026-05-08T10:30:00+09:00",
  "assignment": {
    "id": "assign:12345",
    "moodle_id": 12345,
    "cmid": 67890,
    "course_id": "course:101",
    "course_name": "情報理論",
    "title": "レポート2",
    "due_at": "2026-05-12T23:59:00+09:00",
    "allows_submission_from": "2026-05-01T00:00:00+09:00",
    "cutoff_at": null,
    "description_text": "レポート2では...",
    "description_truncated": false,
    "description_original_length_chars": 1200,
    "description_html": null,
    "attachments": [
      {
        "id": "file:sha256:abc123",
        "name": "report2.pdf",
        "mime_type": "application/pdf",
        "size_bytes": 204812,
        "download_url_available": true,
        "download_command": "campus-lms file download file:sha256:abc123 --out report2.pdf"
      }
    ],
    "submission": {
      "status": "not_submitted",
      "last_modified_at": null,
      "grading_status": "not_graded"
    },
    "url": "https://lms.example.ac.jp/mod/assign/view.php?id=12345"
  },
  "warnings": []
}
```

### 11.8 ai snapshot

```bash
campus-lms ai snapshot --days 14 --json
```

説明:

- AI が最初に叩くための集約コマンド。
- 直近の未対応タスク、締切、コース、詳細取得コマンドを返す。
- デフォルトでは成績・フィードバック・メールアドレス・長大本文は含めない。

オプション:

| option | 説明 |
|---|---|
| `--days <DAYS>` | 何日先まで見るか。default: 14 |
| `--max-items <N>` | 最大件数。default: 30 |
| `--include-grades` | 成績情報を含める。default: false |
| `--include-feedback` | フィードバックを含める。default: false |
| `--refresh` | キャッシュを無視 |
| `--offline` | キャッシュのみ |

JSON 例:

```json
{
  "schema_version": "campus-lms.ai_snapshot.v1",
  "generated_at": "2026-05-08T10:30:00+09:00",
  "privacy": {
    "grades_included": false,
    "feedback_included": false,
    "user_email_included": false
  },
  "range": {
    "from": "2026-05-08",
    "to": "2026-05-22",
    "timezone": "Asia/Tokyo"
  },
  "summary": {
    "pending_count": 3,
    "overdue_count": 0,
    "due_within_48h_count": 1
  },
  "courses": [
    {
      "id": "course:101",
      "name": "情報理論"
    }
  ],
  "pending_tasks": [
    {
      "id": "assign:12345",
      "type": "assignment",
      "course_id": "course:101",
      "course_name": "情報理論",
      "title": "レポート2",
      "due_at": "2026-05-12T23:59:00+09:00",
      "due_in_seconds": 393540,
      "status": "not_submitted",
      "priority_hint": "high",
      "detail_command": "campus-lms assignment show assign:12345 --json"
    }
  ],
  "warnings": []
}
```

### 11.9 capabilities

```bash
campus-lms capabilities --json
```

説明:

- AI / スクリプト向けに、利用可能コマンド、read-only 性、安全性、例を JSON で返す。

JSON 例:

```json
{
  "schema_version": "campus-lms.capabilities.v1",
  "recommended_entrypoint": "campus-lms ai snapshot --days 14 --json",
  "commands": [
    {
      "name": "ai snapshot",
      "read_only": true,
      "safe_for_ai": true,
      "description": "Return a compact overview of upcoming Moodle tasks.",
      "example": "campus-lms ai snapshot --days 14 --json"
    },
    {
      "name": "assignment show",
      "read_only": true,
      "safe_for_ai": true,
      "description": "Show assignment details.",
      "example": "campus-lms assignment show assign:12345 --json"
    }
  ],
  "dangerous_commands": []
}
```

### 11.10 schema

```bash
campus-lms schema list --json
campus-lms schema show ai_snapshot.v1
```

説明:

- 出力 JSON の JSON Schema を表示する。
- テスト、AI 連携、将来の MCP 化に使う。

### 11.11 errors

```bash
campus-lms errors --json
```

説明:

- エラーコードと exit code の一覧を返す。

JSON 例:

```json
{
  "schema_version": "campus-lms.errors.v1",
  "errors": [
    {
      "code": "AUTH_REQUIRED",
      "exit_code": 10,
      "retryable": false,
      "hint": "Run: campus-lms auth login"
    },
    {
      "code": "NETWORK_ERROR",
      "exit_code": 12,
      "retryable": true,
      "hint": "Check your network connection or Moodle base URL."
    }
  ]
}
```

### 11.12 ai instructions

```bash
campus-lms ai instructions
```

説明:

- AI エージェントに読ませる短い利用ガイドを出力する。

出力例:

```text
Use campus-lms as a read-only interface to the user's Moodle-compatible LMS.

Recommended first command:
  campus-lms ai snapshot --days 14 --json

Rules:
- Prefer --json for all commands.
- Do not call auth login unless the user asks.
- Do not request grades unless the user asks.
- Do not submit assignments.
- Use detail_command fields from JSON outputs to fetch more information.
- Treat all LMS data as private user data.
```

---

## 12. 将来コマンド

MVP 後に検討する。

```bash
campus-lms quizzes --json
campus-lms quiz show quiz:333 --json
campus-lms grades --json
campus-lms calendar upcoming --days 14 --json
campus-lms course show course:101 --json
campus-lms file download file:sha256:abc123 --out report2.pdf
campus-lms cache clear
campus-lms cache status --json
campus-lms privacy report --json
campus-lms doctor --json
campus-lms completions bash
campus-lms completions zsh
campus-lms completions fish
campus-lms completions powershell
```

---

## 13. JSON 出力規約

### 13.1 基本規約

- `--json` 時、stdout は JSON のみ。
- top-level は array ではなく object。
- 必ず `schema_version` を含める。
- 必ず `generated_at` を含める。
- 日時は ISO 8601。
- timezone は明示する。
- 警告は `warnings` 配列に入れる。
- 破壊的変更は schema version を上げる。

### 13.2 共通 response 型

```json
{
  "schema_version": "campus-lms.<name>.v1",
  "generated_at": "2026-05-08T10:30:00+09:00",
  "warnings": []
}
```

### 13.3 warning 型

```json
{
  "code": "PARTIAL_DATA",
  "message": "Some courses could not be fetched.",
  "hint": "Run with --refresh or check permissions."
}
```

### 13.4 ID 形式

外部出力では prefix 付き ID を使う。

```text
user:123
course:101
assign:12345
quiz:333
file:sha256:abc123
calendar:555
```

Moodle 内部 ID も必要に応じて `moodle_id` として含める。

### 13.5 日時形式

Unix timestamp は出力しない。

```json
{
  "due_at": "2026-05-12T23:59:00+09:00",
  "due_in_seconds": 393540
}
```

### 13.6 HTML 取り扱い

デフォルトでは text 化した本文を出す。

```json
{
  "description_text": "レポート2では...",
  "description_html": null,
  "description_html_available": true
}
```

HTML が必要な場合だけ以下を使う。

```bash
campus-lms assignment show assign:12345 --json --include-html
```

---

## 14. エラー仕様

### 14.1 exit code

| exit code | 意味 |
|---:|---|
| 0 | success |
| 1 | unknown error |
| 2 | invalid arguments |
| 10 | auth required |
| 11 | permission denied |
| 12 | network error |
| 13 | rate limited |
| 14 | auth expired |
| 20 | Moodle API error |
| 21 | unsupported Moodle feature |
| 30 | local config error |
| 31 | keychain unavailable |
| 32 | cache error |
| 40 | parse error |

### 14.2 error JSON

`--json` 時のエラー形式。

```json
{
  "schema_version": "campus-lms.error.v1",
  "error": {
    "code": "AUTH_REQUIRED",
    "message": "Authentication is required.",
    "retryable": false,
    "hint": "Run: campus-lms auth login"
  }
}
```

### 14.3 エラーコード

```text
AUTH_REQUIRED
AUTH_EXPIRED
PERMISSION_DENIED
NETWORK_ERROR
MOODLE_UNAVAILABLE
RATE_LIMITED
INVALID_ARGUMENT
NOT_FOUND
UNSUPPORTED_MOODLE_FEATURE
KEYCHAIN_UNAVAILABLE
CONFIG_ERROR
CACHE_ERROR
PARSE_ERROR
MOODLE_API_ERROR
```

---

## 15. キャッシュ仕様

### 15.1 目的

- AI が同じコマンドを何度も叩くことによる Moodle 負荷を減らす。
- ネットワーク不安定時に `--offline` で最低限見られるようにする。
- 実行速度を上げる。

### 15.2 キャッシュパス

| OS | 例 |
|---|---|
| macOS | `~/Library/Caches/campus-lms/` |
| Linux | `~/.cache/campus-lms/` |
| Windows | `%LOCALAPPDATA%\\campus-lms\\cache\\` |

### 15.3 TTL

| データ | TTL |
|---|---:|
| courses | 3600 秒 |
| todo | 300 秒 |
| assignment detail | 600 秒 |
| calendar | 300 秒 |
| grades | 300 秒、ただし AI snapshot にはデフォルトで含めない |

### 15.4 オプション

```bash
--refresh  # キャッシュを無視して取得
--offline  # キャッシュのみ使用
```

`--refresh` と `--offline` は同時指定不可。

### 15.5 cache metadata

JSON には可能な限り cache 情報を含める。

```json
{
  "cache": {
    "used": true,
    "fetched_at": "2026-05-08T10:25:00+09:00",
    "ttl_seconds": 300
  }
}
```

---

## 16. privacy 仕様

### 16.1 AI snapshot に含めないもの

デフォルトでは以下を含めない。

- メールアドレス。
- 学籍番号。
- 成績。
- フィードバック全文。
- 教員・TA の個人情報。
- 添付ファイル本文。
- token, cookie, session。
- 長大な HTML。

### 16.2 明示フラグ

必要な場合のみ明示指定する。

```bash
campus-lms ai snapshot --include-grades --json
campus-lms ai snapshot --include-feedback --json
campus-lms assignment show assign:12345 --include-html --json
```

### 16.3 privacy report

将来実装。

```bash
campus-lms privacy report --json
```

出力例:

```json
{
  "schema_version": "campus-lms.privacy_report.v1",
  "stored_locally": [
    {
      "kind": "token",
      "location": "os_credential_store",
      "plaintext_file": false
    },
    {
      "kind": "config",
      "location": "config.toml",
      "contains_secret": false
    },
    {
      "kind": "cache",
      "location": "cache directory",
      "contains_course_content": true
    }
  ]
}
```

---

## 17. help 仕様

### 17.1 方針

AI が叩きやすい CLI にするため、help は以下を明記する。

- 何をするコマンドか。
- read-only かどうか。
- AI に安全かどうか。
- JSON 出力例。
- 代表的な実行例。
- 副作用の有無。
- 認証が必要な場合の対処。

### 17.2 top-level help 例

```text
campus-lms

A read-only CLI for inspecting Moodle-compatible university LMS tasks,
deadlines, assignments, quizzes, and course information.

Most commands are safe for automation and AI agents when used with --json.
This CLI does not submit assignments or modify LMS state by default.

Recommended AI entrypoint:
  campus-lms ai snapshot --days 14 --json

Common commands:
  campus-lms auth login
  campus-lms auth status --json
  campus-lms courses --json
  campus-lms todo --days 14 --json
  campus-lms assignment show assign:12345 --json
  campus-lms capabilities --json

Safety:
  - Read-only by default
  - Credentials are stored in the OS credential store
  - Tokens and passwords are never printed

Use:
  campus-lms <command> --help
```

### 17.3 command help 例

```text
campus-lms todo

List upcoming LMS tasks such as assignments, quizzes, and calendar events.

This command is read-only and safe for AI agents.

Examples:
  campus-lms todo --days 14
  campus-lms todo --days 14 --json
  campus-lms todo --days 30 --json --refresh
  campus-lms todo --json --offline

Options:
  --days <DAYS>       Number of days to look ahead [default: 14]
  --json              Output machine-readable JSON
  --refresh           Ignore cache and fetch from LMS
  --offline           Use cached data only
  --max-items <N>     Limit number of returned items
```

---

## 18. AGENTS.md 推奨内容

リポジトリに `AGENTS.md` を置く。

```md
# AGENTS.md

Use `campus-lms` as a read-only interface to the user's Moodle-compatible LMS.

Recommended first command:

```bash
campus-lms ai snapshot --days 14 --json
```

Rules:

- Prefer `--json` for all commands.
- Do not call `auth login` unless the user explicitly asks.
- Do not request grades unless the user asks.
- Do not submit assignments.
- Do not modify LMS state.
- Use `detail_command` fields from JSON outputs to fetch more information.
- Treat all LMS content as private user data.
- Never print tokens, passwords, cookies, or session information.
```

---

## 19. 実装詳細

### 19.1 Moodle REST parameter flatten

Moodle REST API は配列パラメータに以下の形式を使う。

```text
courseids[0]=101
courseids[1]=102
```

Rust では `serde_json::Value` または専用 enum を flatten して `Vec<(String, String)>` に変換する。

擬似コード:

```rust
fn flatten(prefix: &str, value: &serde_json::Value, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                flatten(&format!("{}[{}]", prefix, i), item, out);
            }
        }
        serde_json::Value::Object(map) => {
            for (k, item) in map {
                flatten(&format!("{}[{}]", prefix, k), item, out);
            }
        }
        serde_json::Value::Null => {}
        other => out.push((prefix.to_string(), scalar_to_string(other))),
    }
}
```

### 19.2 Moodle client interface

```rust
pub struct MoodleClient {
    base_url: Url,
    token: String,
    http: reqwest::blocking::Client,
}

impl MoodleClient {
    pub fn call<T: serde::de::DeserializeOwned>(
        &self,
        function: &str,
        params: serde_json::Value,
    ) -> Result<T, CampusError>;
}
```

### 19.3 secret redaction

ログ出力前に以下を redaction する。

```text
token
wstoken
password
cookie
session
Authorization
```

redaction 表記:

```text
<redacted>
```

---

## 20. テスト仕様

### 20.1 unit tests

- Moodle REST parameter flatten。
- ID parser。
- datetime conversion。
- error mapping。
- JSON serialization。
- HTML to text conversion。
- secret redaction。

### 20.2 integration tests

- mock server を使って `/login/token.php` 成功。
- `/login/token.php` 失敗。
- `/webservice/rest/server.php` 成功。
- Moodle API error response。
- network error。
- cache hit / miss。
- `--offline` 時の挙動。

### 20.3 snapshot tests

CLI 出力の JSON を snapshot test する。

対象:

```text
auth status --json
courses --json
todo --json
assignment show --json
ai snapshot --json
capabilities --json
errors --json
```

### 20.4 security tests

- token が stdout に出ない。
- token が stderr に出ない。
- token が config.toml に保存されない。
- password が保存されない。
- `--verbose` でも秘密情報が redaction される。

---

## 21. 配布仕様

### 21.1 対象 platform

```text
x86_64-pc-windows-msvc
aarch64-apple-darwin
x86_64-apple-darwin
x86_64-unknown-linux-gnu
aarch64-unknown-linux-gnu
```

Linux の完全 static binary は keyring / Secret Service との相性を確認してから判断する。

### 21.2 配布方法

MVP:

```text
cargo install --path .
GitHub Releases binary
```

将来:

```text
Homebrew tap
Scoop bucket
winget
AUR
npm wrapper package
```

### 21.3 バージョン管理

Semantic Versioning を使う。

```text
0.1.0 MVP
0.2.0 quizzes / calendar 改善
0.3.0 grades opt-in
1.0.0 JSON schema 安定化
```

---

## 22. ライセンス仕様

### 22.1 外部 CLI

Moodle 本体コードをコピーせず、HTTP API を呼ぶ独立 CLI として実装する場合は、以下を推奨する。

```text
Apache-2.0
```

理由:

- permissive license で大学・個人・OSS 利用に向いている。
- 特許条項がある。
- Rust エコシステムでも使いやすい。

シンプルさを優先するなら MIT も候補。

### 22.2 Moodle plugin を作る場合

将来的に Moodle サーバー側 plugin を作る場合は、Moodle core との関係上、GPL-3.0-or-later を前提にする。

### 22.3 SPDX

各 Rust ファイルに入れる。

```rust
// SPDX-License-Identifier: Apache-2.0
```

`Cargo.toml`:

```toml
license = "Apache-2.0"
```

---

## 23. セキュリティチェックリスト

MVP 完了前に以下を満たす。

```text
[ ] password を保存しない。
[ ] token を config.toml に保存しない。
[ ] token を OS credential store に保存する。
[ ] HTTPS 以外の login を拒否する。
[ ] stdout / stderr / log に secret が出ない。
[ ] --json 時に余計な text を stdout に出さない。
[ ] auth logout で token を削除できる。
[ ] AI snapshot に grades をデフォルトで含めない。
[ ] AI snapshot に feedback をデフォルトで含めない。
[ ] 課題提出・削除・編集コマンドを実装しない。
[ ] view_* API を MVP で呼ばない。
[ ] キャッシュ TTL を実装する。
[ ] --refresh と --offline の排他制御をする。
[ ] 利用規約・大学ポリシー確認を README に促す。
```

---

## 24. MVP 実装順序

### Phase 0: scaffold

```text
[ ] cargo project 作成
[ ] clap のトップレベル CLI
[ ] --help / --version
[ ] error 型と exit code
[ ] JSON output wrapper
```

### Phase 1: auth

```text
[ ] config.toml 保存
[ ] keyring 保存 / 取得 / 削除
[ ] auth login
[ ] auth logout
[ ] auth status --json
[ ] HTTPS 強制
```

### Phase 2: Moodle client

```text
[ ] REST client
[ ] parameter flatten
[ ] core_webservice_get_site_info
[ ] Moodle API error mapping
[ ] secret redaction
```

### Phase 3: courses / assignments

```text
[ ] courses --json
[ ] mod_assign_get_assignments
[ ] assignment ID mapping
[ ] assignment show --json
[ ] HTML to text
[ ] submission status
```

### Phase 4: todo / AI snapshot

```text
[ ] calendar action events
[ ] todo aggregation
[ ] priority_hint
[ ] detail_command
[ ] ai snapshot --json
```

### Phase 5: help / schemas / docs

```text
[ ] capabilities --json
[ ] errors --json
[ ] schema list --json
[ ] schema show
[ ] ai instructions
[ ] AGENTS.md
[ ] README.md
```

### Phase 6: cache / tests / release

```text
[ ] cache TTL
[ ] --refresh
[ ] --offline
[ ] unit tests
[ ] integration tests with mock server
[ ] release binary build
```

---

## 25. Definition of Done for MVP

MVP は以下を満たしたら完了とする。

```text
[ ] Windows / macOS / Linux でビルドできる。
[ ] campus-lms auth login で token を保存できる。
[ ] password は保存されない。
[ ] campus-lms whoami --json が成功する。
[ ] campus-lms courses --json が成功する。
[ ] campus-lms todo --days 14 --json が成功する。
[ ] campus-lms assignment show assign:<id> --json が成功する。
[ ] campus-lms ai snapshot --days 14 --json が成功する。
[ ] campus-lms capabilities --json が成功する。
[ ] campus-lms errors --json が成功する。
[ ] すべての --json 出力が JSON として parse できる。
[ ] token/password が stdout/stderr/log/config/cache に出ない。
[ ] README に倫理・セキュリティ・大学ポリシー確認を記載している。
[ ] LICENSE が Apache-2.0 である。
```

---

## 26. 参考資料

- Moodle External Services developer documentation: https://moodledev.io/docs/5.0/apis/subsystems/external
- Moodle Web service client documentation: https://docs.moodle.org/dev/Creating_a_web_service_client
- Moodle Web service API functions: https://docs.moodle.org/dev/Web_service_API_functions
- Moodle Assignment Web Services: https://docs.moodle.org/dev/Assignment_Web_Services
- Moodle trademark page: https://moodle.com/trademarks/
- Moodle developer license page: https://moodledev.io/general/license
- Moodle plugin contribution checklist: https://moodledev.io/general/community/plugincontribution/checklist

---

## 27. 備考

この仕様は、大学側の Moodle 設定によって一部変更が必要になる。
特に以下は大学ごとに異なる。

- `/login/token.php` が使えるか。
- Mobile Web Services が有効か。
- `moodle_mobile_app` service が使えるか。
- 各 Web Service function が許可されているか。
- SSO / MFA 環境か。
- 学内ネットワークや VPN が必要か。
- 利用規約上、CLI/API アクセスが許可されているか。

最初の実装では、API が使えない場合に無理に迂回せず、明確なエラーを返す。

```json
{
  "schema_version": "campus-lms.error.v1",
  "error": {
    "code": "UNSUPPORTED_MOODLE_FEATURE",
    "message": "This Moodle site does not expose the required web service function.",
    "retryable": false,
    "hint": "Ask your university LMS administrator whether Moodle Web Services or Mobile Web Services are enabled."
  }
}
```
