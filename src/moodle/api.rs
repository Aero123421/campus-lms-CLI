// SPDX-License-Identifier: Apache-2.0

use crate::moodle::models::{
    ActionEventsResponse, AssignmentsResponse, Course, SiteInfo, SubmissionStatusResponse,
};

pub trait MoodleApi {
    fn site_info(&self) -> crate::error::Result<SiteInfo>;
    fn user_courses(&self, user_id: i64) -> crate::error::Result<Vec<Course>>;
    fn assignments(&self, course_ids: &[i64]) -> crate::error::Result<AssignmentsResponse>;
    fn submission_status(&self, assign_id: i64) -> crate::error::Result<SubmissionStatusResponse>;
    fn action_events_by_timesort(
        &self,
        from: i64,
        to: i64,
        after_event_id: i64,
        limit_num: i64,
    ) -> crate::error::Result<ActionEventsResponse>;
}
