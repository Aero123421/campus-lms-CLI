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
        AssignmentSubmissionOutput, AttachmentOutput, CacheMeta, Warning,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AssignmentDetailCache {
    item: AssignmentIndexItem,
    submission: Option<SubmissionStatusResponse>,
    warnings: Vec<Warning>,
    #[serde(default)]
    description_text: Option<String>,
    #[serde(default)]
    description_html_available: bool,
}

pub fn show(cli: &Cli, args: &AssignmentShowArgs) -> crate::error::Result<()> {
    ensure_cache_flags(args.refresh, args.offline).map_err(|err| err.with_json(cli.json))?;
    ensure_max_chars(args.max_chars).map_err(|err| err.with_json(cli.json))?;
    if args.offline && args.no_cache {
        return Err(CampusError::invalid_argument(
            "--offline cannot be used with --no-cache.",
            Some("Use --offline to read cache, or --no-cache to force a live read."),
        )
        .with_json(cli.json));
    }

    let id = parse_assign_id(&args.id).map_err(|err| err.with_json(cli.json))?;
    let config = config::load(cli).map_err(|err| err.with_json(cli.json))?;
    let profile_name = config::selected_profile_name(cli, &config);
    let profile = config::active_profile(cli, &config).map_err(|err| err.with_json(cli.json))?;
    let namespace = cache::profile_namespace(&profile_name, profile, None);
    prune_profile_cache(&namespace, profile).map_err(|err| err.with_json(cli.json))?;
    let detail_key = cache::profile_key("assignment-show", &namespace, &format!("v4:{id}"));
    let ttl_seconds = profile.cache_ttl_seconds;
    let ttl = Duration::from_secs(ttl_seconds);

    if !args.no_cache {
        if let Some(entry) = cache::get_entry_optional::<AssignmentDetailCache>(
            &detail_key,
            ttl,
            args.refresh,
            args.offline,
        )
        .map_err(|err| err.with_json(cli.json))?
        {
            let cache_meta = CacheMeta::from_entry(&entry, ttl_seconds);
            let value = render_assignment_output(cli, args, entry.value, cache_meta, profile)?;
            return output::print_json(&value);
        }
    }

    if args.offline {
        let entry = cached_assignment_index(&profile_name, profile, ttl, id)
            .map_err(|err| err.with_json(cli.json))?
            .ok_or_else(|| CampusError::cache("offline cache miss for assignments"))?;
        let cache_meta = CacheMeta::from_entry(&entry, ttl_seconds);
        let payload = entry.value;
        let item = find_assignment(&payload, id, cli.json)?;
        let mut warnings =
            filter_assignment_warnings(payload.warnings, id, item.course_id.as_str());
        warnings.push(Warning::new(
            "ASSIGNMENT_DETAIL_RECONSTRUCTED_FROM_INDEX_CACHE",
            "Assignment detail was reconstructed from cached assignment index data.",
            Some(
                "Submission status may be unknown because --offline does not contact Moodle."
                    .to_string(),
            ),
        ));
        let value = render_assignment_output(
            cli,
            args,
            AssignmentDetailCache {
                item,
                submission: None,
                warnings,
                description_text: None,
                description_html_available: false,
            },
            cache_meta,
            profile,
        )?;
        return output::print_json(&value);
    }

    let client = client_from_profile_data(cli, &profile_name, profile)
        .map_err(|err| err.with_json(cli.json))?;
    let payload =
        fetch_assignments_from_api(&client, &[]).map_err(|err| err.with_json(cli.json))?;
    let item = find_assignment(&payload, id, cli.json)?;
    let mut warnings = filter_assignment_warnings(payload.warnings, id, item.course_id.as_str());
    let submission = match client.submission_status(id) {
        Ok(submission) => {
            warnings.extend(submission.warnings.iter().map(Warning::from_moodle_warning));
            Some(submission)
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
            None
        }
    };

    let detail = AssignmentDetailCache {
        item,
        submission,
        warnings,
        description_text: None,
        description_html_available: false,
    };
    if !args.no_cache {
        cache::set(
            &detail_key,
            &redact_assignment_detail_cache(&detail, args.include_html),
        )
        .map_err(|err| err.with_json(cli.json))?;
    }
    let value =
        render_assignment_output(cli, args, detail, CacheMeta::fresh(ttl_seconds), profile)?;
    output::print_json(&value)
}

fn render_assignment_output(
    cli: &Cli,
    args: &AssignmentShowArgs,
    detail: AssignmentDetailCache,
    cache_meta: CacheMeta,
    profile: &config::Profile,
) -> crate::error::Result<AssignmentOutput> {
    let item = detail.item;
    let warnings = detail.warnings;
    let submission = detail.submission.as_ref();
    let description_html = item.assignment.intro.clone();
    let description_text = detail.description_text.unwrap_or_else(|| {
        description_html
            .as_deref()
            .map(html_to_text)
            .unwrap_or_default()
    });
    let original_len = description_text.chars().count();
    let truncated_text = truncate_chars(&description_text, args.max_chars);
    let assignment_url = profile
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
                id: stable_attachment_id(item.assignment.id, file),
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
    let status_source = match (submission.is_some(), cache_meta.used) {
        (true, true) => "cache",
        (true, false) => "live",
        (false, _) => "unavailable",
    }
    .to_string();
    let submission_status = submission
        .and_then(|submission| submission.lastattempt.as_ref())
        .and_then(|attempt| attempt.submission.as_ref())
        .and_then(|submission| submission.status.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let last_modified_at = submission
        .and_then(|submission| submission.lastattempt.as_ref())
        .and_then(|attempt| attempt.submission.as_ref())
        .and_then(|submission| ts(submission.timemodified));
    let grading_status = submission
        .and_then(|submission| submission.lastattempt.as_ref())
        .and_then(|attempt| attempt.gradingstatus.clone());

    Ok(AssignmentOutput {
        schema_version: "campus-lms.assignment.v2".to_string(),
        generated_at: output::generated_at(),
        cache: cache_meta,
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
                description_html
            } else {
                None
            },
            description_html_available: item.assignment.intro.is_some()
                || detail.description_html_available,
            attachments,
            submission: AssignmentSubmissionOutput {
                status: submission_status,
                status_source,
                last_modified_at,
                grading_status,
            },
            url: assignment_url,
        },
        warnings_summary: report.summary,
        warnings_total_count: report.total_count,
        warnings_returned_count: report.returned_count,
        warnings_details_truncated: report.details_truncated,
        warnings: report.details,
    })
}

pub fn fetch_assignments_for_courses(
    cli: &Cli,
    refresh: bool,
    offline: bool,
    course_ids: &[i64],
) -> crate::error::Result<AssignmentIndexPayload> {
    ensure_cache_flags(refresh, offline)?;
    let config = config::load(cli)?;
    let profile_name = config::selected_profile_name(cli, &config);
    let profile = config::active_profile(cli, &config)?;
    let namespace = cache::profile_namespace(&profile_name, profile, None);
    prune_profile_cache(&namespace, profile)?;
    let cache_key = assignments_cache_key(&namespace, course_ids);
    let ttl = Duration::from_secs(profile.cache_ttl_seconds);
    if let Some(payload) = cache::get(&cache_key, ttl, refresh, offline)? {
        return Ok(payload);
    }
    if offline {
        return Err(CampusError::cache("offline cache miss for assignments"));
    }
    let client = client_from_profile_data(cli, &profile_name, profile)?;
    let payload = fetch_assignments_from_api(&client, course_ids)?;
    cache::set(&cache_key, &redact_assignment_cache(&payload))?;
    Ok(payload)
}

fn fetch_assignments_from_api<T: MoodleApi>(
    client: &T,
    course_ids: &[i64],
) -> crate::error::Result<AssignmentIndexPayload> {
    let response: AssignmentsResponse = client.assignments(course_ids)?;
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

fn assignments_cache_key(namespace: &str, course_ids: &[i64]) -> String {
    let scope = if course_ids.is_empty() {
        "all".to_string()
    } else {
        course_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    };
    cache::profile_key("assignments", namespace, &format!("v3:courses={scope}"))
}

fn cached_assignment_index(
    profile_name: &str,
    profile: &config::Profile,
    ttl: Duration,
    assignment_id: i64,
) -> crate::error::Result<Option<cache::CacheEntry<AssignmentIndexPayload>>> {
    let namespace = cache::profile_namespace(profile_name, profile, None);
    let cache_key = assignments_cache_key(&namespace, &[]);
    if let Some(entry) =
        cache::get_entry_optional::<AssignmentIndexPayload>(&cache_key, ttl, false, true)?
    {
        if entry
            .value
            .items
            .iter()
            .any(|item| item.assignment.id == assignment_id)
        {
            return Ok(Some(entry));
        }
    }
    Ok(
        cache::profile_entries_with_prefix::<AssignmentIndexPayload>(
            &namespace,
            "assignments-",
            ttl,
            true,
        )?
        .into_iter()
        .find(|entry| {
            entry
                .value
                .items
                .iter()
                .any(|item| item.assignment.id == assignment_id)
        }),
    )
}

fn find_assignment(
    payload: &AssignmentIndexPayload,
    id: i64,
    json: bool,
) -> crate::error::Result<AssignmentIndexItem> {
    payload
        .items
        .iter()
        .find(|item| item.assignment.id == id)
        .cloned()
        .ok_or_else(|| CampusError::NotFound {
            message: format!("assignment assign:{id} was not found in visible courses"),
            json,
        })
}

fn prune_profile_cache(namespace: &str, profile: &config::Profile) -> crate::error::Result<()> {
    cache::prune_namespace(
        namespace,
        Duration::from_secs(profile.cache_retention_seconds),
    )
    .map(|_| ())
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

fn stable_attachment_id(assign_id: i64, file: &crate::moodle::models::MoodleFile) -> String {
    let mut hasher = Sha256::new();
    hasher.update(assign_id.to_string().as_bytes());
    hasher.update(file.contenthash.as_deref().unwrap_or("").as_bytes());
    hasher.update(file.pathnamehash.as_deref().unwrap_or("").as_bytes());
    hasher.update(file.filename.as_deref().unwrap_or("").as_bytes());
    hasher.update(file.filepath.as_deref().unwrap_or("").as_bytes());
    hasher.update(file.filesize.unwrap_or_default().to_string().as_bytes());
    hasher.update(file.mimetype.as_deref().unwrap_or("").as_bytes());
    format!("file:sha256:{:x}", hasher.finalize())
}

fn redact_assignment_cache(payload: &AssignmentIndexPayload) -> AssignmentIndexPayload {
    let mut redacted = payload.clone();
    for item in &mut redacted.items {
        item.assignment.intro = None;
        item.assignment.introattachments.clear();
    }
    redacted
}

fn redact_assignment_detail_cache(
    detail: &AssignmentDetailCache,
    cache_html: bool,
) -> AssignmentDetailCache {
    let mut redacted = detail.clone();
    redacted.description_text = redacted
        .description_text
        .or_else(|| redacted.item.assignment.intro.as_deref().map(html_to_text));
    redacted.description_html_available = redacted.item.assignment.intro.is_some();
    if cache_html {
        redacted.item.assignment.intro = redacted.item.assignment.intro.map(sanitize_cached_html);
    } else {
        redacted.item.assignment.intro = None;
    }
    for file in &mut redacted.item.assignment.introattachments {
        file.fileurl = None;
    }
    redacted
}

fn sanitize_cached_html(input: String) -> String {
    let mut output = input;
    for key in ["wstoken", "token", "sesskey"] {
        output = strip_query_value(&output, key);
    }
    output
}

fn strip_query_value(input: &str, key: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;
    let needle_a = format!("{key}=");
    while let Some(index) = rest.find(&needle_a) {
        output.push_str(&rest[..index]);
        output.push_str(&needle_a);
        let tail = &rest[index + needle_a.len()..];
        let end = tail.find(['&', '"', '\'', '<', '>']).unwrap_or(tail.len());
        output.push_str("REDACTED");
        rest = &tail[end..];
    }
    output.push_str(rest);
    output
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
    use crate::moodle::models::MoodleFile;

    #[test]
    fn parses_prefixed_assignment_id() {
        assert_eq!(parse_assign_id("assign:123").unwrap(), 123);
        assert_eq!(parse_assign_id("123").unwrap(), 123);
        assert!(parse_assign_id("course:123").is_err());
        assert!(parse_assign_id("assign:0").is_err());
        assert!(parse_assign_id("-1").is_err());
    }

    #[test]
    fn assignment_cache_key_ignores_output_only_options() {
        let key_a = assignments_cache_key("profile", &[]);
        let key_b = assignments_cache_key("profile", &[]);
        assert_eq!(key_a, key_b);
        assert_ne!(key_a, assignments_cache_key("profile", &[123]));
    }

    #[test]
    fn attachment_id_does_not_depend_on_file_url() {
        let mut file = MoodleFile {
            filename: Some("report.pdf".to_string()),
            filepath: Some("/".to_string()),
            contenthash: None,
            pathnamehash: None,
            filesize: Some(10),
            mimetype: Some("application/pdf".to_string()),
            fileurl: Some(
                "https://example.edu/pluginfile.php/1/report.pdf?token=secret".to_string(),
            ),
        };
        let live_id = stable_attachment_id(42, &file);
        file.fileurl = None;
        assert_eq!(live_id, stable_attachment_id(42, &file));
    }

    #[test]
    fn index_cache_removes_assignment_body_and_attachments() {
        let payload = AssignmentIndexPayload {
            items: vec![AssignmentIndexItem {
                course_id: "course:1".to_string(),
                course_name: None,
                assignment: Assignment {
                    id: 42,
                    cmid: Some(100),
                    course: Some(1),
                    name: Some("Report".to_string()),
                    intro: Some("<p>Private body</p>".to_string()),
                    duedate: None,
                    allowsubmissionsfromdate: None,
                    cutoffdate: None,
                    introattachments: vec![MoodleFile {
                        filename: Some("report.pdf".to_string()),
                        filepath: Some("/".to_string()),
                        contenthash: None,
                        pathnamehash: None,
                        filesize: Some(10),
                        mimetype: Some("application/pdf".to_string()),
                        fileurl: Some(
                            "https://example.edu/pluginfile.php/1/report.pdf".to_string(),
                        ),
                    }],
                },
            }],
            warnings: Vec::new(),
        };
        let redacted = redact_assignment_cache(&payload);
        assert!(redacted.items[0].assignment.intro.is_none());
        assert!(redacted.items[0].assignment.introattachments.is_empty());
    }

    #[test]
    fn cached_html_redacts_common_secret_query_values() {
        let html = r#"<a href="https://example.edu/pluginfile.php/1/file.pdf?token=secret&wstoken=abc&x=1">file</a><img src="/x?sesskey=s3">"#;
        let sanitized = sanitize_cached_html(html.to_string());
        assert!(sanitized.contains("token=REDACTED"));
        assert!(sanitized.contains("wstoken=REDACTED"));
        assert!(sanitized.contains("sesskey=REDACTED"));
        assert!(!sanitized.contains("secret"));
        assert!(!sanitized.contains("abc"));
        assert!(!sanitized.contains("s3"));
    }
}
