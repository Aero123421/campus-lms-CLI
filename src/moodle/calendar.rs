// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeSet;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use time::{Date, Duration as TimeDuration, OffsetDateTime, UtcOffset};

use crate::{
    cache,
    cli::{
        ensure_cache_flags, ensure_days, ensure_max_items, ensure_status_check_limit,
        normalize_course_filter, warning_detail_limit, Cli, TodoArgs,
    },
    config,
    dto::{
        warning_report_with_options, CacheMeta, DateRange, TodoItem, TodoOutput, TodoSummary,
        Warning,
    },
    error::CampusError,
    moodle::{
        api::MoodleApi,
        assignments::{fetch_assignments_for_courses, ts, AssignmentIndexPayload},
        client::client_from_profile_data,
        models::{ActionEvent, ActionEventsResponse},
    },
    output,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TodoPayload {
    pub range_from: String,
    pub range_to: String,
    pub items: Vec<TodoItem>,
    pub total_items_before_limit: usize,
    #[serde(default)]
    pub visible_warning_item_ids: Vec<i64>,
    pub warnings: Vec<Warning>,
}

impl TodoPayload {
    fn visible_item_ids(&self) -> BTreeSet<i64> {
        self.visible_warning_item_ids.iter().copied().collect()
    }
}

#[derive(Debug, Clone)]
pub struct FetchedTodo {
    pub payload: TodoPayload,
    pub cache: CacheMeta,
}

pub fn todo(cli: &Cli, args: &TodoArgs) -> crate::error::Result<()> {
    ensure_cache_flags(args.refresh, args.offline).map_err(|err| err.with_json(cli.json))?;
    ensure_days(args.days).map_err(|err| err.with_json(cli.json))?;
    ensure_max_items(args.max_items).map_err(|err| err.with_json(cli.json))?;
    ensure_status_check_limit(args.status_check_limit).map_err(|err| err.with_json(cli.json))?;
    let fetched = fetch(cli, args).map_err(|err| err.with_json(cli.json))?;
    let detail_limit = warning_detail_limit(cli).map_err(|err| err.with_json(cli.json))?;
    let visible_item_ids = fetched.payload.visible_item_ids();
    let report =
        warning_report_with_options(fetched.payload.warnings, detail_limit, &visible_item_ids);
    let returned_count = fetched.payload.items.len();
    let total_matching_count = fetched.payload.total_items_before_limit;
    output::print_json(&TodoOutput {
        schema_version: "campus-lms.todo.v1",
        generated_at: output::generated_at(),
        range: DateRange {
            from: fetched.payload.range_from,
            to: fetched.payload.range_to,
            timezone: configured_timezone(cli),
        },
        cache: fetched.cache,
        summary: TodoSummary {
            returned_count,
            total_matching_count,
            limited: returned_count < total_matching_count,
            overdue_count: fetched
                .payload
                .items
                .iter()
                .filter(|item| item.priority_hint == "overdue")
                .count(),
            due_within_48h_count: fetched
                .payload
                .items
                .iter()
                .filter(|item| item.priority_hint == "high")
                .count(),
        },
        total_items_before_limit: fetched.payload.total_items_before_limit,
        items: fetched.payload.items,
        warnings_summary: report.summary,
        warnings_total_count: report.total_count,
        warnings_returned_count: report.returned_count,
        warnings_details_truncated: report.details_truncated,
        warnings: report.details,
    })
}

pub fn fetch(cli: &Cli, args: &TodoArgs) -> crate::error::Result<FetchedTodo> {
    ensure_cache_flags(args.refresh, args.offline)?;
    ensure_days(args.days)?;
    ensure_max_items(args.max_items)?;
    ensure_status_check_limit(args.status_check_limit)?;
    let config = config::load(cli)?;
    let profile_name = config::selected_profile_name(cli, &config);
    let profile = config::active_profile(cli, &config)?;
    let course_filter = args
        .course
        .as_deref()
        .map(normalize_course_filter)
        .transpose()?;
    let normalized_course = course_filter.as_ref().map(|(course, _)| course.as_str());
    let course_ids = course_filter
        .as_ref()
        .map(|(_, id)| vec![*id])
        .unwrap_or_default();
    let namespace = cache::profile_namespace(&profile_name, profile, None);
    cache::prune_namespace(
        &namespace,
        Duration::from_secs(profile.cache_retention_seconds),
    )?;
    let cache_key = cache::profile_key(
        "todo",
        &namespace,
        &format!(
            "v8:{}:{:?}:{}:{:?}:{}:{}:{}",
            args.days,
            normalized_course,
            args.include_submitted,
            args.max_items,
            args.status_check_limit,
            args.no_submission_status_check,
            args.include_undated
        ),
    );
    let ttl_seconds = profile.cache_ttl_seconds;
    let ttl = Duration::from_secs(ttl_seconds);
    if let Some(entry) = cache::get_entry(&cache_key, ttl, args.refresh, args.offline)? {
        return Ok(FetchedTodo {
            cache: CacheMeta::from_entry(&entry, ttl_seconds),
            payload: entry.value,
        });
    }
    if args.offline {
        return Err(CampusError::cache("offline cache miss for todo"));
    }

    let client = client_from_profile_data(cli, &profile_name, profile)?;
    let now = OffsetDateTime::now_utc();
    let to = now + TimeDuration::days(args.days as i64);
    let timezone = config.output.timezone.clone();
    let mut items = Vec::new();
    let mut warnings = Vec::new();
    let command_prefix = command_prefix(cli);
    let assignments = fetch_assignments_for_courses(cli, args.refresh, args.offline, &course_ids)?;
    let visible_warning_item_ids = assignments
        .items
        .iter()
        .filter(|item| normalized_course.is_none_or(|course| item.course_id == course))
        .flat_map(|item| [Some(item.assignment.id), item.assignment.cmid])
        .flatten()
        .collect::<Vec<_>>();
    warnings.extend(filter_warnings_for_course(
        assignments.warnings.clone(),
        normalized_course,
    ));

    match fetch_action_events(&client, now.unix_timestamp(), to.unix_timestamp()) {
        Ok((events, event_warnings)) => {
            warnings.extend(filter_warnings_for_course(event_warnings, normalized_course));
            items.extend(events.into_iter().filter_map(|event| {
                let due = valid_due(event.timesort.or(event.timestart));
                let course_id = event
                    .course
                    .as_ref()
                    .and_then(|course| course.id)
                    .map(|id| format!("course:{id}"));
                if let Some(filter) = normalized_course {
                    if course_id.as_deref() != Some(filter) {
                        return None;
                    }
                }
                let item_type = event
                    .modulename
                    .clone()
                    .unwrap_or_else(|| "calendar".to_string());
                let detail_command =
                    assignment_id_for_event(&event, due, &assignments).map(|id| {
                        format!("{command_prefix} assignment show assign:{id} --json")
                    });
                let actionable = event.action.as_ref().and_then(|a| a.actionable).unwrap_or(true);
                if !args.include_submitted && !actionable {
                    return None;
                }
                Some(TodoItem {
                    id: format!("calendar:{}", event.id),
                    item_type,
                    course_id,
                    course_name: event
                        .course
                        .and_then(|course| course.fullname.or(course.shortname)),
                    title: event.name,
                    due_at: ts(due),
                    due_in_seconds: due.map(|due| due - now.unix_timestamp()),
                    status: if actionable {
                        "pending".to_string()
                    } else {
                        "completed_or_not_actionable".to_string()
                    },
                    status_reason: Some(if actionable {
                        "Moodle calendar action is marked actionable.".to_string()
                    } else {
                        "Moodle calendar action is marked not actionable.".to_string()
                    }),
                    status_source: "calendar_action".to_string(),
                    priority_hint: priority(due, now.unix_timestamp()),
                    url: event.url,
                    detail_command,
                })
            }));
        }
        Err(err) => warnings.push(Warning::new(
            "CALENDAR_EVENTS_UNAVAILABLE",
            err.to_string(),
            Some(
                "Fallback assignment list was used; quizzes or calendar-only events may be missing."
                    .to_string(),
            ),
        )),
    }

    let existing_assignment_ids = items
        .iter()
        .filter_map(|item| item.detail_command.as_deref())
        .filter_map(|command| command.split("assign:").nth(1))
        .filter_map(|tail| tail.split_whitespace().next())
        .map(|id| format!("assign:{id}"))
        .collect::<std::collections::BTreeSet<_>>();

    let mut status_checks = 0usize;
    let mut status_limit_warning_emitted = false;
    for item in assignments.items {
        let assignment_id = format!("assign:{}", item.assignment.id);
        if existing_assignment_ids.contains(&assignment_id) {
            continue;
        }
        if let Some(filter) = normalized_course {
            if item.course_id != filter {
                continue;
            }
        }
        let due = valid_due(item.assignment.duedate);
        if due.is_none() && !args.include_undated {
            continue;
        }
        if due.is_some_and(|due| due > to.unix_timestamp()) {
            continue;
        }
        let due_at = ts(due);
        if items.iter().any(|existing| {
            existing.course_id.as_deref() == Some(item.course_id.as_str())
                && existing.title.as_deref() == item.assignment.name.as_deref()
                && existing.due_at == due_at
        }) {
            continue;
        }
        let mut status = "unknown".to_string();
        let mut status_reason = "submission status was not checked.".to_string();
        let mut status_source = "assignment_fallback".to_string();
        if args.no_submission_status_check {
            status_reason =
                "--no-submission-status-check was used, so submitted status was not checked."
                    .to_string();
        } else if !args.include_submitted {
            if status_checks >= args.status_check_limit {
                if !status_limit_warning_emitted {
                    warnings.push(Warning::new(
                        "SUBMISSION_STATUS_CHECK_LIMIT_REACHED",
                        format!(
                            "Skipped fallback submission-status checks after {} assignment(s).",
                            args.status_check_limit
                        ),
                        Some("Increase --status-check-limit, use --include-submitted, or rely on calendar action status to reduce LMS load.".to_string()),
                    ));
                    status_limit_warning_emitted = true;
                }
                status_reason =
                    "submission status was not checked because --status-check-limit was reached."
                        .to_string();
            } else {
                status_checks += 1;
                match assignment_submission_status(&client, item.assignment.id) {
                Ok(Some(submission_status)) => {
                    status_source = "submission_status".to_string();
                    status_reason = format!(
                        "Moodle submission status API returned status '{submission_status}'."
                    );
                    status = submission_status;
                    if status == "submitted" {
                        continue;
                    }
                }
                Ok(None) => {
                    status_reason =
                        "Moodle submission status API did not include a submission status."
                            .to_string();
                }
                Err(err) => warnings.push(Warning::new(
                    "SUBMISSION_STATUS_UNAVAILABLE",
                    err.to_string(),
                    Some(
                        "A fallback assignment item was kept because submitted status could not be confirmed."
                            .to_string(),
                    ),
                )),
            }
            }
        } else {
            status_reason =
                "--include-submitted was used, so submitted status was not filtered.".to_string();
        }
        items.push(TodoItem {
            id: assignment_id,
            item_type: "assignment".to_string(),
            course_id: Some(item.course_id),
            course_name: item.course_name,
            title: item.assignment.name,
            due_at,
            due_in_seconds: due.map(|due| due - now.unix_timestamp()),
            status,
            status_reason: Some(status_reason),
            status_source,
            priority_hint: priority(due, now.unix_timestamp()),
            url: None,
            detail_command: Some(format!(
                "{command_prefix} assignment show assign:{} --json",
                item.assignment.id,
            )),
        });
    }

    items.sort_by_key(|item| item.due_in_seconds.unwrap_or(i64::MAX));
    let total_items_before_limit = items.len();
    if let Some(max) = args.max_items {
        items.truncate(max);
    }
    let (range_from, range_to) = date_range(now, to, &timezone);

    let payload = TodoPayload {
        range_from,
        range_to,
        items,
        total_items_before_limit,
        visible_warning_item_ids,
        warnings,
    };
    cache::set(&cache_key, &payload)?;
    Ok(FetchedTodo {
        payload,
        cache: CacheMeta::fresh(ttl_seconds),
    })
}

fn fetch_action_events<T: MoodleApi>(
    client: &T,
    from: i64,
    to: i64,
) -> crate::error::Result<(Vec<ActionEvent>, Vec<Warning>)> {
    let mut all = Vec::new();
    let mut warnings = Vec::new();
    let mut after_event_id = 0;
    let limit_num = 50;
    let mut pages = 0;
    loop {
        if pages >= 100 {
            warnings.push(Warning::new(
                "CALENDAR_PAGINATION_LIMIT_REACHED",
                "Stopped fetching calendar events after 100 pages.",
                Some(
                    "The LMS may be returning repeated pages; results may be partial.".to_string(),
                ),
            ));
            break;
        }
        pages += 1;
        let response: ActionEventsResponse =
            client.action_events_by_timesort(from, to, after_event_id, limit_num)?;
        let count = response.events.len();
        if let Some(last) = response.events.last() {
            after_event_id = last.id;
        }
        warnings.extend(response.warnings.iter().map(Warning::from_moodle_warning));
        all.extend(response.events);
        if count < limit_num as usize || count == 0 {
            break;
        }
    }
    Ok((all, warnings))
}

pub fn priority(due: Option<i64>, now: i64) -> String {
    match due.map(|due| due - now) {
        Some(seconds) if seconds < 0 => "overdue".to_string(),
        Some(seconds) if seconds <= 48 * 3600 => "high".to_string(),
        Some(seconds) if seconds <= 7 * 24 * 3600 => "medium".to_string(),
        Some(_) => "low".to_string(),
        None => "unknown".to_string(),
    }
}

fn valid_due(timestamp: Option<i64>) -> Option<i64> {
    timestamp.filter(|timestamp| *timestamp > 0)
}

fn assignment_id_for_event(
    event: &ActionEvent,
    due: Option<i64>,
    assignments: &AssignmentIndexPayload,
) -> Option<i64> {
    if event.modulename.as_deref() != Some("assign") {
        return None;
    }
    if let Some(instance) = event.instance {
        if let Some(item) = assignments
            .items
            .iter()
            .find(|item| item.assignment.id == instance)
        {
            return Some(item.assignment.id);
        }
        if let Some(item) = assignments
            .items
            .iter()
            .find(|item| item.assignment.cmid == Some(instance))
        {
            return Some(item.assignment.id);
        }
    }
    let course_id = event
        .course
        .as_ref()
        .and_then(|course| course.id)
        .map(|id| format!("course:{id}"));
    assignments
        .items
        .iter()
        .find(|item| {
            course_id.as_deref() == Some(item.course_id.as_str())
                && event.name.as_deref() == item.assignment.name.as_deref()
                && valid_due(item.assignment.duedate) == due
        })
        .map(|item| item.assignment.id)
}

fn configured_timezone(cli: &Cli) -> String {
    config::load(cli)
        .ok()
        .map(|config| config.output.timezone)
        .unwrap_or_else(|| "UTC".to_string())
}

fn date_range(from: OffsetDateTime, to: OffsetDateTime, timezone: &str) -> (String, String) {
    let offset = match timezone {
        "Asia/Tokyo" => UtcOffset::from_hms(9, 0, 0).ok(),
        "UTC" => UtcOffset::from_hms(0, 0, 0).ok(),
        _ => None,
    };
    match offset {
        Some(offset) => (
            date(from.to_offset(offset).date()),
            date(to.to_offset(offset).date()),
        ),
        None => (date(from.date()), date(to.date())),
    }
}

fn filter_warnings_for_course(warnings: Vec<Warning>, course: Option<&str>) -> Vec<Warning> {
    let Some(course) = course else {
        return warnings;
    };
    let Some(course_id) = course
        .strip_prefix("course:")
        .and_then(|id| id.parse::<i64>().ok())
    else {
        return warnings;
    };
    warnings
        .into_iter()
        .filter(|warning| match (warning.item.as_deref(), warning.itemid) {
            (Some("course"), Some(itemid)) => itemid == course_id,
            (_, Some(_)) => false,
            _ => false,
        })
        .collect()
}

fn date(date: Date) -> String {
    date.to_string()
}

fn assignment_submission_status<T: MoodleApi>(
    client: &T,
    assign_id: i64,
) -> crate::error::Result<Option<String>> {
    let status = client.submission_status(assign_id)?;
    Ok(status
        .lastattempt
        .and_then(|attempt| attempt.submission)
        .and_then(|submission| submission.status))
}

fn command_prefix(cli: &Cli) -> String {
    match cli.profile.as_deref() {
        Some(profile) => format!("campus-lms --profile {profile}"),
        None => "campus-lms".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use crate::moodle::{
        assignments::AssignmentIndexItem,
        models::{
            ActionEvent, ActionEventsResponse, Assignment, AssignmentsResponse, Course, SiteInfo,
            SubmissionStatusResponse,
        },
    };

    use super::*;

    struct FakeApi {
        calls: RefCell<Vec<i64>>,
    }

    impl MoodleApi for FakeApi {
        fn site_info(&self) -> crate::error::Result<SiteInfo> {
            unimplemented!()
        }

        fn user_courses(&self, _user_id: i64) -> crate::error::Result<Vec<Course>> {
            unimplemented!()
        }

        fn assignments(&self, _course_ids: &[i64]) -> crate::error::Result<AssignmentsResponse> {
            unimplemented!()
        }

        fn submission_status(
            &self,
            _assign_id: i64,
        ) -> crate::error::Result<SubmissionStatusResponse> {
            unimplemented!()
        }

        fn action_events_by_timesort(
            &self,
            _from: i64,
            _to: i64,
            after_event_id: i64,
            _limit_num: i64,
        ) -> crate::error::Result<ActionEventsResponse> {
            self.calls.borrow_mut().push(after_event_id);
            let start = if after_event_id == 0 { 1 } else { 51 };
            let count = if after_event_id == 0 { 50 } else { 10 };
            Ok(ActionEventsResponse {
                events: (start..start + count)
                    .map(|id| ActionEvent {
                        id,
                        name: Some(format!("event {id}")),
                        description: None,
                        timestart: None,
                        timesort: Some(id),
                        course: None,
                        modulename: None,
                        instance: None,
                        url: None,
                        action: None,
                    })
                    .collect(),
                warnings: Vec::new(),
            })
        }
    }

    #[test]
    fn action_events_are_paginated_until_short_page() {
        let api = FakeApi {
            calls: RefCell::new(Vec::new()),
        };
        let (events, warnings) = fetch_action_events(&api, 0, 100).unwrap();
        assert_eq!(events.len(), 60);
        assert!(warnings.is_empty());
        assert_eq!(api.calls.into_inner(), vec![0, 50]);
    }

    #[test]
    fn zero_due_date_is_not_overdue() {
        assert_eq!(valid_due(Some(0)), None);
        assert_eq!(priority(valid_due(Some(0)), 100), "unknown");
    }

    #[test]
    fn assignment_event_detail_uses_assignment_id_not_cmid() {
        let assignments = AssignmentIndexPayload {
            items: vec![AssignmentIndexItem {
                course_id: "course:10".to_string(),
                course_name: Some("Course".to_string()),
                assignment: Assignment {
                    id: 123,
                    cmid: Some(999),
                    course: Some(10),
                    name: Some("Report".to_string()),
                    intro: None,
                    duedate: Some(1_800),
                    allowsubmissionsfromdate: None,
                    cutoffdate: None,
                    introattachments: Vec::new(),
                },
            }],
            warnings: Vec::new(),
        };
        let event = ActionEvent {
            id: 1,
            name: Some("Report".to_string()),
            description: None,
            timestart: None,
            timesort: Some(1_800),
            course: Some(crate::moodle::models::CourseSummary {
                id: Some(10),
                fullname: Some("Course".to_string()),
                shortname: None,
            }),
            modulename: Some("assign".to_string()),
            instance: Some(999),
            url: None,
            action: None,
        };
        assert_eq!(
            assignment_id_for_event(&event, Some(1_800), &assignments),
            Some(123)
        );
    }
}
