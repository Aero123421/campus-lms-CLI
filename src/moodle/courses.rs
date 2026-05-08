// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    cache,
    cli::{ensure_cache_flags, CachedArgs, Cli},
    error::CampusError,
    moodle::client_from_profile,
    output,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CourseItem {
    pub id: String,
    pub moodle_id: i64,
    pub short_name: Option<String>,
    pub full_name: Option<String>,
    pub visible: bool,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoursesPayload {
    pub courses: Vec<CourseItem>,
}

pub fn run(cli: &Cli, args: &CachedArgs) -> crate::error::Result<()> {
    ensure_cache_flags(args.refresh, args.offline).map_err(|err| err.with_json(cli.json))?;
    let payload = fetch(cli, args.refresh, args.offline).map_err(|err| err.with_json(cli.json))?;
    output::print_json(&serde_json::json!({
        "schema_version": "campus-lms.courses.v1",
        "generated_at": output::generated_at(),
        "cache": {
            "used": args.offline,
            "fetched_at": output::generated_at(),
            "ttl_seconds": 3600
        },
        "courses": payload.courses,
        "warnings": []
    }))
}

pub fn fetch(cli: &Cli, refresh: bool, offline: bool) -> crate::error::Result<CoursesPayload> {
    ensure_cache_flags(refresh, offline)?;
    let cache_key = cache::key("courses", &cli.profile);
    if let Some(payload) = cache::get(&cache_key, Duration::from_secs(3600), refresh, offline)? {
        return Ok(payload);
    }
    if offline {
        return Err(CampusError::cache("offline cache miss for courses"));
    }
    let client = client_from_profile(cli)?;
    let site = client.site_info()?;
    let courses = client
        .user_courses(site.userid)?
        .into_iter()
        .map(|course| {
            let url = client
                .base_url
                .join(&format!("course/view.php?id={}", course.id))
                .map(|url| url.to_string())
                .unwrap_or_default();
            CourseItem {
                id: format!("course:{}", course.id),
                moodle_id: course.id,
                short_name: course.shortname,
                full_name: course.fullname,
                visible: course.visible.unwrap_or(1) != 0,
                url,
            }
        })
        .collect();
    let payload = CoursesPayload { courses };
    cache::set(&cache_key, &payload)?;
    Ok(payload)
}
