// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct Warning {
    #[schemars(regex(pattern = "^[A-Z0-9_]+$"))]
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub itemid: Option<i64>,
}

impl Warning {
    pub fn new(code: impl Into<String>, message: impl Into<String>, hint: Option<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            hint,
            item: None,
            itemid: None,
        }
    }

    pub fn from_moodle_warning(warning: &crate::moodle::models::MoodleWarning) -> Self {
        let message = warning
            .message
            .clone()
            .unwrap_or_else(|| "Moodle returned a warning without a message.".to_string());
        Self {
            code: stable_warning_code(warning.warningcode.as_deref(), &message),
            message,
            hint: Some("The LMS may have returned partial results.".to_string()),
            item: warning.item.clone(),
            itemid: warning.itemid,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct WarningSummary {
    #[schemars(regex(pattern = "^[A-Z0-9_]+$"))]
    pub code: String,
    pub count: usize,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WarningReport {
    pub summary: Vec<WarningSummary>,
    pub details: Vec<Warning>,
    pub total_count: usize,
    pub returned_count: usize,
    pub details_truncated: bool,
}

pub fn warning_report(warnings: Vec<Warning>) -> WarningReport {
    let total_count = warnings.len();
    let mut groups: BTreeMap<(String, String, Option<String>), usize> = BTreeMap::new();
    for warning in &warnings {
        *groups
            .entry((
                warning.code.clone(),
                warning.message.clone(),
                warning.hint.clone(),
            ))
            .or_default() += 1;
    }
    let summary = groups
        .into_iter()
        .map(|((code, message, hint), count)| WarningSummary {
            code,
            count,
            message,
            hint,
        })
        .collect();
    let detail_limit = 5;
    let details_truncated = warnings.len() > detail_limit;
    let details: Vec<Warning> = warnings.into_iter().take(detail_limit).collect();
    let returned_count = details.len();
    WarningReport {
        summary,
        details,
        total_count,
        returned_count,
        details_truncated,
    }
}

fn stable_warning_code(code: Option<&str>, message: &str) -> String {
    let lower = message.to_ascii_lowercase();
    if lower.contains("no access rights in module context") {
        return "ACCESS_DENIED_IN_MODULE_CONTEXT".to_string();
    }
    normalize_warning_code(code)
}

fn normalize_warning_code(code: Option<&str>) -> String {
    let Some(code) = code else {
        return "MOODLE_WARNING".to_string();
    };
    let normalized = code
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if normalized.is_empty() {
        "MOODLE_WARNING".to_string()
    } else {
        normalized
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct CacheMeta {
    pub used: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_seconds: Option<u64>,
    pub ttl_seconds: u64,
    pub stale: bool,
}

impl CacheMeta {
    pub fn fresh(ttl_seconds: u64) -> Self {
        Self {
            used: false,
            fetched_at: Some(crate::output::generated_at()),
            age_seconds: None,
            ttl_seconds,
            stale: false,
        }
    }

    pub fn from_entry<T>(entry: &crate::cache::CacheEntry<T>, ttl_seconds: u64) -> Self {
        Self {
            used: true,
            fetched_at: entry.fetched_at.clone(),
            age_seconds: Some(entry.age.as_secs()),
            ttl_seconds,
            stale: entry.stale,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct DateRange {
    pub from: String,
    pub to: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AuthStatusOutput {
    pub schema_version: &'static str,
    pub generated_at: String,
    pub authenticated: bool,
    pub profile: String,
    pub active_profile: String,
    pub config_path: Option<String>,
    pub base_url: Option<String>,
    pub username: Option<String>,
    pub credential_target: Option<crate::keychain::CredentialTarget>,
    pub token_available: bool,
    pub token_readable: bool,
    #[schemars(regex(pattern = "^(verified|failed|not_requested|not_checked)$"))]
    pub live_status: String,
    #[schemars(regex(pattern = "^(ok|failed|not_run|not_checked)$"))]
    pub live_check: String,
    pub live_ok: Option<bool>,
    pub warnings: Vec<Warning>,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AuthLoginOutput {
    pub schema_version: &'static str,
    pub generated_at: String,
    pub authenticated: bool,
    pub profile: String,
    pub base_url: String,
    pub username: String,
    pub credential_target: crate::keychain::CredentialTarget,
    pub token_verified: bool,
    pub warnings: Vec<Warning>,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AuthImportTokenOutput {
    pub schema_version: &'static str,
    pub generated_at: String,
    pub authenticated: bool,
    pub profile: String,
    pub base_url: String,
    pub username: String,
    pub credential_target: crate::keychain::CredentialTarget,
    pub token_verified: bool,
    pub warnings: Vec<Warning>,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AuthLogoutOutput {
    pub schema_version: &'static str,
    pub generated_at: String,
    pub logged_out: bool,
    pub profile: String,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AuthVerifyOutput {
    pub schema_version: &'static str,
    pub generated_at: String,
    pub profile: String,
    pub active_profile: String,
    pub config_path: Option<String>,
    pub profile_configured: bool,
    pub base_url: Option<String>,
    pub username: Option<String>,
    pub credential_target: Option<crate::keychain::CredentialTarget>,
    pub backend_roundtrip_ok: bool,
    pub credential_backend_ok: bool,
    pub token_available: bool,
    pub token_readable: bool,
    #[schemars(regex(pattern = "^(verified|failed|not_requested|not_checked)$"))]
    pub live_status: String,
    #[schemars(regex(pattern = "^(ok|failed|not_run|not_checked)$"))]
    pub live_check: String,
    pub live_ok: Option<bool>,
    pub warnings: Vec<Warning>,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct UserInfo {
    #[schemars(regex(pattern = "^user:[1-9][0-9]*$"))]
    pub id: String,
    pub username: String,
    pub fullname: Option<String>,
    pub site_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct WhoamiOutput {
    pub schema_version: &'static str,
    pub generated_at: String,
    pub user: UserInfo,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct CourseItem {
    #[schemars(regex(pattern = "^course:[1-9][0-9]*$"))]
    pub id: String,
    pub moodle_id: i64,
    pub short_name: Option<String>,
    pub full_name: Option<String>,
    pub visible: bool,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct CoursesOutput {
    pub schema_version: &'static str,
    pub generated_at: String,
    pub cache: CacheMeta,
    pub courses: Vec<CourseItem>,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct TodoItem {
    #[schemars(regex(pattern = "^(calendar|assign):[1-9][0-9]*$"))]
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub course_id: Option<String>,
    pub course_name: Option<String>,
    pub title: Option<String>,
    pub due_at: Option<String>,
    pub due_in_seconds: Option<i64>,
    #[schemars(regex(
        pattern = "^(pending|submitted|completed_or_not_actionable|unknown|new|draft|reopened)$"
    ))]
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_reason: Option<String>,
    #[schemars(regex(pattern = "^(calendar_action|assignment_fallback|submission_status)$"))]
    pub status_source: String,
    #[schemars(regex(pattern = "^(overdue|high|medium|low|unknown)$"))]
    pub priority_hint: String,
    pub url: Option<String>,
    pub detail_command: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct TodoOutput {
    pub schema_version: &'static str,
    pub generated_at: String,
    pub range: DateRange,
    pub cache: CacheMeta,
    pub summary: TodoSummary,
    pub total_items_before_limit: usize,
    pub items: Vec<TodoItem>,
    pub warnings_summary: Vec<WarningSummary>,
    pub warnings_total_count: usize,
    pub warnings_returned_count: usize,
    pub warnings_details_truncated: bool,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct TodoSummary {
    pub returned_count: usize,
    pub total_matching_count: usize,
    pub limited: bool,
    pub overdue_count: usize,
    pub due_within_48h_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AttachmentOutput {
    #[schemars(regex(pattern = "^file:sha256:[a-f0-9]{64}$"))]
    pub id: String,
    pub name: String,
    pub mime_type: Option<String>,
    pub size_bytes: Option<i64>,
    pub download_url_available: bool,
    pub download_command: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AssignmentSubmissionOutput {
    pub status: String,
    pub last_modified_at: Option<String>,
    pub grading_status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AssignmentDetailOutput {
    #[schemars(regex(pattern = "^assign:[1-9][0-9]*$"))]
    pub id: String,
    pub moodle_id: i64,
    pub cmid: Option<i64>,
    #[schemars(regex(pattern = "^course:[1-9][0-9]*$"))]
    pub course_id: String,
    pub course_name: Option<String>,
    pub title: Option<String>,
    pub due_at: Option<String>,
    pub allows_submission_from: Option<String>,
    pub cutoff_at: Option<String>,
    pub description_text: String,
    pub description_truncated: bool,
    pub description_original_length_chars: usize,
    pub description_html: Option<String>,
    pub description_html_available: bool,
    pub attachments: Vec<AttachmentOutput>,
    pub submission: AssignmentSubmissionOutput,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AssignmentOutput {
    pub schema_version: &'static str,
    pub generated_at: String,
    pub assignment: AssignmentDetailOutput,
    pub warnings_summary: Vec<WarningSummary>,
    pub warnings_total_count: usize,
    pub warnings_returned_count: usize,
    pub warnings_details_truncated: bool,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct PrivacyOutput {
    pub grades_included: bool,
    pub feedback_included: bool,
    pub user_email_included: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SummaryOutput {
    pub returned_count: usize,
    pub total_matching_count: usize,
    pub limited: bool,
    pub pending_count: usize,
    pub pending_returned_count: usize,
    pub pending_total_matching_count: usize,
    pub overdue_count: usize,
    pub due_within_48h_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SnapshotCourse {
    #[schemars(regex(pattern = "^course:[1-9][0-9]*$"))]
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AiSnapshotOutput {
    pub schema_version: &'static str,
    pub generated_at: String,
    pub privacy: PrivacyOutput,
    pub range: DateRange,
    pub summary: SummaryOutput,
    pub courses: Vec<SnapshotCourse>,
    pub courses_in_pending_tasks: Vec<SnapshotCourse>,
    pub pending_tasks: Vec<TodoItem>,
    pub unsupported_flags: Vec<String>,
    pub warnings_summary: Vec<WarningSummary>,
    pub warnings_total_count: usize,
    pub warnings_returned_count: usize,
    pub warnings_details_truncated: bool,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SchemaListOutput {
    pub schema_version: &'static str,
    pub generated_at: String,
    pub schemas: Vec<String>,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct DoctorOutput {
    pub schema_version: &'static str,
    pub generated_at: String,
    pub profile: String,
    pub active_profile: String,
    pub config_path: Option<String>,
    pub base_url: Option<String>,
    pub username: Option<String>,
    pub credential_target: Option<crate::keychain::CredentialTarget>,
    pub authenticated: bool,
    pub checks: Vec<DoctorCheck>,
    pub missing_functions: Vec<String>,
    pub unchecked_functions: Vec<String>,
    pub warnings: Vec<Warning>,
    pub next_steps: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_snapshot_golden_shape_is_stable() {
        let output = AiSnapshotOutput {
            schema_version: "campus-lms.ai_snapshot.v1",
            generated_at: "2026-05-08T00:00:00Z".to_string(),
            privacy: PrivacyOutput {
                grades_included: false,
                feedback_included: false,
                user_email_included: false,
            },
            range: DateRange {
                from: "2026-05-08".to_string(),
                to: "2026-05-22".to_string(),
                timezone: "UTC".to_string(),
            },
            summary: SummaryOutput {
                returned_count: 1,
                total_matching_count: 1,
                limited: false,
                pending_count: 1,
                pending_returned_count: 1,
                pending_total_matching_count: 1,
                overdue_count: 0,
                due_within_48h_count: 1,
            },
            courses: vec![SnapshotCourse {
                id: "course:42".to_string(),
                name: Some("Algorithms".to_string()),
            }],
            courses_in_pending_tasks: vec![SnapshotCourse {
                id: "course:42".to_string(),
                name: Some("Algorithms".to_string()),
            }],
            pending_tasks: vec![TodoItem {
                id: "assign:100".to_string(),
                item_type: "assignment".to_string(),
                course_id: Some("course:42".to_string()),
                course_name: Some("Algorithms".to_string()),
                title: Some("Report".to_string()),
                due_at: Some("2026-05-09T00:00:00Z".to_string()),
                due_in_seconds: Some(86_400),
                status: "pending".to_string(),
                status_reason: Some("submission status indicates this task is pending".to_string()),
                status_source: "submission_status".to_string(),
                priority_hint: "high".to_string(),
                url: None,
                detail_command: Some("campus-lms assignment show assign:100 --json".to_string()),
            }],
            unsupported_flags: Vec::new(),
            warnings_summary: Vec::new(),
            warnings_total_count: 0,
            warnings_returned_count: 0,
            warnings_details_truncated: false,
            warnings: Vec::new(),
        };

        assert_eq!(
            serde_json::to_value(output).unwrap(),
            serde_json::json!({
                "schema_version": "campus-lms.ai_snapshot.v1",
                "generated_at": "2026-05-08T00:00:00Z",
                "privacy": {
                    "grades_included": false,
                    "feedback_included": false,
                    "user_email_included": false
                },
                "range": {
                    "from": "2026-05-08",
                    "to": "2026-05-22",
                    "timezone": "UTC"
                },
                "summary": {
                    "returned_count": 1,
                    "total_matching_count": 1,
                    "limited": false,
                    "pending_count": 1,
                    "pending_returned_count": 1,
                    "pending_total_matching_count": 1,
                    "overdue_count": 0,
                    "due_within_48h_count": 1
                },
                "courses": [
                    {"id": "course:42", "name": "Algorithms"}
                ],
                "courses_in_pending_tasks": [
                    {"id": "course:42", "name": "Algorithms"}
                ],
                "pending_tasks": [
                    {
                        "id": "assign:100",
                        "type": "assignment",
                        "course_id": "course:42",
                        "course_name": "Algorithms",
                        "title": "Report",
                        "due_at": "2026-05-09T00:00:00Z",
                        "due_in_seconds": 86400,
                        "status": "pending",
                        "status_reason": "submission status indicates this task is pending",
                        "status_source": "submission_status",
                        "priority_hint": "high",
                        "url": null,
                        "detail_command": "campus-lms assignment show assign:100 --json"
                    }
                ],
                "unsupported_flags": [],
                "warnings_summary": [],
                "warnings_total_count": 0,
                "warnings_returned_count": 0,
                "warnings_details_truncated": false,
                "warnings": []
            })
        );
    }

    #[test]
    fn moodle_warning_code_is_schema_safe() {
        assert_eq!(
            normalize_warning_code(Some("course-not-visible")),
            "COURSE_NOT_VISIBLE"
        );
        assert_eq!(normalize_warning_code(Some("  ")), "MOODLE_WARNING");
    }

    #[test]
    fn warning_report_aggregates_repeated_warnings() {
        let warnings = vec![
            Warning::new("NO_ACCESS", "No access rights in module context", None),
            Warning::new("NO_ACCESS", "No access rights in module context", None),
            Warning::new("OTHER", "Different", Some("Check permissions".to_string())),
        ];
        let report = warning_report(warnings);
        assert_eq!(report.summary.len(), 2);
        assert!(report.summary.iter().any(|item| item.code == "NO_ACCESS"
            && item.count == 2
            && item.message == "No access rights in module context"));
        assert_eq!(report.details.len(), 3);
        assert_eq!(report.total_count, 3);
        assert_eq!(report.returned_count, 3);
        assert!(!report.details_truncated);
    }
}
