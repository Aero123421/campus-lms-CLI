// SPDX-License-Identifier: Apache-2.0

use schemars::{schema_for, JsonSchema};
use serde_json::{json, Value};

use crate::{
    cli::SchemaCommand,
    dto::{
        AiSnapshotOutput, AssignmentOutput, AuthLoginOutput, AuthLogoutOutput, AuthStatusOutput,
        CoursesOutput, DoctorOutput, SchemaListOutput, TodoOutput, WhoamiOutput,
    },
    error::CampusError,
    output,
};

const SCHEMAS: &[&str] = &[
    "auth_login.v1",
    "auth_logout.v1",
    "auth_status.v1",
    "whoami.v1",
    "doctor.v1",
    "courses.v1",
    "todo.v1",
    "assignment.v1",
    "ai_snapshot.v1",
    "schema_list.v1",
    "capabilities.v1",
    "errors.v1",
    "error.v1",
    "init.v1",
    "cleanup.v1",
    "uninstall.v1",
];

pub fn run(command: &SchemaCommand) -> crate::error::Result<()> {
    match command {
        SchemaCommand::List => output::print_json(&SchemaListOutput {
            schema_version: "campus-lms.schema_list.v1",
            generated_at: output::generated_at(),
            schemas: SCHEMAS.iter().map(|name| (*name).to_string()).collect(),
            warnings: Vec::new(),
        }),
        SchemaCommand::Show { name } => show(name),
    }
}

fn show(name: &str) -> crate::error::Result<()> {
    let schema = match name {
        "auth_login.v1" | "campus-lms.auth_login.v1" => {
            generated_schema::<AuthLoginOutput>("campus-lms.auth_login.v1")?
        }
        "auth_logout.v1" | "campus-lms.auth_logout.v1" => {
            generated_schema::<AuthLogoutOutput>("campus-lms.auth_logout.v1")?
        }
        "auth_status.v1" | "campus-lms.auth_status.v1" => {
            generated_schema::<AuthStatusOutput>("campus-lms.auth_status.v1")?
        }
        "whoami.v1" | "campus-lms.whoami.v1" => {
            generated_schema::<WhoamiOutput>("campus-lms.whoami.v1")?
        }
        "doctor.v1" | "campus-lms.doctor.v1" => {
            generated_schema::<DoctorOutput>("campus-lms.doctor.v1")?
        }
        "courses.v1" | "campus-lms.courses.v1" => {
            generated_schema::<CoursesOutput>("campus-lms.courses.v1")?
        }
        "todo.v1" | "campus-lms.todo.v1" => generated_schema::<TodoOutput>("campus-lms.todo.v1")?,
        "assignment.v1" | "campus-lms.assignment.v1" => {
            generated_schema::<AssignmentOutput>("campus-lms.assignment.v1")?
        }
        "ai_snapshot.v1" | "campus-lms.ai_snapshot.v1" => {
            generated_schema::<AiSnapshotOutput>("campus-lms.ai_snapshot.v1")?
        }
        "schema_list.v1" | "campus-lms.schema_list.v1" => {
            generated_schema::<SchemaListOutput>("campus-lms.schema_list.v1")?
        }
        "capabilities.v1" | "campus-lms.capabilities.v1" => static_schema(
            "campus-lms.capabilities.v1",
            &[
                "schema_version",
                "recommended_entrypoint",
                "commands",
                "dangerous_commands",
            ],
            json!({
                "schema_version": {"const": "campus-lms.capabilities.v1"},
                "recommended_entrypoint": {"type": "string"},
                "commands": {"type": "array"},
                "dangerous_commands": {"type": "array"}
            }),
        ),
        "errors.v1" | "campus-lms.errors.v1" => static_schema(
            "campus-lms.errors.v1",
            &["schema_version", "errors"],
            json!({
                "schema_version": {"const": "campus-lms.errors.v1"},
                "errors": {"type": "array"}
            }),
        ),
        "error.v1" | "campus-lms.error.v1" => static_schema(
            "campus-lms.error.v1",
            &["schema_version", "error"],
            json!({
                "schema_version": {"const": "campus-lms.error.v1"},
                "error": {"type": "object"}
            }),
        ),
        "init.v1" | "campus-lms.init.v1" => static_schema(
            "campus-lms.init.v1",
            &[
                "schema_version",
                "generated_at",
                "config_path",
                "cache_dir",
                "created",
                "existing",
                "next_steps",
                "warnings",
            ],
            json!({
                "schema_version": {"const": "campus-lms.init.v1"},
                "generated_at": {"type": "string", "format": "date-time"},
                "config_path": {"type": "string"},
                "cache_dir": {"type": "string"},
                "created": {"type": "array", "items": {"type": "string"}},
                "existing": {"type": "array", "items": {"type": "string"}},
                "agents_snippet": {"type": ["string", "null"]},
                "next_steps": {"type": "array", "items": {"type": "string"}},
                "warnings": {"type": "array"}
            }),
        ),
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

fn generated_schema<T: JsonSchema>(id: &str) -> crate::error::Result<Value> {
    let mut value = serde_json::to_value(schema_for!(T)).map_err(|err| CampusError::Parse {
        message: format!("failed to serialize generated schema: {err}"),
        json: true,
    })?;
    if let Value::Object(map) = &mut value {
        map.insert("$id".to_string(), Value::String(id.to_string()));
        if let Some(Value::Object(properties)) = map.get_mut("properties") {
            if properties.contains_key("schema_version") {
                properties.insert("schema_version".to_string(), json!({ "const": id }));
            }
        }
    }
    Ok(value)
}

fn static_schema(id: &str, required: &[&str], properties: Value) -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": id,
        "type": "object",
        "required": required,
        "properties": properties
    })
}

fn lifecycle_schema(id: &str) -> Value {
    static_schema(
        id,
        &[
            "schema_version",
            "generated_at",
            "dry_run",
            "planned",
            "removed",
            "warnings",
        ],
        json!({
            "schema_version": {"const": id},
            "generated_at": {"type": "string", "format": "date-time"},
            "dry_run": {"type": "boolean"},
            "planned": {"type": "array", "items": {"type": "string"}},
            "removed": {"type": "array", "items": {"type": "string"}},
            "npm_uninstall_command": {"type": ["string", "null"]},
            "warnings": {"type": "array"}
        }),
    )
}
