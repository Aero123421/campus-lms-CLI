// SPDX-License-Identifier: Apache-2.0

use crate::output;

pub fn print() -> crate::error::Result<()> {
    output::print_json(&serde_json::json!({
        "schema_version": "campus-lms.capabilities.v1",
        "recommended_entrypoint": "campus-lms ai snapshot --days 14 --json",
        "commands": [
            {
                "name": "auth login",
                "read_only": false,
                "safe_for_ai": false,
                "description": "Request a Moodle Web Services token and verify OS credential-store persistence.",
                "example": "campus-lms auth login --base-url https://lms.example.edu/ --username student123 --password-stdin --json"
            },
            {
                "name": "auth import-token",
                "read_only": false,
                "safe_for_ai": false,
                "description": "Store an administrator-issued Moodle Web Services token for SSO/MFA environments.",
                "example": "campus-lms auth import-token --base-url https://lms.example.edu/ --username student123 --token-stdin --live --json"
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
                "description": "Return a compact overview of upcoming Moodle tasks. Fallback submission-status checks are capped to reduce LMS load.",
                "example": "campus-lms ai snapshot --days 14 --json",
                "ai_safety_notes": [
                    "Includes cache metadata so agents can tell whether the snapshot came from cache.",
                    "Reserved grade and feedback flags are reported as unsupported in this version."
                ]
            },
            {
                "name": "assignment show",
                "read_only": true,
                "safe_for_ai": true,
                "description": "Show assignment details without submitting or changing completion state. Use --no-cache for sensitive one-off detail reads.",
                "example": "campus-lms assignment show assign:12345 --no-cache --json",
                "ai_safety_notes": [
                    "--include-html may expose raw LMS HTML and should be avoided unless explicitly needed.",
                    "Use --max-chars for large descriptions and prefer the detail_command returned by todo or ai snapshot."
                ]
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
                "description": "List upcoming LMS tasks. Use --status-check-limit or --no-submission-status-check to control fallback API load.",
                "example": "campus-lms todo --days 14 --status-check-limit 20 --json",
                "ai_safety_notes": [
                    "Undated fallback assignments are excluded by default; use --include-undated only when needed.",
                    "Use --warning-details for debugging; normal AI flows should rely on warnings_summary."
                ]
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
