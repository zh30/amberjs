//! Embedded Database & Vector Store Module for Beejs (`bee:db`, `bee:sqlite`, `bee:vector`).

pub mod sqlite;
pub mod vector;

use anyhow::Result;
use rusty_v8 as v8;
use std::sync::atomic::{AtomicBool, Ordering};

static DB_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Setup database and vector API in the V8 context
pub fn setup_db_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // 1. Register native SQLite callbacks
    let open_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let path = if args.length() > 0 && !args.get(0).is_undefined() {
                args.get(0).to_rust_string_lossy(scope)
            } else {
                ":memory:".to_string()
            };
            let readonly = if args.length() > 1 {
                args.get(1).boolean_value(scope)
            } else {
                false
            };

            match sqlite::open_database(&path, readonly) {
                Ok(handle) => {
                    retval.set(v8::Integer::new_from_unsigned(scope, handle).into());
                }
                Err(e) => {
                    let err_msg =
                        v8::String::new(scope, &format!("Failed to open database: {}", e)).unwrap();
                    let exception = v8::Exception::error(scope, err_msg);
                    scope.throw_exception(exception);
                }
            }
        },
    )
    .get_function(scope)
    .unwrap();

    let close_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let handle = if args.length() > 0 {
                args.get(0).uint32_value(scope).unwrap_or(0)
            } else {
                0
            };
            let ok = sqlite::close_database(handle);
            retval.set(v8::Boolean::new(scope, ok).into());
        },
    )
    .get_function(scope)
    .unwrap();

    let exec_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let handle = if args.length() > 0 {
                args.get(0).uint32_value(scope).unwrap_or(0)
            } else {
                0
            };
            let sql = if args.length() > 1 {
                args.get(1).to_rust_string_lossy(scope)
            } else {
                String::new()
            };

            match sqlite::exec_database(handle, &sql) {
                Ok(()) => {
                    retval.set(v8::undefined(scope).into());
                }
                Err(e) => {
                    let err_msg =
                        v8::String::new(scope, &format!("SQLite exec error: {}", e)).unwrap();
                    let exception = v8::Exception::error(scope, err_msg);
                    scope.throw_exception(exception);
                }
            }
        },
    )
    .get_function(scope)
    .unwrap();

    let query_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let handle = if args.length() > 0 {
                args.get(0).uint32_value(scope).unwrap_or(0)
            } else {
                0
            };
            let sql = if args.length() > 1 {
                args.get(1).to_rust_string_lossy(scope)
            } else {
                String::new()
            };
            let params_json = if args.length() > 2 {
                args.get(2).to_rust_string_lossy(scope)
            } else {
                "[]".to_string()
            };

            match sqlite::query_statement(handle, &sql, &params_json) {
                Ok(rows_json) => {
                    let json_str = v8::String::new(scope, &rows_json).unwrap();
                    retval.set(json_str.into());
                }
                Err(e) => {
                    let err_msg =
                        v8::String::new(scope, &format!("SQLite query error: {}", e)).unwrap();
                    let exception = v8::Exception::error(scope, err_msg);
                    scope.throw_exception(exception);
                }
            }
        },
    )
    .get_function(scope)
    .unwrap();

    let run_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let handle = if args.length() > 0 {
                args.get(0).uint32_value(scope).unwrap_or(0)
            } else {
                0
            };
            let sql = if args.length() > 1 {
                args.get(1).to_rust_string_lossy(scope)
            } else {
                String::new()
            };
            let params_json = if args.length() > 2 {
                args.get(2).to_rust_string_lossy(scope)
            } else {
                "[]".to_string()
            };

            match sqlite::run_statement(handle, &sql, &params_json) {
                Ok((changes, last_id)) => {
                    let res_obj = v8::Object::new(scope);
                    let changes_k = v8::String::new(scope, "changes").unwrap();
                    let changes_v = v8::Integer::new(scope, changes as i32);
                    res_obj.set(scope, changes_k.into(), changes_v.into());

                    let last_id_k = v8::String::new(scope, "lastInsertRowid").unwrap();
                    let last_id_v = v8::Number::new(scope, last_id as f64);
                    res_obj.set(scope, last_id_k.into(), last_id_v.into());

                    retval.set(res_obj.into());
                }
                Err(e) => {
                    let err_msg =
                        v8::String::new(scope, &format!("SQLite run error: {}", e)).unwrap();
                    let exception = v8::Exception::error(scope, err_msg);
                    scope.throw_exception(exception);
                }
            }
        },
    )
    .get_function(scope)
    .unwrap();

    // Attach native functions to internal binding object
    let db_internal = v8::Object::new(scope);
    let k_open = v8::String::new(scope, "open").unwrap();
    let k_close = v8::String::new(scope, "close").unwrap();
    let k_exec = v8::String::new(scope, "exec").unwrap();
    let k_query = v8::String::new(scope, "query").unwrap();
    let k_run = v8::String::new(scope, "run").unwrap();

    db_internal.set(scope, k_open.into(), open_fn.into());
    db_internal.set(scope, k_close.into(), close_fn.into());
    db_internal.set(scope, k_exec.into(), exec_fn.into());
    db_internal.set(scope, k_query.into(), query_fn.into());
    db_internal.set(scope, k_run.into(), run_fn.into());

    let k_db_internal = v8::String::new(scope, "__bee_db_native").unwrap();
    global.set(scope, k_db_internal.into(), db_internal.into());

    // 2. Inject Database & VectorDB JavaScript implementations
    let js_code = r#"
    (function() {
        const native = globalThis.__bee_db_native;

        // ==========================================
        // 1. Database: Embedded SQLite Class
        // ==========================================
        class Database {
            constructor(filename = ':memory:', options = {}) {
                this.filename = filename;
                this.readonly = Boolean(options.readonly);
                this._handle = native.open(filename, this.readonly);
                this._closed = false;
            }

            _checkOpen() {
                if (this._closed) {
                    throw new Error('Database is closed');
                }
            }

            exec(sql) {
                this._checkOpen();
                native.exec(this._handle, String(sql));
                return this;
            }

            prepare(sql) {
                this._checkOpen();
                const db = this;
                const sqlStr = String(sql);

                return {
                    all(...params) {
                        db._checkOpen();
                        const raw = native.query(db._handle, sqlStr, JSON.stringify(params));
                        return JSON.parse(raw);
                    },
                    get(...params) {
                        db._checkOpen();
                        const raw = native.query(db._handle, sqlStr, JSON.stringify(params));
                        const rows = JSON.parse(raw);
                        return rows.length > 0 ? rows[0] : null;
                    },
                    run(...params) {
                        db._checkOpen();
                        return native.run(db._handle, sqlStr, JSON.stringify(params));
                    }
                };
            }

            query(sql, ...params) {
                return this.prepare(sql).all(...params);
            }

            run(sql, ...params) {
                return this.prepare(sql).run(...params);
            }

            transaction(fn) {
                this._checkOpen();
                this.exec('BEGIN');
                try {
                    const result = fn();
                    this.exec('COMMIT');
                    return result;
                } catch (err) {
                    this.exec('ROLLBACK');
                    throw err;
                }
            }

            close() {
                if (!this._closed) {
                    native.close(this._handle);
                    this._closed = true;
                }
            }
        }

        // ==========================================
        // 2. VectorDB: Agentic Vector Search Engine
        // ==========================================
        function toFloat32Array(vec) {
            if (vec instanceof Float32Array) return vec;
            if (Array.isArray(vec)) return new Float32Array(vec);
            if (vec && vec.data instanceof Float32Array) return vec.data; // bee:ai Tensor
            if (ArrayBuffer.isView(vec)) return new Float32Array(vec.buffer, vec.byteOffset, vec.byteLength / 4);
            throw new TypeError('Vector must be an Array, Float32Array, or Tensor');
        }

        function dotProduct(a, b) {
            let sum = 0;
            const len = a.length;
            for (let i = 0; i < len; i++) {
                sum += a[i] * b[i];
            }
            return sum;
        }

        function euclideanDistance(a, b) {
            let sum = 0;
            const len = a.length;
            for (let i = 0; i < len; i++) {
                const diff = a[i] - b[i];
                sum += diff * diff;
            }
            return Math.sqrt(sum);
        }

        function cosineSimilarity(a, b) {
            let dot = 0;
            let normA = 0;
            let normB = 0;
            const len = a.length;
            for (let i = 0; i < len; i++) {
                const x = a[i];
                const y = b[i];
                dot += x * y;
                normA += x * x;
                normB += y * y;
            }
            if (normA === 0 || normB === 0) return 0;
            return dot / (Math.sqrt(normA) * Math.sqrt(normB));
        }

        class VectorDB {
            constructor(options = {}) {
                this.dimensions = options.dimensions || 0;
                this.metric = options.metric || 'cosine'; // 'cosine' | 'euclidean' | 'dot'
                this.entries = new Map(); // id -> { id, vector: Float32Array, metadata }
            }

            insert(id, vector, metadata = {}) {
                const idStr = String(id);
                const v = toFloat32Array(vector);
                if (this.dimensions > 0 && v.length !== this.dimensions) {
                    throw new Error(`Vector dimensions mismatch: expected ${this.dimensions}, got ${v.length}`);
                }
                if (this.dimensions === 0) {
                    this.dimensions = v.length;
                }
                this.entries.set(idStr, {
                    id: idStr,
                    vector: v,
                    metadata
                });
                return this;
            }

            delete(id) {
                return this.entries.delete(String(id));
            }

            get(id) {
                const item = this.entries.get(String(id));
                if (!item) return null;
                return {
                    id: item.id,
                    vector: item.vector,
                    metadata: item.metadata
                };
            }

            get size() {
                return this.entries.size;
            }

            search(queryVector, options = {}) {
                const query = toFloat32Array(queryVector);
                if (this.dimensions > 0 && query.length !== this.dimensions) {
                    throw new Error(`Query vector dimensions mismatch: expected ${this.dimensions}, got ${query.length}`);
                }

                const topK = options.topK || 10;
                const threshold = options.threshold !== undefined ? options.threshold : -Infinity;
                const filter = options.filter || null;

                const results = [];
                for (const entry of this.entries.values()) {
                    if (filter && !filter(entry.metadata)) {
                        continue;
                    }

                    let score = 0;
                    if (this.metric === 'cosine') {
                        score = cosineSimilarity(query, entry.vector);
                    } else if (this.metric === 'euclidean') {
                        const dist = euclideanDistance(query, entry.vector);
                        score = 1 / (1 + dist);
                    } else if (this.metric === 'dot') {
                        score = dotProduct(query, entry.vector);
                    }

                    if (score >= threshold) {
                        results.push({
                            id: entry.id,
                            score,
                            metadata: entry.metadata,
                            vector: entry.vector
                        });
                    }
                }

                results.sort((a, b) => b.score - a.score);
                return results.slice(0, topK);
            }

            toJSON() {
                const arr = [];
                for (const entry of this.entries.values()) {
                    arr.push({
                        id: entry.id,
                        vector: Array.from(entry.vector),
                        metadata: entry.metadata
                    });
                }
                return {
                    dimensions: this.dimensions,
                    metric: this.metric,
                    entries: arr
                };
            }

            static fromJSON(data) {
                const vdb = new VectorDB({
                    dimensions: data.dimensions,
                    metric: data.metric
                });
                if (Array.isArray(data.entries)) {
                    for (const item of data.entries) {
                        vdb.insert(item.id, item.vector, item.metadata);
                    }
                }
                return vdb;
            }
        }

        // ==========================================
        // 3. Exports Setup
        // ==========================================
        const beeDb = {
            Database,
            VectorDB,
            cosineSimilarity,
            euclideanDistance,
            dotProduct,
            version: '1.4.0'
        };

        const beeVector = {
            VectorDB,
            cosineSimilarity,
            euclideanDistance,
            dotProduct,
            version: '1.4.0'
        };

        globalThis.__bee_db = beeDb;
        globalThis.__bee_vector = beeVector;
        globalThis.db = beeDb;
        globalThis.vector = beeVector;
        globalThis.Database = Database;
        globalThis.VectorDB = VectorDB;
    })();
    "#;

    let script_source = v8::String::new(scope, js_code).unwrap();
    if let Some(script) = v8::Script::compile(scope, script_source, None) {
        let _ = script.run(scope);
    }

    DB_INITIALIZED.store(true, Ordering::SeqCst);
    Ok(())
}
