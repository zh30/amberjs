//! Embedded SQLite Database engine for Amber (`amber:db` / `amber:sqlite`).
//!
//! Provides zero-dependency, in-process SQLite storage powered by native `rusqlite`.
//! Supports both in-memory databases and file-based persistence, prepared statements,
//! and automatic transactions.

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use rusqlite::types::ValueRef;
use rusqlite::{params_from_iter, Connection, OpenFlags};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);
static DB_CONNECTIONS: Lazy<Mutex<HashMap<u32, Connection>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Open a SQLite database and return a unique numeric handle
pub fn open_database(path: &str, readonly: bool) -> Result<u32> {
    let conn = if path == ":memory:" || path.is_empty() {
        Connection::open_in_memory()?
    } else {
        let flags = if readonly {
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_URI
        };
        Connection::open_with_flags(path, flags)?
    };

    let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut conns = DB_CONNECTIONS
        .lock()
        .map_err(|_| anyhow!("Failed to lock DB registry"))?;
    conns.insert(handle, conn);
    Ok(handle)
}

/// Close a database by handle
pub fn close_database(handle: u32) -> bool {
    if let Ok(mut conns) = DB_CONNECTIONS.lock() {
        conns.remove(&handle).is_some()
    } else {
        false
    }
}

/// Execute one or multiple DDL / SQL statements (batch execution)
pub fn exec_database(handle: u32, sql: &str) -> Result<()> {
    let conns = DB_CONNECTIONS
        .lock()
        .map_err(|_| anyhow!("Failed to lock DB registry"))?;
    let conn = conns
        .get(&handle)
        .ok_or_else(|| anyhow!("Database handle {} is closed or invalid", handle))?;
    conn.execute_batch(sql)?;
    Ok(())
}

fn json_to_sqlite_value(v: &Value) -> rusqlite::types::Value {
    match v {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                rusqlite::types::Value::Real(f)
            } else {
                rusqlite::types::Value::Null
            }
        }
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        Value::Array(_) | Value::Object(_) => rusqlite::types::Value::Text(v.to_string()),
    }
}

/// Run a statement (INSERT/UPDATE/DELETE) with parameters, returning (changes, last_insert_rowid)
pub fn run_statement(handle: u32, sql: &str, params_json: &str) -> Result<(usize, i64)> {
    let conns = DB_CONNECTIONS
        .lock()
        .map_err(|_| anyhow!("Failed to lock DB registry"))?;
    let conn = conns
        .get(&handle)
        .ok_or_else(|| anyhow!("Database handle {} is closed or invalid", handle))?;

    let parsed_params: Value = if params_json.is_empty() {
        Value::Array(vec![])
    } else {
        serde_json::from_str(params_json).unwrap_or(Value::Array(vec![]))
    };

    let params_vec: Vec<rusqlite::types::Value> = match parsed_params {
        Value::Array(arr) => arr.iter().map(json_to_sqlite_value).collect(),
        _ => vec![],
    };

    let changes = conn.execute(sql, params_from_iter(params_vec.iter()))?;
    let last_id = conn.last_insert_rowid();
    Ok((changes, last_id))
}

/// Execute a query (SELECT) with parameters, returning a JSON array of row objects
pub fn query_statement(handle: u32, sql: &str, params_json: &str) -> Result<String> {
    let conns = DB_CONNECTIONS
        .lock()
        .map_err(|_| anyhow!("Failed to lock DB registry"))?;
    let conn = conns
        .get(&handle)
        .ok_or_else(|| anyhow!("Database handle {} is closed or invalid", handle))?;

    let parsed_params: Value = if params_json.is_empty() {
        Value::Array(vec![])
    } else {
        serde_json::from_str(params_json).unwrap_or(Value::Array(vec![]))
    };

    let params_vec: Vec<rusqlite::types::Value> = match parsed_params {
        Value::Array(arr) => arr.iter().map(json_to_sqlite_value).collect(),
        _ => vec![],
    };

    let mut stmt = conn.prepare(sql)?;
    let col_names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let mut rows = stmt.query(params_from_iter(params_vec.iter()))?;
    let mut results: Vec<Value> = Vec::new();

    while let Some(row) = rows.next()? {
        let mut obj = serde_json::Map::new();
        for (i, name) in col_names.iter().enumerate() {
            let val = match row.get_ref(i)? {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(n) => json!(n),
                ValueRef::Real(f) => json!(f),
                ValueRef::Text(s) => {
                    let s_str = String::from_utf8_lossy(s);
                    Value::String(s_str.to_string())
                }
                ValueRef::Blob(b) => {
                    // Blob as array of numbers
                    let bytes: Vec<Value> = b.iter().map(|byte| json!(byte)).collect();
                    Value::Array(bytes)
                }
            };
            obj.insert(name.clone(), val);
        }
        results.push(Value::Object(obj));
    }

    Ok(serde_json::to_string(&results)?)
}
