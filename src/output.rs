// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::error::{CampusError, ErrorBody, ErrorResponse, Result};

pub fn generated_at() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value).map_err(|err| CampusError::Parse {
        message: err.to_string(),
        json: true,
    })?;
    println!("{text}");
    Ok(())
}

pub fn print_error(err: &CampusError) -> Result<()> {
    let response = ErrorResponse {
        schema_version: "campus-lms.error.v1",
        error: ErrorBody {
            code: err.code(),
            message: err.to_string(),
            retryable: err.retryable(),
            hint: err.hint(),
        },
    };
    let text = serde_json::to_string_pretty(&response).map_err(|json_err| CampusError::Parse {
        message: json_err.to_string(),
        json: true,
    })?;
    eprintln!("{text}");
    Ok(())
}
