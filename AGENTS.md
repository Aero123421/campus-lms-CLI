# AGENTS.md

このリポジトリでは AI エージェント向けに以下を運用します。

- 主に日本語を使用する。
- 簡単なタスクは gpt5.3spark を割り当てる。

Use `campus-lms` as a read-only interface to the user's Moodle-compatible LMS.

Recommended first command:

```bash
campus-lms ai snapshot --days 14 --json
```

## AI ルール

- 全コマンドで可能な限り `--json` を優先する。
- `auth login` はユーザー明示なく呼び出さない。
- 成績・フィードバック・メールアドレスは、ユーザー明示の `--include-*` がない限り取得しない。
- 課題提出・編集・削除・投稿・完了状態変更を行わない。
- LMS の状態変更を伴う処理は実行しない。
- JSON 出力の `detail_command` を用いて、必要な場合のみ詳細を追加取得する。
- すべての LMS コンテンツを private user data として扱う。
- `token`、`password`、`cookie`、`session` を出力・保存・記録しない。
- 学生本人のアカウントと閲覧権限内の情報に限定する。
- 大学の情報ポリシーに反する操作は提案しない。
- 追加情報や自動実行は、ユーザーの明示的承認を優先する。
