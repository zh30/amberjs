//! Official modern standard library for Beejs (`bee:std`, `bee:std/*`).

pub mod assert;
pub mod cli;
pub mod crypto;
pub mod dotenv;
pub mod fs;

use anyhow::Result;
use rusty_v8 as v8;
use std::path::Path;

/// Register standard library APIs in the V8 context
pub fn setup_std_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // 1. Native callbacks for std_lib
    let dotenv_parse_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let content = if args.length() > 0 {
                args.get(0).to_rust_string_lossy(scope)
            } else {
                String::new()
            };

            let parsed = dotenv::parse_dotenv(&content);
            let json_str = serde_json::to_string(&parsed).unwrap_or_else(|_| "{}".to_string());
            let v8_str = v8::String::new(scope, &json_str).unwrap();
            retval.set(v8_str.into());
        },
    )
    .get_function(scope)
    .unwrap();

    let cli_table_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let headers_json = if args.length() > 0 {
                args.get(0).to_rust_string_lossy(scope)
            } else {
                "[]".to_string()
            };
            let rows_json = if args.length() > 1 {
                args.get(1).to_rust_string_lossy(scope)
            } else {
                "[]".to_string()
            };

            let headers: Vec<String> = serde_json::from_str(&headers_json).unwrap_or_default();
            let rows: Vec<Vec<String>> = serde_json::from_str(&rows_json).unwrap_or_default();

            let table_str = cli::format_table(&headers, &rows);
            let v8_str = v8::String::new(scope, &table_str).unwrap();
            retval.set(v8_str.into());
        },
    )
    .get_function(scope)
    .unwrap();

    let fs_walk_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let dir_str = if args.length() > 0 {
                args.get(0).to_rust_string_lossy(scope)
            } else {
                ".".to_string()
            };
            let max_depth = if args.length() > 1 && !args.get(1).is_undefined() {
                args.get(1).uint32_value(scope).map(|d| d as usize)
            } else {
                None
            };
            let exts_json = if args.length() > 2 && !args.get(2).is_undefined() {
                Some(args.get(2).to_rust_string_lossy(scope))
            } else {
                None
            };

            let exts: Option<Vec<String>> = exts_json.and_then(|j| serde_json::from_str(&j).ok());

            match fs::walk_dir_sync(Path::new(&dir_str), max_depth, exts.as_deref()) {
                Ok(entries) => {
                    let json_entries: Vec<serde_json::Value> = entries
                        .into_iter()
                        .map(|e| {
                            serde_json::json!({
                                "path": e.path,
                                "name": e.name,
                                "isFile": e.is_file,
                                "isDirectory": e.is_dir,
                                "size": e.size
                            })
                        })
                        .collect();
                    let json_str =
                        serde_json::to_string(&json_entries).unwrap_or_else(|_| "[]".to_string());
                    let v8_str = v8::String::new(scope, &json_str).unwrap();
                    retval.set(v8_str.into());
                }
                Err(e) => {
                    let err_msg = v8::String::new(scope, &format!("walkDir error: {}", e)).unwrap();
                    let exception = v8::Exception::error(scope, err_msg);
                    scope.throw_exception(exception);
                }
            }
        },
    )
    .get_function(scope)
    .unwrap();

    let fs_copy_dir_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let src = if args.length() > 0 {
                args.get(0).to_rust_string_lossy(scope)
            } else {
                String::new()
            };
            let dest = if args.length() > 1 {
                args.get(1).to_rust_string_lossy(scope)
            } else {
                String::new()
            };

            match fs::copy_dir_sync(Path::new(&src), Path::new(&dest)) {
                Ok(()) => {
                    retval.set(v8::undefined(scope).into());
                }
                Err(e) => {
                    let err_msg = v8::String::new(scope, &format!("copyDir error: {}", e)).unwrap();
                    let exception = v8::Exception::error(scope, err_msg);
                    scope.throw_exception(exception);
                }
            }
        },
    )
    .get_function(scope)
    .unwrap();

    let fs_empty_dir_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let dir = if args.length() > 0 {
                args.get(0).to_rust_string_lossy(scope)
            } else {
                String::new()
            };

            match fs::empty_dir_sync(Path::new(&dir)) {
                Ok(()) => {
                    retval.set(v8::undefined(scope).into());
                }
                Err(e) => {
                    let err_msg =
                        v8::String::new(scope, &format!("emptyDir error: {}", e)).unwrap();
                    let exception = v8::Exception::error(scope, err_msg);
                    scope.throw_exception(exception);
                }
            }
        },
    )
    .get_function(scope)
    .unwrap();

    let crypto_uuid_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let uuid_str = crypto::generate_uuid_v4();
            let v8_str = v8::String::new(scope, &uuid_str).unwrap();
            retval.set(v8_str.into());
        },
    )
    .get_function(scope)
    .unwrap();

    let crypto_uuidv7_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let uuid_str = crypto::generate_uuid_v7();
            let v8_str = v8::String::new(scope, &uuid_str).unwrap();
            retval.set(v8_str.into());
        },
    )
    .get_function(scope)
    .unwrap();

    // Attach native callbacks to __bee_std_native
    let std_internal = v8::Object::new(scope);
    let k_dotenv = v8::String::new(scope, "dotenvParse").unwrap();
    let k_table = v8::String::new(scope, "cliTable").unwrap();
    let k_walk = v8::String::new(scope, "fsWalk").unwrap();
    let k_copydir = v8::String::new(scope, "fsCopyDir").unwrap();
    let k_emptydir = v8::String::new(scope, "fsEmptyDir").unwrap();
    let k_uuid = v8::String::new(scope, "uuid").unwrap();
    let k_uuidv7 = v8::String::new(scope, "uuidv7").unwrap();

    std_internal.set(scope, k_dotenv.into(), dotenv_parse_fn.into());
    std_internal.set(scope, k_table.into(), cli_table_fn.into());
    std_internal.set(scope, k_walk.into(), fs_walk_fn.into());
    std_internal.set(scope, k_copydir.into(), fs_copy_dir_fn.into());
    std_internal.set(scope, k_emptydir.into(), fs_empty_dir_fn.into());
    std_internal.set(scope, k_uuid.into(), crypto_uuid_fn.into());
    std_internal.set(scope, k_uuidv7.into(), crypto_uuidv7_fn.into());

    let k_std_native = v8::String::new(scope, "__bee_std_native").unwrap();
    global.set(scope, k_std_native.into(), std_internal.into());

    // 2. Load assert JS implementation
    let assert_src = v8::String::new(scope, assert::ASSERT_JS_CODE).unwrap();
    if let Some(script) = v8::Script::compile(scope, assert_src, None) {
        let _ = script.run(scope);
    }

    // 3. Inject rich JavaScript standard modules
    let js_code = r#"
    (function() {
        const native = globalThis.__bee_std_native;
        const assertModule = globalThis.__bee_assert;

        // ==========================================
        // 1. dotenv: Environment Variables
        // ==========================================
        const dotenv = {
            parse(src) {
                if (typeof src !== 'string') return {};
                return JSON.parse(native.dotenvParse(src));
            },
            config(options = {}) {
                const fs = require('fs');
                const path = require('path');
                const envPath = options.path || '.env';
                const resolved = path.resolve(process.cwd(), envPath);
                const override = Boolean(options.override);

                if (!fs.existsSync(resolved)) {
                    return { parsed: {}, error: new Error(`File not found: ${resolved}`) };
                }

                try {
                    const content = fs.readFileSync(resolved, 'utf8');
                    const parsed = this.parse(content);
                    for (const [key, value] of Object.entries(parsed)) {
                        if (override || process.env[key] === undefined) {
                            process.env[key] = value;
                        }
                    }
                    return { parsed };
                } catch (err) {
                    return { parsed: {}, error: err };
                }
            }
        };

        // ==========================================
        // 2. cli: Colors, Tables, Progress & Prompt
        // ==========================================
        const colorCodes = {
            reset: '\x1b[0m',
            bold: '\x1b[1m',
            dim: '\x1b[2m',
            italic: '\x1b[3m',
            underline: '\x1b[4m',
            red: '\x1b[31m',
            green: '\x1b[32m',
            yellow: '\x1b[33m',
            blue: '\x1b[34m',
            magenta: '\x1b[35m',
            cyan: '\x1b[36m',
            white: '\x1b[37m',
            gray: '\x1b[90m',
            bgRed: '\x1b[41m',
            bgGreen: '\x1b[42m',
            bgYellow: '\x1b[43m',
            bgBlue: '\x1b[44m'
        };

        const colors = {};
        for (const [name, code] of Object.entries(colorCodes)) {
            colors[name] = function(str) {
                return `${code}${str}${colorCodes.reset}`;
            };
        }

        const cli = {
            colors,
            table(data, columns = null) {
                if (!Array.isArray(data) || data.length === 0) return '';
                const headers = columns || Object.keys(data[0]);
                const rows = data.map(item => {
                    return headers.map(h => {
                        const val = item[h];
                        return val !== undefined && val !== null ? String(val) : '';
                    });
                });
                return native.cliTable(JSON.stringify(headers), JSON.stringify(rows));
            },
            progressBar(options = {}) {
                const total = options.total || 100;
                const width = options.width || 30;
                let current = 0;

                return {
                    tick(delta = 1) {
                        current = Math.min(total, current + delta);
                        this.render();
                    },
                    render() {
                        const percent = Math.floor((current / total) * 100);
                        const filled = Math.floor((current / total) * width);
                        const empty = width - filled;
                        const bar = '█'.repeat(filled) + '░'.repeat(empty);
                        process.stdout.write(`\r[${bar}] ${percent}% (${current}/${total})`);
                        if (current >= total) {
                            process.stdout.write('\n');
                        }
                    },
                    complete() {
                        current = total;
                        this.render();
                    }
                };
            },
            async prompt(question, defaultValue = '') {
                const readline = require('readline');
                const rl = readline.createInterface({
                    input: process.stdin,
                    output: process.stdout
                });
                return new Promise(resolve => {
                    const q = defaultValue ? `${question} (${defaultValue}): ` : `${question}: `;
                    rl.question(q, answer => {
                        rl.close();
                        resolve(answer.trim() || defaultValue);
                    });
                });
            }
        };

        // ==========================================
        // 3. fs: High-level Directory & File Utils
        // ==========================================
        const stdFs = {
            walkDir(dir = '.', options = {}) {
                const maxDepth = options.maxDepth !== undefined ? options.maxDepth : undefined;
                const exts = options.exts ? JSON.stringify(options.exts) : undefined;
                const raw = native.fsWalk(String(dir), maxDepth, exts);
                return JSON.parse(raw);
            },
            copyDir(src, dest) {
                native.fsCopyDir(String(src), String(dest));
            },
            emptyDir(dir) {
                native.fsEmptyDir(String(dir));
            },
            ensureDir(dir) {
                const fs = require('fs');
                if (!fs.existsSync(dir)) {
                    fs.mkdirSync(dir, { recursive: true });
                }
            },
            ensureFile(file) {
                const fs = require('fs');
                const path = require('path');
                this.ensureDir(path.dirname(file));
                if (!fs.existsSync(file)) {
                    fs.writeFileSync(file, '');
                }
            }
        };

        // ==========================================
        // 4. crypto: JWT, Hashes & UUIDs
        // ==========================================
        function base64UrlEncode(str) {
            const buf = Buffer.from(str);
            return buf.toString('base64').replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
        }

        function base64UrlDecode(str) {
            let base64 = str.replace(/-/g, '+').replace(/_/g, '/');
            while (base64.length % 4) {
                base64 += '=';
            }
            return Buffer.from(base64, 'base64').toString('utf8');
        }

        const stdCrypto = {
            uuid() {
                return native.uuid();
            },
            uuidv7() {
                return native.uuidv7();
            },
            hash(algorithm, data, encoding = 'hex') {
                const crypto = require('crypto');
                return crypto.createHash(algorithm).update(data).digest(encoding);
            },
            jwt: {
                sign(payload, secret, options = {}) {
                    const crypto = require('crypto');
                    const header = {
                        alg: 'HS256',
                        typ: 'JWT'
                    };
                    const iat = Math.floor(Date.now() / 1000);
                    const body = { iat, ...payload };
                    if (options.expiresIn) {
                        const secs = typeof options.expiresIn === 'number'
                            ? options.expiresIn
                            : parseInt(options.expiresIn, 10);
                        body.exp = iat + secs;
                    }

                    const encodedHeader = base64UrlEncode(JSON.stringify(header));
                    const encodedPayload = base64UrlEncode(JSON.stringify(body));
                    const dataToSign = `${encodedHeader}.${encodedPayload}`;

                    const signature = crypto.createHmac('sha256', secret)
                        .update(dataToSign)
                        .digest('base64')
                        .replace(/=/g, '')
                        .replace(/\+/g, '-')
                        .replace(/\//g, '_');

                    return `${dataToSign}.${signature}`;
                },
                verify(token, secret) {
                    const crypto = require('crypto');
                    const parts = String(token).split('.');
                    if (parts.length !== 3) {
                        throw new Error('Invalid JWT format');
                    }
                    const [encodedHeader, encodedPayload, signature] = parts;
                    const dataToSign = `${encodedHeader}.${encodedPayload}`;
                    const expectedSig = crypto.createHmac('sha256', secret)
                        .update(dataToSign)
                        .digest('base64')
                        .replace(/=/g, '')
                        .replace(/\+/g, '-')
                        .replace(/\//g, '_');

                    if (signature !== expectedSig) {
                        throw new Error('JWT signature mismatch');
                    }

                    const payload = JSON.parse(base64UrlDecode(encodedPayload));
                    if (payload.exp && Math.floor(Date.now() / 1000) > payload.exp) {
                        throw new Error('JWT token expired');
                    }
                    return payload;
                }
            }
        };

        // ==========================================
        // 5. Global & Modules Binding
        // ==========================================
        const beeStd = {
            dotenv,
            cli,
            fs: stdFs,
            crypto: stdCrypto,
            assert: assertModule.assert,
            assertEquals: assertModule.assertEquals,
            assertNotEquals: assertModule.assertNotEquals,
            assertThrows: assertModule.assertThrows,
            version: '1.4.0'
        };

        globalThis.__bee_std = beeStd;
        globalThis.__bee_std_dotenv = dotenv;
        globalThis.__bee_std_cli = cli;
        globalThis.__bee_std_fs = stdFs;
        globalThis.__bee_std_crypto = stdCrypto;
        globalThis.__bee_std_assert = assertModule;
        globalThis.std = beeStd;
    })();
    "#;

    let script_source = v8::String::new(scope, js_code).unwrap();
    if let Some(script) = v8::Script::compile(scope, script_source, None) {
        let _ = script.run(scope);
    }

    Ok(())
}
