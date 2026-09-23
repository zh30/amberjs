// Amber v1.6.0: Deterministic Agent Replay Engine (`amber:replay`)
//
// Provides deterministic recording and time-travel replay for AI Agents:
// - Intercepts and records non-deterministic runtime sources (time, random, fetch, fs, agent steps)
// - Emits portable, serialized .amber-trace.json artifacts
// - Offline replay with divergence detection (pinpoints the exact step where an agent deviates)

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::RwLock;

use once_cell::sync::Lazy;
use rusty_v8 as v8;
use serde::{Deserialize, Serialize};

/// Trace Event Record representing a non-deterministic interaction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum TraceEvent {
    #[serde(rename = "time_now")]
    TimeNow { timestamp_ms: u64 },

    #[serde(rename = "random")]
    Random { value: f64 },

    #[serde(rename = "fetch")]
    Fetch {
        url: String,
        status: u16,
        headers: HashMap<String, String>,
        body: String,
    },

    #[serde(rename = "fs_read")]
    FsRead { path: String, content: String },

    #[serde(rename = "agent_step")]
    AgentStep {
        name: String,
        input: serde_json::Value,
        output: serde_json::Value,
    },

    #[serde(rename = "custom")]
    Custom {
        key: String,
        data: serde_json::Value,
    },
}

/// Comprehensive Trace Schema for serialization/deserialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub version: String,
    pub created_at: String,
    pub platform: String,
    pub arch: String,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub random_seed: Option<u64>,
    pub events: Vec<TraceEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayMode {
    Idle,
    Recording,
    Replaying,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayStats {
    pub mode: String,
    pub is_recording: bool,
    pub is_replaying: bool,
    pub total_events: usize,
    pub step_count: usize,
    pub current_cursor: usize,
    pub script: Option<String>,
}

/// Thread-safe Replay Engine state
pub struct ReplayEngine {
    pub mode: ReplayMode,
    pub recorded_events: Vec<TraceEvent>,
    pub current_trace: Option<Trace>,
    pub cursor: usize,
    pub step_cursor: usize,
    pub script_path: Option<String>,
    pub output_path: Option<String>,
}

impl ReplayEngine {
    pub fn new() -> Self {
        Self {
            mode: ReplayMode::Idle,
            recorded_events: Vec::new(),
            current_trace: None,
            cursor: 0,
            step_cursor: 0,
            script_path: None,
            output_path: None,
        }
    }

    pub fn start_recording(&mut self, script: Option<String>, output_path: Option<String>) {
        self.mode = ReplayMode::Recording;
        self.recorded_events.clear();
        self.current_trace = None;
        self.cursor = 0;
        self.step_cursor = 0;
        self.script_path = script;
        self.output_path = output_path;
    }

    pub fn stop_recording(&mut self, output_file: Option<&str>) -> Result<Trace, String> {
        if self.mode != ReplayMode::Recording {
            return Err("Cannot stop recording: engine is not in recording mode".to_string());
        }

        let now_iso = chrono_lite_now();
        let trace = Trace {
            version: "1.6.0".to_string(),
            created_at: now_iso,
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            script: self.script_path.clone(),
            random_seed: Some(133742),
            events: self.recorded_events.clone(),
        };

        let target_path = output_file
            .map(|s| s.to_string())
            .or_else(|| self.output_path.clone());

        if let Some(ref path_str) = target_path {
            let json = serde_json::to_string_pretty(&trace)
                .map_err(|e| format!("Failed to serialize trace: {}", e))?;
            let mut file = File::create(path_str)
                .map_err(|e| format!("Failed to write trace file '{}': {}", path_str, e))?;
            file.write_all(json.as_bytes())
                .map_err(|e| format!("Failed to write trace bytes: {}", e))?;
        }

        self.mode = ReplayMode::Idle;
        self.current_trace = Some(trace.clone());
        Ok(trace)
    }

    pub fn load_trace(&mut self, trace: Trace) {
        self.mode = ReplayMode::Replaying;
        self.recorded_events = trace.events.clone();
        if let Some(seed) = trace.random_seed {
            crate::permissions::set_deterministic_seed(Some(seed));
        }
        self.current_trace = Some(trace);
        self.cursor = 0;
        self.step_cursor = 0;
    }

    pub fn load_trace_from_file(&mut self, path: &str) -> Result<(), String> {
        let mut file =
            File::open(path).map_err(|e| format!("Failed to open trace file '{}': {}", path, e))?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| format!("Failed to read trace file: {}", e))?;

        let trace: Trace = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse trace JSON: {}", e))?;

        self.load_trace(trace);
        Ok(())
    }

    pub fn record_event(&mut self, event: TraceEvent) {
        if self.mode == ReplayMode::Recording {
            self.recorded_events.push(event);
        }
    }

    pub fn step(
        &mut self,
        name: &str,
        input: serde_json::Value,
        output: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match self.mode {
            ReplayMode::Recording => {
                let event = TraceEvent::AgentStep {
                    name: name.to_string(),
                    input,
                    output: output.clone(),
                };
                self.recorded_events.push(event);
                Ok(output)
            }
            ReplayMode::Replaying => {
                let events = match &self.current_trace {
                    Some(t) => &t.events,
                    None => &self.recorded_events,
                };

                let mut found_index = None;
                for i in self.step_cursor..events.len() {
                    if let TraceEvent::AgentStep { .. } = &events[i] {
                        found_index = Some(i);
                        break;
                    }
                }

                let idx = match found_index {
                    Some(i) => i,
                    None => {
                        return Err(format!(
                            "ReplayDivergenceError: Extra step '{}' executed beyond recorded trace boundary (step cursor: {})",
                            name, self.step_cursor
                        ));
                    }
                };

                if let TraceEvent::AgentStep {
                    name: rec_name,
                    input: rec_input,
                    output: rec_output,
                } = &events[idx]
                {
                    if rec_name != name {
                        return Err(format!(
                            "ReplayDivergenceError: Step name mismatch at step #{}: expected '{}', got '{}'",
                            self.step_cursor, rec_name, name
                        ));
                    }

                    if rec_input != &input {
                        return Err(format!(
                            "ReplayDivergenceError: Step input divergence at step #{} ('{}'): recorded={:?}, current={:?}",
                            self.step_cursor, name, rec_input, input
                        ));
                    }

                    self.step_cursor = idx + 1;
                    Ok(rec_output.clone())
                } else {
                    unreachable!()
                }
            }
            ReplayMode::Idle => Ok(output),
        }
    }

    pub fn get_stats(&self) -> ReplayStats {
        let (total_events, step_count) = match self.mode {
            ReplayMode::Recording => {
                let steps = self
                    .recorded_events
                    .iter()
                    .filter(|e| matches!(e, TraceEvent::AgentStep { .. }))
                    .count();
                (self.recorded_events.len(), steps)
            }
            ReplayMode::Replaying => {
                let events = self
                    .current_trace
                    .as_ref()
                    .map(|t| &t.events)
                    .unwrap_or(&self.recorded_events);
                let steps = events
                    .iter()
                    .filter(|e| matches!(e, TraceEvent::AgentStep { .. }))
                    .count();
                (events.len(), steps)
            }
            ReplayMode::Idle => (0, 0),
        };

        let mode_str = match self.mode {
            ReplayMode::Idle => "idle",
            ReplayMode::Recording => "recording",
            ReplayMode::Replaying => "replaying",
        };

        ReplayStats {
            mode: mode_str.to_string(),
            is_recording: self.mode == ReplayMode::Recording,
            is_replaying: self.mode == ReplayMode::Replaying,
            total_events,
            step_count,
            current_cursor: self.step_cursor,
            script: self.script_path.clone(),
        }
    }

    pub fn reset(&mut self) {
        self.mode = ReplayMode::Idle;
        self.recorded_events.clear();
        self.current_trace = None;
        self.cursor = 0;
        self.step_cursor = 0;
        self.script_path = None;
        self.output_path = None;
        crate::permissions::reset_runtime_permission_state();
    }
}

pub static GLOBAL_REPLAY: Lazy<RwLock<ReplayEngine>> =
    Lazy::new(|| RwLock::new(ReplayEngine::new()));

fn chrono_lite_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now();
    let epoch = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}-trace", epoch)
}

/// Native dispatcher callback for __amber_replay_native
fn replay_native_dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.length() == 0 || !args.get(0).is_string() {
        rv.set(v8::null(scope).into());
        return;
    }

    let action = args.get(0).to_rust_string_lossy(scope);

    match action.as_str() {
        "start" => {
            let script = if args.length() > 1 && args.get(1).is_string() {
                Some(args.get(1).to_rust_string_lossy(scope))
            } else {
                None
            };
            let output_path = if args.length() > 2 && args.get(2).is_string() {
                Some(args.get(2).to_rust_string_lossy(scope))
            } else {
                None
            };
            if let Ok(mut engine) = GLOBAL_REPLAY.write() {
                engine.start_recording(script, output_path);
                rv.set(v8::Boolean::new(scope, true).into());
            } else {
                rv.set(v8::Boolean::new(scope, false).into());
            }
        }
        "stop" => {
            let output_path = if args.length() > 1 && args.get(1).is_string() {
                Some(args.get(1).to_rust_string_lossy(scope))
            } else {
                None
            };
            let res = {
                if let Ok(mut engine) = GLOBAL_REPLAY.write() {
                    engine.stop_recording(output_path.as_deref())
                } else {
                    Err("Lock error".to_string())
                }
            };
            match res {
                Ok(trace) => {
                    let json = serde_json::to_string(&trace).unwrap_or_else(|_| "{}".to_string());
                    let v8_str = v8::String::new(scope, &json).unwrap();
                    rv.set(v8_str.into());
                }
                Err(err) => {
                    let msg = v8::String::new(scope, &err).unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        }
        "load_file" => {
            if args.length() > 1 && args.get(1).is_string() {
                let path_str = args.get(1).to_rust_string_lossy(scope);
                if let Ok(mut engine) = GLOBAL_REPLAY.write() {
                    match engine.load_trace_from_file(&path_str) {
                        Ok(_) => rv.set(v8::Boolean::new(scope, true).into()),
                        Err(e) => {
                            let msg = v8::String::new(scope, &e).unwrap();
                            let exc = v8::Exception::error(scope, msg);
                            scope.throw_exception(exc);
                        }
                    }
                } else {
                    rv.set(v8::Boolean::new(scope, false).into());
                }
            } else {
                rv.set(v8::Boolean::new(scope, false).into());
            }
        }
        "load_json" => {
            if args.length() > 1 && args.get(1).is_string() {
                let json_str = args.get(1).to_rust_string_lossy(scope);
                match serde_json::from_str::<Trace>(&json_str) {
                    Ok(trace) => {
                        if let Ok(mut engine) = GLOBAL_REPLAY.write() {
                            engine.load_trace(trace);
                            rv.set(v8::Boolean::new(scope, true).into());
                        } else {
                            rv.set(v8::Boolean::new(scope, false).into());
                        }
                    }
                    Err(e) => {
                        let msg =
                            v8::String::new(scope, &format!("Invalid trace JSON: {}", e)).unwrap();
                        let exc = v8::Exception::type_error(scope, msg);
                        scope.throw_exception(exc);
                    }
                }
            } else {
                rv.set(v8::Boolean::new(scope, false).into());
            }
        }
        "step" => {
            let name = if args.length() > 1 && args.get(1).is_string() {
                args.get(1).to_rust_string_lossy(scope)
            } else {
                "step".to_string()
            };
            let input_json = if args.length() > 2 && args.get(2).is_string() {
                args.get(2).to_rust_string_lossy(scope)
            } else {
                "null".to_string()
            };
            let output_json = if args.length() > 3 && args.get(3).is_string() {
                args.get(3).to_rust_string_lossy(scope)
            } else {
                "null".to_string()
            };

            let input_val: serde_json::Value =
                serde_json::from_str(&input_json).unwrap_or(serde_json::Value::Null);
            let output_val: serde_json::Value =
                serde_json::from_str(&output_json).unwrap_or(serde_json::Value::Null);

            let res = {
                if let Ok(mut engine) = GLOBAL_REPLAY.write() {
                    engine.step(&name, input_val, output_val)
                } else {
                    Err("Lock error".to_string())
                }
            };

            match res {
                Ok(out) => {
                    let out_str =
                        serde_json::to_string(&out).unwrap_or_else(|_| "null".to_string());
                    let v8_str = v8::String::new(scope, &out_str).unwrap();
                    rv.set(v8_str.into());
                }
                Err(err) => {
                    let msg = v8::String::new(scope, &err).unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        }
        "get_mode" => {
            let mode = GLOBAL_REPLAY
                .read()
                .map(|e| match e.mode {
                    ReplayMode::Idle => "idle",
                    ReplayMode::Recording => "recording",
                    ReplayMode::Replaying => "replaying",
                })
                .unwrap_or("idle");
            let v8_str = v8::String::new(scope, mode).unwrap();
            rv.set(v8_str.into());
        }
        "is_recording" => {
            let is_rec = GLOBAL_REPLAY
                .read()
                .map(|e| e.mode == ReplayMode::Recording)
                .unwrap_or(false);
            rv.set(v8::Boolean::new(scope, is_rec).into());
        }
        "is_replaying" => {
            let is_rep = GLOBAL_REPLAY
                .read()
                .map(|e| e.mode == ReplayMode::Replaying)
                .unwrap_or(false);
            rv.set(v8::Boolean::new(scope, is_rep).into());
        }
        "stats" => {
            let stats = match GLOBAL_REPLAY.read() {
                Ok(e) => e.get_stats(),
                Err(_) => {
                    rv.set(v8::null(scope).into());
                    return;
                }
            };
            let json = serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string());
            let v8_str = v8::String::new(scope, &json).unwrap();
            rv.set(v8_str.into());
        }
        "reset" => {
            if let Ok(mut engine) = GLOBAL_REPLAY.write() {
                engine.reset();
                rv.set(v8::Boolean::new(scope, true).into());
            } else {
                rv.set(v8::Boolean::new(scope, false).into());
            }
        }
        _ => {
            rv.set(v8::null(scope).into());
        }
    }
}

/// Sets up the `amber:replay` API inside V8 Context
pub fn setup_replay_api(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> anyhow::Result<()> {
    let global = context.global(scope);

    // Register native dispatcher
    let native_fn = v8::Function::new(scope, replay_native_dispatch).unwrap();
    let k_native = v8::String::new(scope, "__amber_replay_native").unwrap();
    global.set(scope, k_native.into(), native_fn.into());

    let replay_js_bootstrap = r#"
    (function() {
        const native = globalThis.__amber_replay_native;
        const replay = {
            startRecording(opts = {}) {
                const script = typeof opts === 'string' ? opts : (opts && opts.script);
                const outputPath = typeof opts === 'object' ? (opts && opts.outputPath) : undefined;
                return native('start', script, outputPath);
            },
            stopRecording(outputPath) {
                const jsonStr = native('stop', outputPath);
                return JSON.parse(jsonStr);
            },
            loadTrace(traceOrPath) {
                if (typeof traceOrPath === 'string') {
                    return native('load_file', traceOrPath);
                } else {
                    return native('load_json', JSON.stringify(traceOrPath));
                }
            },
            step(name, input, outputOrFn) {
                const mode = native('get_mode');
                if (mode === 'replaying') {
                    const resJson = native('step', name, JSON.stringify(input), 'null');
                    return JSON.parse(resJson);
                }
                let outputVal = outputOrFn;
                if (typeof outputOrFn === 'function') {
                    outputVal = outputOrFn(input);
                }
                if (mode === 'recording') {
                    native('step', name, JSON.stringify(input), JSON.stringify(outputVal));
                }
                return outputVal;
            },
            isRecording() {
                return native('is_recording');
            },
            isReplaying() {
                return native('is_replaying');
            },
            getTraceStats() {
                return JSON.parse(native('stats'));
            },
            reset() {
                return native('reset');
            }
        };
        globalThis.__amber_replay = replay;
        globalThis.replay = replay;
    })();
    "#;

    if let Some(code) = v8::String::new(scope, replay_js_bootstrap) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}
