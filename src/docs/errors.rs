// SPDX-License-Identifier: Apache-2.0

use crate::output;

pub fn print() -> crate::error::Result<()> {
    output::print_json(&serde_json::json!({
        "schema_version": "campus-lms.errors.v1",
        "errors": [
            {"code": "AUTH_REQUIRED", "exit_code": 10, "retryable": false, "hint": "Run: campus-lms auth login"},
            {"code": "AUTH_EXPIRED", "exit_code": 14, "retryable": false, "hint": "Run: campus-lms auth login"},
            {"code": "PERMISSION_DENIED", "exit_code": 11, "retryable": false, "hint": "Check LMS permissions."},
            {"code": "NETWORK_ERROR", "exit_code": 12, "retryable": true, "hint": "Check your network connection or Moodle base URL."},
            {"code": "RATE_LIMITED", "exit_code": 13, "retryable": true, "hint": "Wait and retry later."},
            {"code": "MOODLE_API_ERROR", "exit_code": 20, "retryable": false, "hint": "Check whether the Moodle Web Service function is enabled."},
            {"code": "UNSUPPORTED_MOODLE_FEATURE", "exit_code": 21, "retryable": false, "hint": "Ask your LMS administrator about Mobile Web Services."},
            {"code": "CONFIG_ERROR", "exit_code": 30, "retryable": false, "hint": "Check config.toml."},
            {"code": "KEYCHAIN_UNAVAILABLE", "exit_code": 31, "retryable": false, "hint": "Check OS credential store availability."},
            {"code": "CACHE_ERROR", "exit_code": 32, "retryable": false, "hint": "Clear the campus-lms cache directory."},
            {"code": "PARSE_ERROR", "exit_code": 40, "retryable": false, "hint": "The LMS response may not match the expected schema."}
        ]
    }))
}
