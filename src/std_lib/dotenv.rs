//! Zero-dependency environment variable parser (`amber:std/dotenv`).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Parse `.env` file formatted content into a key-value map
pub fn parse_dotenv(content: &str) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        // Ignore empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Strip optional "export " prefix
        let clean_line = if trimmed.starts_with("export ") {
            trimmed["export ".len()..].trim_start()
        } else {
            trimmed
        };

        if let Some((raw_key, raw_val)) = clean_line.split_once('=') {
            let key = raw_key.trim().to_string();
            if key.is_empty() {
                continue;
            }

            let val_trimmed = raw_val.trim();
            let parsed_val = if val_trimmed.starts_with('\'')
                && val_trimmed.ends_with('\'')
                && val_trimmed.len() >= 2
            {
                // Single quoted: literal string
                val_trimmed[1..val_trimmed.len() - 1].to_string()
            } else if val_trimmed.starts_with('"')
                && val_trimmed.ends_with('"')
                && val_trimmed.len() >= 2
            {
                // Double quoted: unescape newlines and tabs
                let inner = &val_trimmed[1..val_trimmed.len() - 1];
                inner
                    .replace("\\n", "\n")
                    .replace("\\r", "\r")
                    .replace("\\t", "\t")
                    .replace("\\\"", "\"")
            } else {
                // Strip inline comment if any
                let mut val = val_trimmed;
                if let Some(idx) = val.find(" #") {
                    val = val[..idx].trim_end();
                }
                val.to_string()
            };

            // Basic variable interpolation: $KEY or ${KEY}
            let mut interpolated = parsed_val.clone();
            for (k, v) in map.iter() {
                interpolated = interpolated.replace(&format!("${{{}}}", k), v);
                interpolated = interpolated.replace(&format!("${}", k), v);
            }
            map.insert(key, interpolated);
        }
    }

    map
}

/// Load a `.env` file from disk and return the parsed key-values
pub fn load_dotenv_file(path: &Path) -> Option<HashMap<String, String>> {
    if path.exists() && path.is_file() {
        if let Ok(content) = fs::read_to_string(path) {
            return Some(parse_dotenv(&content));
        }
    }
    None
}
