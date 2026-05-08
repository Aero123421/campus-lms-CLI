# AGENTS.md

- 主に日本語で対応する。
- 簡単なタスクは gpt5.3spark を使う。
- `campus-lms` は read-only な LMS 確認ツールとして扱う。
- AI の最初の確認は `campus-lms ai snapshot --days 14 --json` を使う。
- `auth login`、成績取得、提出・編集・削除はユーザーの明示なしに行わない。
- token、password、cookie、session は出力しない。
