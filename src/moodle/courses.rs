// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    cache,
    cli::{ensure_cache_flags, CachedArgs, Cli},
    config,
    dto::{CacheMeta, CourseItem, CoursesOutput},
    error::CampusError,
    moodle::{api::MoodleApi, client::client_from_profile_data},
    output,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoursesPayload {
    pub courses: Vec<CourseItem>,
}

#[derive(Debug, Clone)]
pub struct FetchedCourses {
    pub payload: CoursesPayload,
    pub cache: CacheMeta,
}

pub fn run(cli: &Cli, args: &CachedArgs) -> crate::error::Result<()> {
    ensure_cache_flags(args.refresh, args.offline).map_err(|err| err.with_json(cli.json))?;
    let fetched = fetch(cli, args.refresh, args.offline).map_err(|err| err.with_json(cli.json))?;
    output::print_json(&CoursesOutput {
        schema_version: "campus-lms.courses.v1",
        generated_at: output::generated_at(),
        cache: fetched.cache,
        courses: fetched.payload.courses,
        warnings: Vec::new(),
    })
}

pub fn fetch(cli: &Cli, refresh: bool, offline: bool) -> crate::error::Result<FetchedCourses> {
    ensure_cache_flags(refresh, offline)?;
    let config = config::load(cli)?;
    let profile_name = config::selected_profile_name(cli, &config);
    let profile = config::active_profile(cli, &config)?;
    cache::prune_older_than(Duration::from_secs(profile.cache_retention_seconds))?;
    let ttl_seconds = profile.cache_ttl_seconds;
    let ttl = Duration::from_secs(ttl_seconds);
    let namespace = cache::profile_namespace(&profile_name, profile, None);
    let cache_key = cache::key("courses", &namespace);
    if let Some(entry) = cache::get_entry(&cache_key, ttl, refresh, offline)? {
        return Ok(FetchedCourses {
            cache: CacheMeta::from_entry(&entry, ttl_seconds),
            payload: entry.value,
        });
    }
    if offline {
        return Err(CampusError::cache("offline cache miss for courses"));
    }
    let client = client_from_profile_data(cli, &profile_name, profile)?;
    let payload = fetch_from_api(&client, &client.base_url)?;
    cache::set(&cache_key, &payload)?;
    Ok(FetchedCourses {
        payload,
        cache: CacheMeta::fresh(ttl_seconds),
    })
}

fn fetch_from_api<T: MoodleApi>(
    client: &T,
    base_url: &url::Url,
) -> crate::error::Result<CoursesPayload> {
    let site = client.site_info()?;
    let courses = client
        .user_courses(site.userid)?
        .into_iter()
        .map(|course| {
            let url = base_url
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
    Ok(CoursesPayload { courses })
}
