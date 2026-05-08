// SPDX-License-Identifier: Apache-2.0

use crate::{
    cli::{AiCommand, Cli, TodoArgs},
    moodle::calendar,
    output,
};

pub fn run(cli: &Cli, command: &AiCommand) -> crate::error::Result<()> {
    match command {
        AiCommand::Snapshot(args) => snapshot(cli, args),
        AiCommand::Instructions => instructions(),
    }
}

fn snapshot(cli: &Cli, args: &crate::cli::AiSnapshotArgs) -> crate::error::Result<()> {
    let mut warnings = Vec::new();
    if args.include_grades {
        warnings.push(serde_json::json!({
            "code": "GRADES_NOT_IMPLEMENTED",
            "message": "--include-grades is accepted for CLI compatibility, but grade retrieval is not implemented in this MVP.",
            "hint": "Do not assume grades are present in this snapshot."
        }));
    }
    if args.include_feedback {
        warnings.push(serde_json::json!({
            "code": "FEEDBACK_NOT_IMPLEMENTED",
            "message": "--include-feedback is accepted for CLI compatibility, but feedback retrieval is not implemented in this MVP.",
            "hint": "Use assignment detail commands only for assignment text and submission metadata."
        }));
    }
    let todo_args = TodoArgs {
        days: args.days,
        max_items: Some(args.max_items),
        refresh: args.refresh,
        offline: args.offline,
        include_submitted: false,
        course: None,
    };
    let payload = calendar::fetch(cli, &todo_args).map_err(|err| err.with_json(cli.json))?;
    let pending_count = payload.items.len();
    let overdue_count = payload
        .items
        .iter()
        .filter(|item| item.priority_hint == "overdue")
        .count();
    let due_within_48h_count = payload
        .items
        .iter()
        .filter(|item| item.priority_hint == "high")
        .count();

    let mut courses = Vec::new();
    for item in &payload.items {
        if let Some(course_id) = &item.course_id {
            if !courses.iter().any(|course: &serde_json::Value| {
                course.get("id").and_then(|value| value.as_str()) == Some(course_id.as_str())
            }) {
                courses.push(serde_json::json!({
                    "id": course_id,
                    "name": item.course_name
                }));
            }
        }
    }

    output::print_json(&serde_json::json!({
        "schema_version": "campus-lms.ai_snapshot.v1",
        "generated_at": output::generated_at(),
        "privacy": {
            "grades_included": false,
            "feedback_included": false,
            "user_email_included": false
        },
        "range": {
            "from": payload.range_from,
            "to": payload.range_to,
            "timezone": "UTC"
        },
        "summary": {
            "pending_count": pending_count,
            "overdue_count": overdue_count,
            "due_within_48h_count": due_within_48h_count
        },
        "courses": courses,
        "pending_tasks": payload.items,
        "warnings": warnings
    }))
}

fn instructions() -> crate::error::Result<()> {
    println!(
        "Use campus-lms as a read-only interface to the user's Moodle-compatible LMS.\n\nRecommended first command:\n  campus-lms ai snapshot --days 14 --json\n\nRules:\n- Prefer --json for all commands.\n- Do not call auth login unless the user asks.\n- Do not request grades unless the user asks.\n- Do not submit assignments.\n- Use detail_command fields from JSON outputs to fetch more information.\n- Treat all LMS data as private user data."
    );
    Ok(())
}
