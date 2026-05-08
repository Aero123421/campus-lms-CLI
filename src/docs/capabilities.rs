// SPDX-License-Identifier: Apache-2.0

use crate::output;

pub fn print() -> crate::error::Result<()> {
    output::print_json(&serde_json::json!({
        "schema_version": "campus-lms.capabilities.v1",
        "recommended_entrypoint": "campus-lms ai snapshot --days 14 --json",
        "commands": [
            {
                "name": "auth login",
                "read_only": true,
                "safe_for_ai": false,
                "description": "Request a Moodle Web Services token and verify OS credential-store persistence.",
                "example": "campus-lms auth login --base-url https://lms.example.edu/ --username student123 --password-stdin --json"
            },
            {
                "name": "auth import-token",
                "read_only": true,
                "safe_for_ai": false,
                "description": "Store an administrator-issued Moodle Web Services token for SSO/MFA environments.",
                "example": "campus-lms auth import-token --base-url https://lms.example.edu/ --username student123 --token-stdin --json"
            },
            {
                "name": "auth status",
                "read_only": true,
                "safe_for_ai": true,
                "description": "Show selected profile, credential target, token readability, and optional live API status.",
                "example": "campus-lms auth status --live --json"
            },
            {
                "name": "auth verify",
                "read_only": true,
                "safe_for_ai": true,
                "description": "Verify profile configuration, OS credential-store roundtrip, token readability, and optional live API access.",
                "example": "campus-lms auth verify --live --json"
            },
            {
                "name": "ai snapshot",
                "read_only": true,
                "safe_for_ai": true,
                "description": "Return a compact overview of upcoming Moodle tasks.",
                "example": "campus-lms ai snapshot --days 14 --json"
            },
            {
                "name": "assignment show",
                "read_only": true,
                "safe_for_ai": true,
                "description": "Show assignment details without submitting or changing completion state.",
                "example": "campus-lms assignment show assign:12345 --json"
            },
            {
                "name": "courses",
                "read_only": true,
                "safe_for_ai": true,
                "description": "List visible courses.",
                "example": "campus-lms courses --json"
            },
            {
                "name": "todo",
                "read_only": true,
                "safe_for_ai": true,
                "description": "List upcoming LMS tasks.",
                "example": "campus-lms todo --days 14 --json"
            },
            {
                "name": "doctor",
                "read_only": true,
                "safe_for_ai": true,
                "description": "Diagnose profile, token, and Moodle Web Services function availability.",
                "example": "campus-lms doctor --json"
            },
            {
                "name": "init",
                "read_only": true,
                "safe_for_ai": true,
                "description": "Create local config/cache directories and a non-secret default config.",
                "example": "campus-lms init --json"
            },
            {
                "name": "cleanup",
                "read_only": false,
                "safe_for_ai": false,
                "description": "Remove selected local campus-lms data after explicit confirmation.",
                "example": "campus-lms cleanup --cache --dry-run --json"
            },
            {
                "name": "uninstall",
                "read_only": false,
                "safe_for_ai": false,
                "description": "Clean local campus-lms user data and print the npm uninstall command.",
                "example": "campus-lms uninstall --dry-run --json"
            }
        ],
        "dangerous_commands": ["cleanup", "uninstall"]
    }))
}
