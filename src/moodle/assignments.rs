// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    cache,
    cli::{ensure_cache_flags, ensure_max_chars, warning_detail_limit, AssignmentShowArgs, Cli},
    config,
    dto::{
        warning_report_with_options, AssignmentDetailOutput, AssignmentOutput,
        AssignmentSubmissionOutput, AttachmentOutput, Warning,
    },
    error::CampusError,
    moodle::{
        api::MoodleApi,
        client::client_from_profile_data,
        models::{Assignment, AssignmentsResponse, SubmissionStatusResponse},
    },
    output,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssignmentIndexItem {
    pub course_id: String,
    pub course_name: Option<String>,
    pub assignment: Assignment,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssignmentIndexPayload {
    pub items: Vec<AssignmentIndexItem>,
    pub warnings: Vec<Warning>,
}

pub fn show(cli: &Cli, args: &AssignmentShowArgs) -> crate::error::Result<()> {
    ensure_cache_flags(args.refresh, args.offline).map_err(|err| err.with_json(cli.json))?;
    ensure_max_chars(args.max_chars).map_err(|err| err.with_json(cli.json))?;
    let id = parse_assign_id(&args.id).map_err(|err| err.with_json(cli.json))?;
    let config = config::load(cli).map_err(|err| err.with_json(cli.json))?;
    let profile_name = config::selected_profile_name(cli, &config);
    let profile = config::active_profile(cli, &config).map_err(|err| err.with_json(cli.json))?;
    let namespace = cache::profile_namespace(&profile_name, profile, None);
    let cache_key = cache::key(
        "assignment-show",
        &format!("v2:{namespace}:{id}:{}", args.include_html),
    );
    if let Some(value) = cache::get::<serde_json::Value>(
        &cache_key,
        Duration::from_secs(600),
        args.refresh,
        args.offline,
    )
    .map_err(|err| err.with_json(cli.json))?
    {
        return output::print_json(&value);
    }

    let client = client_from_profile_data(cli, &profile_name, profile)
        .map_err(|err| err.with_json(cli.json))?;
    let all = if args.offline {
        fetch_assignments(cli, args.refresh, args.offline).map_err(|err| err.with_json(cli.json))?
    } else {
        fetch_assignments_from_api(&client).map_err(|err| err.with_json(cli.json))?
    };
    let item = all
        .items
        .into_iter()
        .find(|item| item.assignment.id == id)
        .ok_or_else(|| CampusError::NotFound {
            message: format!("assignment {} was not found in visible courses", args.id),
            json: cli.json,
        })?;

    let mut warnings = filter_assignment_warnings(all.warnings, id, item.course_id.as_str());
    let submission = match client.submission_status(id) {
        Ok(submission) => {
            warnings.extend(submission.warnings.iter().map(Warning::from_moodle_warning));
            submission
        }
        Err(err @ CampusError::AuthRequired { .. })
        | Err(err @ CampusError::AuthExpired { .. })
        | Err(err @ CampusError::PermissionDenied { .. }) => return Err(err.with_json(cli.json)),
        Err(err) => {
            warnings.push(Warning::new(
                "SUBMISSION_STATUS_UNAVAILABLE",
                err.to_string(),
                Some(
                    "Assignment details were returned, but submission status could not be fetched."
                        .to_string(),
                ),
            ));
            SubmissionStatusResponse {
                lastattempt: None,
                warnings: vec![],
            }
        }
    };

    let description_html = item.assignment.intro.clone();
    let description_text = description_html
        .as_deref()
        .map(html_to_text)
        .unwrap_or_default();
    let original_len = description_text.chars().count();
    let truncated_text = truncate_chars(&description_text, args.max_chars);
    let assignment_url = client
        .base_url
        .join(&format!(
            "mod/assign/view.php?id={}",
            item.assignment.cmid.unwrap_or(item.assignment.id)
        ))
        .map(|url| url.to_string())
        .unwrap_or_default();

    let attachments = item
        .assignment
        .introattachments
        .iter()
        .map(|file| {
            let name = file
                .filename
                .clone()
                .unwrap_or_else(|| "attachment".to_string());
            AttachmentOutput {
                id: format!(
                    "file:sha256:{}",
                    sha_file_id(&name, file.fileurl.as_deref().unwrap_or(""))
                ),
                name,
                mime_type: file.mimetype.clone(),
                size_bytes: file.filesize,
                download_url_available: file.fileurl.is_some(),
                download_command: None,
            }
        })
        .collect();

    let detail_limit = warning_detail_limit(cli).map_err(|err| err.with_json(cli.json))?;
    let visible_item_ids = [item.assignment.id]
        .into_iter()
        .chain(item.assignment.cmid)
        .collect();
    let report = warning_report_with_options(warnings, detail_limit, &visible_item_ids);
    let value = AssignmentOutput {
        schema_version: "campus-lms.assignment.v1",
        generated_at: output::generated_at(),
        assignment: AssignmentDetailOutput {
            id: format!("assign:{}", item.assignment.id),
            moodle_id: item.assignment.id,
            cmid: item.assignment.cmid,
            course_id: item.course_id.clone(),
            course_name: item.course_name.clone(),
            title: item.assignment.name.clone(),
            due_at: ts(item.assignment.duedate),
            allows_submission_from: ts(item.assignment.allowsubmissionsfromdate),
            cutoff_at: ts(item.assignment.cutoffdate),
            description_text: truncated_text,
            description_truncated: original_len > args.max_chars,
            description_original_length_chars: original_len,
            description_html: if args.include_html {
                description_html.clone()
            } else {
                None
            },
            description_html_available: description_html.is_some(),
            attachments,
            submission: AssignmentSubmissionOutput {
                status: submission
                    .lastattempt
                    .as_ref()
                    .and_then(|a| a.submission.as_ref())
                    .and_then(|s| s.status.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
                last_modified_at: submission
                    .lastattempt
                    .as_ref()
                    .and_then(|a| a.submission.as_ref())
                    .and_then(|s| ts(s.timemodified)),
                grading_status: submission
                    .lastattempt
                    .as_ref()
                    .and_then(|a| a.gradingstatus.clone()),
            },
            url: assignment_url,
        },
        warnings_summary: report.summary,
        warnings_total_count: report.total_count,
        warnings_returned_count: report.returned_count,
        warnings_details_truncated: report.details_truncated,
        warnings: report.details,
    };
    let cached_value = serde_json::to_value(&value).map_err(|err| CampusError::Parse {
        message: format!("failed to serialize assignment output: {err}"),
        json: cli.json,
    })?;
    cache::set(&cache_key, &cached_value).map_err(|err| err.with_json(cli.json))?;
    output::print_json(&value)
}

pub fn fetch_assignments(
    cli: &Cli,
    refresh: bool,
    offline: bool,
) -> crate::error::Result<AssignmentIndexPayload> {
    ensure_cache_flags(refresh, offline)?;
    let config = config::load(cli)?;
    let profile_name = config::selected_profile_name(cli, &config);
    let profile = config::active_profile(cli, &config)?;
    let namespace = cache::profile_namespace(&profile_name, profile, None);
    let cache_key = cache::key("assignments", &namespace);
    if let Some(payload) = cache::get(&cache_key, Duration::from_secs(600), refresh, offline)? {
        return Ok(payload);
    }
    if offline {
        return Err(CampusError::cache("offline cache miss for assignments"));
    }
    let client = client_from_profile_data(cli, &profile_name, profile)?;
    let payload = fetch_assignments_from_api(&client)?;
    cache::set(&cache_key, &redact_assignment_cache(&payload))?;
    Ok(payload)
}

fn fetch_assignments_from_api<T: MoodleApi>(
    client: &T,
) -> crate::error::Result<AssignmentIndexPayload> {
    let response: AssignmentsResponse = client.assignments(&[])?;
    let warnings = response
        .warnings
        .iter()
        .map(Warning::from_moodle_warning)
        .collect();
    let mut items = Vec::new();
    for course in response.courses {
        for assignment in course.assignments {
            items.push(AssignmentIndexItem {
                course_id: format!("course:{}", course.id),
                course_name: course.fullname.clone().or(course.shortname.clone()),
                assignment,
            });
        }
    }
    Ok(AssignmentIndexPayload { items, warnings })
}

pub fn parse_assign_id(input: &str) -> crate::error::Result<i64> {
    let raw = input.strip_prefix("assign:").unwrap_or(input);
    let id = raw.parse::<i64>().map_err(|_| {
        CampusError::invalid_argument("assignment id must look like assign:12345", None)
    })?;
    if id <= 0 {
        return Err(CampusError::invalid_argument(
            "assignment id must be a positive integer.",
            Some("Use an id such as assign:12345."),
        ));
    }
    Ok(id)
}

pub fn ts(timestamp: Option<i64>) -> Option<String> {
    let timestamp = timestamp?;
    if timestamp <= 0 {
        return None;
    }
    OffsetDateTime::from_unix_timestamp(timestamp)
        .ok()
        .and_then(|dt| dt.format(&Rfc3339).ok())
}

fn html_to_text(html: &str) -> String {
    html2text::from_read(html.as_bytes(), 80)
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn sha_file_id(name: &str, url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(url.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn redact_assignment_cache(payload: &AssignmentIndexPayload) -> AssignmentIndexPayload {
    let mut redacted = payload.clone();
    for item in &mut redacted.items {
        for file in &mut item.assignment.introattachments {
            file.fileurl = None;
        }
    }
    redacted
}

fn filter_assignment_warnings(
    warnings: Vec<Warning>,
    assignment_id: i64,
    course_id: &str,
) -> Vec<Warning> {
    let course_id = course_id
        .strip_prefix("course:")
        .and_then(|id| id.parse::<i64>().ok());
    warnings
        .into_iter()
        .filter(|warning| match (warning.item.as_deref(), warning.itemid) {
            (Some("assign" | "assignment"), Some(itemid)) => itemid == assignment_id,
            (Some("course"), Some(itemid)) => course_id == Some(itemid),
            (_, Some(_)) => false,
            _ => false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prefixed_assignment_id() {
        assert_eq!(parse_assign_id("assign:123").unwrap(), 123);
        assert_eq!(parse_assign_id("123").unwrap(), 123);
        assert!(parse_assign_id("course:123").is_err());
        assert!(parse_assign_id("assign:0").is_err());
        assert!(parse_assign_id("-1").is_err());
    }
}
