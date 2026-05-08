// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;

pub fn flatten_params(value: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Value::Object(map) = value {
        for (key, item) in map {
            flatten(key, item, &mut out);
        }
    }
    out
}

fn flatten(prefix: &str, value: &Value, out: &mut Vec<(String, String)>) {
    match value {
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                flatten(&format!("{prefix}[{index}]"), item, out);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                flatten(&format!("{prefix}[{key}]"), item, out);
            }
        }
        Value::Null => {}
        Value::Bool(value) => out.push((
            prefix.to_string(),
            if *value { "1" } else { "0" }.to_string(),
        )),
        Value::Number(value) => out.push((prefix.to_string(), value.to_string())),
        Value::String(value) => out.push((prefix.to_string(), value.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattens_moodle_array_params() {
        let params = serde_json::json!({
            "courseids": [101, 102],
            "options": {"ids": [7]}
        });
        let flattened = flatten_params(&params);
        assert!(flattened.contains(&("courseids[0]".to_string(), "101".to_string())));
        assert!(flattened.contains(&("courseids[1]".to_string(), "102".to_string())));
        assert!(flattened.contains(&("options[ids][0]".to_string(), "7".to_string())));
    }
}
