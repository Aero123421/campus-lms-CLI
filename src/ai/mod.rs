// SPDX-License-Identifier: Apache-2.0

use crate::{
    cli::{ensure_days, ensure_max_items, AiCommand, Cli, TodoArgs},
    config,
    dto::{
        warning_report, AiSnapshotOutput, DateRange, PrivacyOutput, SnapshotCourse, SummaryOutput,
        Warning,
    },
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
    ensure_days(args.days).map_err(|err| err.with_json(cli.json))?;
    ensure_max_items(Some(args.max_items)).map_err(|err| err.with_json(cli.json))?;
    let mut warnings = Vec::new();
    let mut unsupported_flags = Vec::new();
    if args.include_grades {
        unsupported_flags.push("include_grades".to_string());
        warnings.push(Warning::new(
            "GRADES_NOT_IMPLEMENTED",
            "--include-grades is accepted for CLI compatibility, but grade retrieval is not implemented in this MVP.",
            Some("Do not assume grades are present in this snapshot.".to_string()),
        ));
    }
    if args.include_feedback {
        unsupported_flags.push("include_feedback".to_string());
        warnings.push(Warning::new(
            "FEEDBACK_NOT_IMPLEMENTED",
            "--include-feedback is accepted for CLI compatibility, but feedback retrieval is not implemented in this MVP.",
            Some("Use assignment detail commands only for assignment text and submission metadata.".to_string()),
        ));
    }
    let todo_args = TodoArgs {
        days: args.days,
        max_items: Some(args.max_items),
        refresh: args.refresh,
        offline: args.offline,
        include_submitted: false,
        course: None,
    };
    let fetched = calendar::fetch(cli, &todo_args).map_err(|err| err.with_json(cli.json))?;
    let payload = fetched.payload;
    let total_matching_count = payload.total_items_before_limit;
    warnings.extend(payload.warnings.clone());
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

    let mut courses: Vec<SnapshotCourse> = Vec::new();
    for item in &payload.items {
        if let Some(course_id) = &item.course_id {
            if !courses.iter().any(|course| course.id == *course_id) {
                courses.push(SnapshotCourse {
                    id: course_id.clone(),
                    name: item.course_name.clone(),
                });
            }
        }
    }

    let config = config::load(cli).map_err(|err| err.with_json(cli.json))?;
    if config.privacy.include_grades_in_ai_snapshot
        || config.privacy.include_feedback_in_ai_snapshot
    {
        warnings.push(Warning::new(
            "PRIVACY_CONFIG_NOT_IMPLEMENTED",
            "Privacy config for grades/feedback is recorded but those data types are not fetched by this CLI yet.",
            Some("The snapshot remains grades_included=false and feedback_included=false.".to_string()),
        ));
    }

    let timezone = config.output.timezone.clone();
    let report = warning_report(warnings);
    output::print_json(&AiSnapshotOutput {
        schema_version: "campus-lms.ai_snapshot.v1",
        generated_at: output::generated_at(),
        privacy: PrivacyOutput {
            grades_included: false,
            feedback_included: false,
            user_email_included: false,
        },
        range: DateRange {
            from: payload.range_from,
            to: payload.range_to,
            timezone,
        },
        summary: SummaryOutput {
            returned_count: payload.items.len(),
            total_matching_count,
            limited: payload.items.len() < total_matching_count,
            pending_count,
            pending_returned_count: pending_count,
            pending_total_matching_count: total_matching_count,
            overdue_count,
            due_within_48h_count,
        },
        courses: courses.clone(),
        courses_in_pending_tasks: courses,
        pending_tasks: payload.items,
        unsupported_flags,
        warnings_summary: report.summary,
        warnings_total_count: report.total_count,
        warnings_returned_count: report.returned_count,
        warnings_details_truncated: report.details_truncated,
        warnings: report.details,
    })
}

fn instructions() -> crate::error::Result<()> {
    println!(
        "Use campus-lms as a read-only interface to the user's Moodle-compatible LMS.\n\nRecommended first command:\n  campus-lms ai snapshot --days 14 --json\n\nRules:\n- Prefer --json for all commands.\n- Do not call auth login unless the user asks.\n- Do not request grades unless the user asks.\n- Do not submit assignments.\n- Use detail_command fields from JSON outputs to fetch more information.\n- Treat all LMS data as private user data."
    );
    Ok(())
}
