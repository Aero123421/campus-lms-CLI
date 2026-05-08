// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    cache,
    cli::{ensure_cache_flags, AssignmentShowArgs, Cli},
    error::CampusError,
    moodle::{
        client_from_profile,
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

pub fn show(cli: &Cli, args: &AssignmentShowArgs) -> crate::error::Result<()> {
    ensure_cache_flags(args.refresh, args.offline).map_err(|err| err.with_json(cli.json))?;
    let id = parse_assign_id(&args.id).map_err(|err| err.with_json(cli.json))?;
    let cache_key = cache::key(
        "assignment-show",
        &format!("{}:{}:{}", cli.profile, id, args.include_html),
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

    let client = client_from_profile(cli).map_err(|err| err.with_json(cli.json))?;
    let all = fetch_assignments(cli, args.refresh, args.offline)
        .map_err(|err| err.with_json(cli.json))?;
    let item = all
        .into_iter()
        .find(|item| item.assignment.id == id)
        .ok_or_else(|| CampusError::NotFound {
            message: format!("assignment {} was not found in visible courses", args.id),
            json: cli.json,
        })?;

    let mut warnings = Vec::new();
    let submission = match client.call::<SubmissionStatusResponse>(
        "mod_assign_get_submission_status",
        serde_json::json!({ "assignid": id }),
    ) {
        Ok(submission) => submission,
        Err(err @ CampusError::AuthRequired { .. })
        | Err(err @ CampusError::AuthExpired { .. })
        | Err(err @ CampusError::PermissionDenied { .. }) => return Err(err.with_json(cli.json)),
        Err(err) => {
            warnings.push(serde_json::json!({
                "code": "SUBMISSION_STATUS_UNAVAILABLE",
                "message": err.to_string(),
                "hint": "Assignment details were returned, but submission status could not be fetched."
            }));
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

    let value = serde_json::json!({
        "schema_version": "campus-lms.assignment.v1",
        "generated_at": output::generated_at(),
        "assignment": {
            "id": format!("assign:{}", item.assignment.id),
            "moodle_id": item.assignment.id,
            "cmid": item.assignment.cmid,
            "course_id": item.course_id.clone(),
            "course_name": item.course_name.clone(),
            "title": item.assignment.name.clone(),
            "due_at": ts(item.assignment.duedate),
            "allows_submission_from": ts(item.assignment.allowsubmissionsfromdate),
            "cutoff_at": ts(item.assignment.cutoffdate),
            "description_text": truncated_text,
            "description_truncated": original_len > args.max_chars,
            "description_original_length_chars": original_len,
            "description_html": if args.include_html { description_html.clone() } else { None },
            "description_html_available": description_html.is_some(),
            "attachments": item.assignment.introattachments.iter().map(|file| {
                let name = file.filename.clone().unwrap_or_else(|| "attachment".to_string());
                serde_json::json!({
                    "id": format!("file:sha256:{}", sha_file_id(&name, file.fileurl.as_deref().unwrap_or(""))),
                    "name": name,
                    "mime_type": file.mimetype.clone(),
                    "size_bytes": file.filesize,
                    "download_url_available": file.fileurl.is_some(),
                    "download_command": null
                })
            }).collect::<Vec<_>>(),
            "submission": {
                "status": submission.lastattempt.as_ref().and_then(|a| a.submission.as_ref()).and_then(|s| s.status.clone()).unwrap_or_else(|| "unknown".to_string()),
                "last_modified_at": submission.lastattempt.as_ref().and_then(|a| a.submission.as_ref()).and_then(|s| ts(s.timemodified)),
                "grading_status": submission.lastattempt.as_ref().and_then(|a| a.gradingstatus.clone())
            },
            "url": assignment_url
        },
        "warnings": warnings
    });
    cache::set(&cache_key, &value).map_err(|err| err.with_json(cli.json))?;
    output::print_json(&value)
}

pub fn fetch_assignments(
    cli: &Cli,
    refresh: bool,
    offline: bool,
) -> crate::error::Result<Vec<AssignmentIndexItem>> {
    ensure_cache_flags(refresh, offline)?;
    let cache_key = cache::key("assignments", &cli.profile);
    if let Some(items) = cache::get(&cache_key, Duration::from_secs(600), refresh, offline)? {
        return Ok(items);
    }
    if offline {
        return Err(CampusError::cache("offline cache miss for assignments"));
    }
    let client = client_from_profile(cli)?;
    let response: AssignmentsResponse =
        client.call("mod_assign_get_assignments", serde_json::json!({}))?;
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
    cache::set(&cache_key, &items)?;
    Ok(items)
}

pub fn parse_assign_id(input: &str) -> crate::error::Result<i64> {
    let raw = input.strip_prefix("assign:").unwrap_or(input);
    raw.parse::<i64>().map_err(|_| {
        CampusError::invalid_argument("assignment id must look like assign:12345", None)
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prefixed_assignment_id() {
        assert_eq!(parse_assign_id("assign:123").unwrap(), 123);
        assert_eq!(parse_assign_id("123").unwrap(), 123);
        assert!(parse_assign_id("course:123").is_err());
    }
}
