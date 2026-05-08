// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MoodleException {
    pub exception: Option<String>,
    pub errorcode: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SiteInfo {
    pub userid: i64,
    pub username: String,
    #[serde(default)]
    pub fullname: Option<String>,
    #[serde(default)]
    pub sitename: Option<String>,
    #[serde(default)]
    pub siteurl: Option<String>,
    #[serde(default)]
    pub downloadfiles: Option<i64>,
    #[serde(default)]
    pub uploadfiles: Option<i64>,
    #[serde(default)]
    pub functions: Vec<WebServiceFunction>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebServiceFunction {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Course {
    pub id: i64,
    #[serde(default)]
    pub shortname: Option<String>,
    #[serde(default)]
    pub fullname: Option<String>,
    #[serde(default)]
    pub visible: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssignmentsResponse {
    #[serde(default)]
    pub courses: Vec<AssignmentCourse>,
    #[serde(default)]
    pub warnings: Vec<MoodleWarning>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssignmentCourse {
    pub id: i64,
    #[serde(default)]
    pub fullname: Option<String>,
    #[serde(default)]
    pub shortname: Option<String>,
    #[serde(default)]
    pub assignments: Vec<Assignment>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Assignment {
    pub id: i64,
    #[serde(default)]
    pub cmid: Option<i64>,
    #[serde(default)]
    pub course: Option<i64>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub intro: Option<String>,
    #[serde(default)]
    pub duedate: Option<i64>,
    #[serde(default)]
    pub allowsubmissionsfromdate: Option<i64>,
    #[serde(default)]
    pub cutoffdate: Option<i64>,
    #[serde(default)]
    pub introattachments: Vec<MoodleFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MoodleFile {
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub filepath: Option<String>,
    #[serde(default)]
    pub filesize: Option<i64>,
    #[serde(default)]
    pub mimetype: Option<String>,
    #[serde(default)]
    pub fileurl: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MoodleWarning {
    #[serde(default)]
    pub item: Option<String>,
    #[serde(default)]
    pub itemid: Option<i64>,
    #[serde(default)]
    pub warningcode: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubmissionStatusResponse {
    #[serde(default)]
    pub lastattempt: Option<LastAttempt>,
    #[serde(default)]
    pub warnings: Vec<MoodleWarning>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LastAttempt {
    #[serde(default)]
    pub submission: Option<Submission>,
    #[serde(default)]
    pub gradingstatus: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Submission {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub timemodified: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActionEventsResponse {
    #[serde(default)]
    pub events: Vec<ActionEvent>,
    #[serde(default)]
    pub warnings: Vec<MoodleWarning>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActionEvent {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub timestart: Option<i64>,
    #[serde(default)]
    pub timesort: Option<i64>,
    #[serde(default)]
    pub course: Option<CourseSummary>,
    #[serde(default)]
    pub modulename: Option<String>,
    #[serde(default)]
    pub instance: Option<i64>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub action: Option<EventAction>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CourseSummary {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub fullname: Option<String>,
    #[serde(default)]
    pub shortname: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventAction {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub actionable: Option<bool>,
}
