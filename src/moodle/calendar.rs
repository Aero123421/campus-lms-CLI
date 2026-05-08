// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use serde::{Deserialize, Serialize};
use time::{Date, Duration as TimeDuration, OffsetDateTime};

use crate::{
    cache,
    cli::{ensure_cache_flags, Cli, TodoArgs},
    error::CampusError,
    moodle::{
        assignments::{fetch_assignments, ts},
        client_from_profile,
        models::ActionEventsResponse,
    },
    output,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TodoItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub course_id: Option<String>,
    pub course_name: Option<String>,
    pub title: Option<String>,
    pub due_at: Option<String>,
    pub due_in_seconds: Option<i64>,
    pub status: String,
    pub priority_hint: String,
    pub url: Option<String>,
    pub detail_command: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TodoPayload {
    pub range_from: String,
    pub range_to: String,
    pub items: Vec<TodoItem>,
}

pub fn todo(cli: &Cli, args: &TodoArgs) -> crate::error::Result<()> {
    ensure_cache_flags(args.refresh, args.offline).map_err(|err| err.with_json(cli.json))?;
    let payload = fetch(cli, args).map_err(|err| err.with_json(cli.json))?;
    output::print_json(&serde_json::json!({
        "schema_version": "campus-lms.todo.v1",
        "generated_at": output::generated_at(),
        "range": {
            "from": payload.range_from,
            "to": payload.range_to,
            "timezone": "UTC"
        },
        "cache": {
            "used": args.offline,
            "fetched_at": output::generated_at(),
            "ttl_seconds": 300
        },
        "items": payload.items,
        "warnings": []
    }))
}

pub fn fetch(cli: &Cli, args: &TodoArgs) -> crate::error::Result<TodoPayload> {
    ensure_cache_flags(args.refresh, args.offline)?;
    let cache_key = cache::key(
        "todo",
        &format!(
            "{}:{}:{:?}:{}",
            cli.profile, args.days, args.course, args.include_submitted
        ),
    );
    if let Some(payload) = cache::get(
        &cache_key,
        Duration::from_secs(300),
        args.refresh,
        args.offline,
    )? {
        return Ok(payload);
    }
    if args.offline {
        return Err(CampusError::cache("offline cache miss for todo"));
    }

    let now = OffsetDateTime::now_utc();
    let to = now + TimeDuration::days(args.days as i64);
    let mut items = Vec::new();

    if let Ok(events) = fetch_action_events(cli, now.unix_timestamp(), to.unix_timestamp()) {
        items.extend(events.into_iter().filter_map(|event| {
            let due = event.timesort.or(event.timestart);
            let course_id = event
                .course
                .as_ref()
                .and_then(|course| course.id)
                .map(|id| format!("course:{id}"));
            if let Some(filter) = &args.course {
                if course_id.as_deref() != Some(filter.as_str()) {
                    return None;
                }
            }
            let item_type = event
                .modulename
                .clone()
                .unwrap_or_else(|| "calendar".to_string());
            let detail_command = match (event.modulename.as_deref(), event.instance) {
                (Some("assign"), Some(id)) => {
                    Some(format!("campus-lms assignment show assign:{id} --json"))
                }
                _ => None,
            };
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
                status: if event.action.and_then(|a| a.actionable).unwrap_or(true) {
                    "pending".to_string()
                } else {
                    "done_or_not_actionable".to_string()
                },
                priority_hint: priority(due, now.unix_timestamp()),
                url: event.url,
                detail_command,
            })
        }));
    }

    let existing_assignment_ids = items
        .iter()
        .filter_map(|item| item.detail_command.as_deref())
        .filter_map(|command| command.split("assign:").nth(1))
        .filter_map(|tail| tail.split_whitespace().next())
        .map(|id| format!("assign:{id}"))
        .collect::<std::collections::BTreeSet<_>>();

    let assignments = fetch_assignments(cli, args.refresh, args.offline)?;
    items.extend(assignments.into_iter().filter_map(|item| {
        let assignment_id = format!("assign:{}", item.assignment.id);
        if existing_assignment_ids.contains(&assignment_id) {
            return None;
        }
        if let Some(filter) = &args.course {
            if &item.course_id != filter {
                return None;
            }
        }
        let due = item.assignment.duedate;
        if due.is_some_and(|due| due > to.unix_timestamp()) {
            return None;
        }
        Some(TodoItem {
            id: assignment_id,
            item_type: "assignment".to_string(),
            course_id: Some(item.course_id),
            course_name: item.course_name,
            title: item.assignment.name,
            due_at: ts(due),
            due_in_seconds: due.map(|due| due - now.unix_timestamp()),
            status: "unknown".to_string(),
            priority_hint: priority(due, now.unix_timestamp()),
            url: None,
            detail_command: Some(format!(
                "campus-lms assignment show assign:{} --json",
                item.assignment.id
            )),
        })
    }));

    items.sort_by_key(|item| item.due_in_seconds.unwrap_or(i64::MAX));
    if let Some(max) = args.max_items {
        items.truncate(max);
    }

    let payload = TodoPayload {
        range_from: date(now.date()),
        range_to: date(to.date()),
        items,
    };
    cache::set(&cache_key, &payload)?;
    Ok(payload)
}

fn fetch_action_events(
    cli: &Cli,
    from: i64,
    to: i64,
) -> crate::error::Result<Vec<crate::moodle::models::ActionEvent>> {
    let client = client_from_profile(cli)?;
    let response: ActionEventsResponse = client.call(
        "core_calendar_get_action_events_by_timesort",
        serde_json::json!({
            "timesortfrom": from,
            "timesortto": to,
        }),
    )?;
    Ok(response.events)
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

fn date(date: Date) -> String {
    date.to_string()
}
