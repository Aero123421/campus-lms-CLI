// SPDX-License-Identifier: Apache-2.0

use crate::{cli::SchemaCommand, error::CampusError, output};

pub fn run(command: &SchemaCommand) -> crate::error::Result<()> {
    match command {
        SchemaCommand::List => output::print_json(&serde_json::json!({
            "schema_version": "campus-lms.schema_list.v1",
            "generated_at": output::generated_at(),
            "schemas": [
                "auth_status.v1",
                "whoami.v1",
                "courses.v1",
                "todo.v1",
                "assignment.v1",
                "ai_snapshot.v1",
                "capabilities.v1",
                "errors.v1",
                "error.v1",
                "init.v1",
                "cleanup.v1",
                "uninstall.v1"
            ],
            "warnings": []
        })),
        SchemaCommand::Show { name } => show(name),
    }
}

fn show(name: &str) -> crate::error::Result<()> {
    let schema = match name {
        "auth_status.v1" | "campus-lms.auth_status.v1" => serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "campus-lms.auth_status.v1",
            "type": "object",
            "required": ["schema_version", "generated_at", "authenticated", "profile", "token_available", "warnings"],
            "properties": {
                "schema_version": {"const": "campus-lms.auth_status.v1"},
                "generated_at": {"type": "string", "format": "date-time"},
                "authenticated": {"type": "boolean"},
                "profile": {"type": "string"},
                "base_url": {"type": ["string", "null"]},
                "username": {"type": ["string", "null"]},
                "token_available": {"type": "boolean"},
                "warnings": {"type": "array"}
            }
        }),
        "whoami.v1" | "campus-lms.whoami.v1" => serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "campus-lms.whoami.v1",
            "type": "object",
            "required": ["schema_version", "generated_at", "user", "warnings"],
            "properties": {
                "schema_version": {"const": "campus-lms.whoami.v1"},
                "generated_at": {"type": "string", "format": "date-time"},
                "user": {"type": "object"},
                "warnings": {"type": "array"}
            }
        }),
        "courses.v1" | "campus-lms.courses.v1" => serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "campus-lms.courses.v1",
            "type": "object",
            "required": ["schema_version", "generated_at", "courses", "warnings"],
            "properties": {
                "schema_version": {"const": "campus-lms.courses.v1"},
                "generated_at": {"type": "string", "format": "date-time"},
                "cache": {"type": "object"},
                "courses": {"type": "array"},
                "warnings": {"type": "array"}
            }
        }),
        "ai_snapshot.v1" | "campus-lms.ai_snapshot.v1" => serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "campus-lms.ai_snapshot.v1",
            "type": "object",
            "required": ["schema_version", "generated_at", "privacy", "range", "summary", "pending_tasks", "warnings"],
            "properties": {
                "schema_version": {"const": "campus-lms.ai_snapshot.v1"},
                "generated_at": {"type": "string", "format": "date-time"},
                "privacy": {"type": "object"},
                "range": {"type": "object"},
                "summary": {"type": "object"},
                "courses": {"type": "array"},
                "pending_tasks": {"type": "array"},
                "warnings": {"type": "array"}
            }
        }),
        "todo.v1" | "campus-lms.todo.v1" => serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "campus-lms.todo.v1",
            "type": "object",
            "required": ["schema_version", "generated_at", "range", "items", "warnings"],
            "properties": {
                "schema_version": {"const": "campus-lms.todo.v1"},
                "generated_at": {"type": "string", "format": "date-time"},
                "range": {"type": "object"},
                "items": {"type": "array"},
                "warnings": {"type": "array"}
            }
        }),
        "assignment.v1" | "campus-lms.assignment.v1" => serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "campus-lms.assignment.v1",
            "type": "object",
            "required": ["schema_version", "generated_at", "assignment", "warnings"],
            "properties": {
                "schema_version": {"const": "campus-lms.assignment.v1"},
                "generated_at": {"type": "string", "format": "date-time"},
                "assignment": {"type": "object"},
                "warnings": {"type": "array"}
            }
        }),
        "capabilities.v1" | "campus-lms.capabilities.v1" => serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "campus-lms.capabilities.v1",
            "type": "object",
            "required": ["schema_version", "recommended_entrypoint", "commands", "dangerous_commands"],
            "properties": {
                "schema_version": {"const": "campus-lms.capabilities.v1"},
                "recommended_entrypoint": {"type": "string"},
                "commands": {"type": "array"},
                "dangerous_commands": {"type": "array"}
            }
        }),
        "errors.v1" | "campus-lms.errors.v1" => serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "campus-lms.errors.v1",
            "type": "object",
            "required": ["schema_version", "errors"],
            "properties": {
                "schema_version": {"const": "campus-lms.errors.v1"},
                "errors": {"type": "array"}
            }
        }),
        "error.v1" | "campus-lms.error.v1" => serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "campus-lms.error.v1",
            "type": "object",
            "required": ["schema_version", "error"],
            "properties": {
                "schema_version": {"const": "campus-lms.error.v1"},
                "error": {"type": "object"}
            }
        }),
        "init.v1" | "campus-lms.init.v1" => serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "campus-lms.init.v1",
            "type": "object",
            "required": ["schema_version", "generated_at", "config_path", "cache_dir", "created", "existing", "next_steps", "warnings"],
            "properties": {
                "schema_version": {"const": "campus-lms.init.v1"},
                "generated_at": {"type": "string", "format": "date-time"},
                "config_path": {"type": "string"},
                "cache_dir": {"type": "string"},
                "created": {"type": "array"},
                "existing": {"type": "array"},
                "agents_snippet": {"type": ["string", "null"]},
                "next_steps": {"type": "array"},
                "warnings": {"type": "array"}
            }
        }),
        "cleanup.v1" | "campus-lms.cleanup.v1" => lifecycle_schema("campus-lms.cleanup.v1"),
        "uninstall.v1" | "campus-lms.uninstall.v1" => lifecycle_schema("campus-lms.uninstall.v1"),
        _ => {
            return Err(CampusError::NotFound {
                message: format!("schema {name} is not implemented yet"),
                json: true,
            })
        }
    };
    output::print_json(&schema)
}

fn lifecycle_schema(id: &str) -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": id,
        "type": "object",
        "required": ["schema_version", "generated_at", "dry_run", "planned", "removed", "warnings"],
        "properties": {
            "schema_version": {"const": id},
            "generated_at": {"type": "string", "format": "date-time"},
            "dry_run": {"type": "boolean"},
            "planned": {"type": "array"},
            "removed": {"type": "array"},
            "npm_uninstall_command": {"type": ["string", "null"]},
            "warnings": {"type": "array"}
        }
    })
}
