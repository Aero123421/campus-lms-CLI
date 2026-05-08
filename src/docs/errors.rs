// SPDX-License-Identifier: Apache-2.0

use crate::output;

pub fn print() -> crate::error::Result<()> {
    output::print_json(&serde_json::json!({
        "schema_version": "campus-lms.errors.v1",
        "errors": [
            {"code": "AUTH_REQUIRED", "exit_code": 10, "retryable": false, "hint": "Run: campus-lms auth verify --json, then campus-lms auth login or auth import-token."},
            {"code": "AUTH_EXPIRED", "exit_code": 14, "retryable": false, "hint": "Run: campus-lms auth login or auth import-token."},
            {"code": "PERMISSION_DENIED", "exit_code": 11, "retryable": false, "hint": "Run: campus-lms doctor --json and check LMS permissions."},
            {"code": "NETWORK_ERROR", "exit_code": 12, "retryable": true, "hint": "Check network, VPN, proxy, captive portal, or Moodle base URL."},
            {"code": "RATE_LIMITED", "exit_code": 13, "retryable": true, "hint": "Wait and retry later, or use --offline when cached data is acceptable."},
            {"code": "MOODLE_API_ERROR", "exit_code": 20, "retryable": false, "hint": "Run: campus-lms doctor --json to check enabled Web Service functions."},
            {"code": "UNSUPPORTED_MOODLE_FEATURE", "exit_code": 21, "retryable": false, "hint": "Ask your LMS administrator about Mobile Web Services and the token service."},
            {"code": "CONFIG_ERROR", "exit_code": 30, "retryable": false, "hint": "Run: campus-lms auth status --json or campus-lms init."},
            {"code": "KEYCHAIN_UNAVAILABLE", "exit_code": 31, "retryable": false, "hint": "Run: campus-lms auth verify --json and check OS credential store availability."},
            {"code": "CACHE_ERROR", "exit_code": 32, "retryable": false, "hint": "Run: campus-lms cleanup --cache --dry-run first, then cleanup --cache --yes."},
            {"code": "PARSE_ERROR", "exit_code": 40, "retryable": false, "hint": "The LMS response may not match the expected schema; run campus-lms doctor --json."}
        ]
    }))
}
