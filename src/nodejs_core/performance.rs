// v0.3.275: Node.js performance API implementation
// Provides high-resolution timing for AI workloads and performance monitoring
// Implements Web Performance API compatible with Node.js/Bun

use once_cell::sync::Lazy;
use rusty_v8 as v8;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

/// Performance entry type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerformanceEntryType {
    Mark,
    Measure,
    Histogram,
}

/// Performance entry for storing timing data
#[derive(Debug, Clone)]
pub struct PerformanceEntry {
    pub name: String,
    pub entry_type: PerformanceEntryType,
    pub start_time: f64,
    pub duration: f64,
    pub detail: Option<String>,
}

/// Performance mark result
#[derive(Debug, Clone)]
pub struct PerformanceMark {
    pub name: String,
    pub start_time: f64,
}

/// Performance measurement result
#[derive(Debug, Clone)]
pub struct PerformanceMeasure {
    pub name: String,
    pub start_time: f64,
    pub duration: f64,
}

/// Global performance state
struct PerformanceState {
    /// Performance entries
    entries: Vec<PerformanceEntry>,
    /// Marks by name
    marks: Vec<PerformanceMark>,
    /// Next entry ID
    #[allow(dead_code)]
    next_id: u64,
}

impl PerformanceState {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            marks: Vec::new(),
            next_id: 1,
        }
    }

    fn add_entry(&mut self, entry: PerformanceEntry) {
        self.entries.push(entry);
    }

    fn add_mark(&mut self, mark: PerformanceMark) {
        self.marks.push(mark);
    }

    fn get_mark(&self, name: &str) -> Option<f64> {
        self.marks
            .iter()
            .filter(|m| m.name == name)
            .max_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap())
            .map(|m| m.start_time)
    }

    fn clear_entries(&mut self) {
        self.entries.clear();
        self.marks.clear();
    }
}

struct PerformanceClock {
    time_origin_ms: f64,
    start_instant: Instant,
}

impl PerformanceClock {
    fn new() -> Self {
        Self {
            time_origin_ms: system_epoch_millis(),
            start_instant: Instant::now(),
        }
    }
}

/// Global performance state (thread-safe for V8 main thread access)
static PERFORMANCE_STATE: Lazy<Mutex<PerformanceState>> =
    Lazy::new(|| Mutex::new(PerformanceState::new()));

/// Process-wide high-resolution clock for performance timeline values.
static PERFORMANCE_CLOCK: Lazy<PerformanceClock> = Lazy::new(PerformanceClock::new);

/// High-resolution time origin (time since system boot for macOS, or Unix epoch for others)
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn get_time_origin() -> Instant {
    // On macOS, use std::time::UNIX_EPOCH and calculate boot time
    // For simplicity, we use the Unix epoch as reference
    Instant::now()
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn get_time_origin() -> Instant {
    Instant::now()
}

fn system_epoch_millis() -> f64 {
    if let Some(frozen) = crate::permissions::get_frozen_time_ms() {
        return frozen as f64;
    }
    let now = SystemTime::now();
    let duration = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    duration.as_secs_f64() * 1000.0
}

/// Get high-resolution time in milliseconds (since time origin)
fn get_high_res_time() -> f64 {
    if crate::permissions::get_frozen_time_ms().is_some() {
        return 0.0;
    }
    PERFORMANCE_CLOCK.start_instant.elapsed().as_secs_f64() * 1000.0
}

/// Get high-resolution time in microseconds
#[allow(dead_code)]
fn get_high_res_time_us() -> f64 {
    get_high_res_time() * 1000.0
}

/// Get the time origin timestamp in milliseconds
fn get_time_origin_ms() -> f64 {
    PERFORMANCE_CLOCK.time_origin_ms
}

/// Setup performance API in the V8 context
pub fn setup_performance_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<(), anyhow::Error> {
    let global = context.global(scope);

    // performance.now() - returns a high-resolution timestamp in milliseconds
    let now_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let now = get_high_res_time();
            retval.set(v8::Number::new(scope, now).into());
        },
    )
    .unwrap();

    // performance.mark(name) - creates a performance mark
    let mark_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            if args.length() < 1 {
                let error = v8::String::new(scope, "performance.mark requires at least 1 argument")
                    .unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let name = args
                .get(0)
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope);

            if name.is_empty() {
                let error =
                    v8::String::new(scope, "performance.mark name cannot be empty").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let start_time = get_high_res_time();

            let mut state = PERFORMANCE_STATE.lock().unwrap();
            state.add_mark(PerformanceMark {
                name: name.clone(),
                start_time,
            });
            state.add_entry(PerformanceEntry {
                name,
                entry_type: PerformanceEntryType::Mark,
                start_time,
                duration: 0.0,
                detail: None,
            });
        },
    )
    .unwrap();

    // performance.measure(name, startMark, endMark) - creates a performance measure
    let measure_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            if args.length() < 1 {
                let error =
                    v8::String::new(scope, "performance.measure requires at least 1 argument")
                        .unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let name = args
                .get(0)
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope);

            let start_time = if args.length() >= 2 {
                let start_mark = args
                    .get(1)
                    .to_string(scope)
                    .unwrap()
                    .to_rust_string_lossy(scope);
                PERFORMANCE_STATE
                    .lock()
                    .unwrap()
                    .get_mark(&start_mark)
                    .unwrap_or(0.0)
            } else {
                0.0
            };

            let end_time = if args.length() >= 3 {
                let end_mark = args
                    .get(2)
                    .to_string(scope)
                    .unwrap()
                    .to_rust_string_lossy(scope);
                PERFORMANCE_STATE
                    .lock()
                    .unwrap()
                    .get_mark(&end_mark)
                    .unwrap_or(get_high_res_time())
            } else {
                get_high_res_time()
            };

            let duration = end_time - start_time;

            let mut state = PERFORMANCE_STATE.lock().unwrap();
            state.add_entry(PerformanceEntry {
                name: name.clone(),
                entry_type: PerformanceEntryType::Measure,
                start_time,
                duration,
                detail: None,
            });
        },
    )
    .unwrap();

    // performance.clearMarks() - removes all marks
    let clear_marks_fn = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            let mut state = PERFORMANCE_STATE.lock().unwrap();
            state.marks.clear();
            state
                .entries
                .retain(|e| e.entry_type != PerformanceEntryType::Mark);
        },
    )
    .unwrap();

    // performance.clearMeasures() - removes all measures
    let clear_measures_fn = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            let mut state = PERFORMANCE_STATE.lock().unwrap();
            state
                .entries
                .retain(|e| e.entry_type != PerformanceEntryType::Measure);
        },
    )
    .unwrap();

    // performance.clearAllMarks() - removes all marks (alias for clearMarks)
    let clear_all_marks_fn = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            let mut state = PERFORMANCE_STATE.lock().unwrap();
            state.marks.clear();
            state
                .entries
                .retain(|e| e.entry_type != PerformanceEntryType::Mark);
        },
    )
    .unwrap();

    // performance.getEntries() - returns all performance entries
    let get_entries_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let state = PERFORMANCE_STATE.lock().unwrap();
            let entries: Vec<v8::Local<v8::Value>> = state
                .entries
                .iter()
                .map(|e| {
                    let obj = v8::Object::new(scope);

                    let name_key = v8::String::new(scope, "name").unwrap();
                    let name_val = v8::String::new(scope, &e.name).unwrap();
                    obj.set(scope, name_key.into(), name_val.into());

                    let entry_type_key = v8::String::new(scope, "entryType").unwrap();
                    let entry_type_val = v8::String::new(
                        scope,
                        match e.entry_type {
                            PerformanceEntryType::Mark => "mark",
                            PerformanceEntryType::Measure => "measure",
                            PerformanceEntryType::Histogram => "histogram",
                        },
                    )
                    .unwrap();
                    obj.set(scope, entry_type_key.into(), entry_type_val.into());

                    let start_time_key = v8::String::new(scope, "startTime").unwrap();
                    let start_time_val = v8::Number::new(scope, e.start_time);
                    obj.set(scope, start_time_key.into(), start_time_val.into());

                    let duration_key = v8::String::new(scope, "duration").unwrap();
                    let duration_val = v8::Number::new(scope, e.duration);
                    obj.set(scope, duration_key.into(), duration_val.into());

                    obj.into()
                })
                .collect();

            let array = v8::Array::new_with_elements(scope, &entries);
            retval.set(array.into());
        },
    )
    .unwrap();

    // performance.getEntriesByName(name) - returns entries matching the name
    let get_entries_by_name_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            if args.length() < 1 {
                let array = v8::Array::new(scope, 0);
                retval.set(array.into());
                return;
            }

            let name = args
                .get(0)
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope);

            let state = PERFORMANCE_STATE.lock().unwrap();
            let entries: Vec<v8::Local<v8::Value>> = state
                .entries
                .iter()
                .filter(|e| e.name == name)
                .map(|e| {
                    let obj = v8::Object::new(scope);

                    let name_key = v8::String::new(scope, "name").unwrap();
                    let name_val = v8::String::new(scope, &e.name).unwrap();
                    obj.set(scope, name_key.into(), name_val.into());

                    let entry_type_key = v8::String::new(scope, "entryType").unwrap();
                    let entry_type_val = v8::String::new(
                        scope,
                        match e.entry_type {
                            PerformanceEntryType::Mark => "mark",
                            PerformanceEntryType::Measure => "measure",
                            PerformanceEntryType::Histogram => "histogram",
                        },
                    )
                    .unwrap();
                    obj.set(scope, entry_type_key.into(), entry_type_val.into());

                    let start_time_key = v8::String::new(scope, "startTime").unwrap();
                    let start_time_val = v8::Number::new(scope, e.start_time);
                    obj.set(scope, start_time_key.into(), start_time_val.into());

                    let duration_key = v8::String::new(scope, "duration").unwrap();
                    let duration_val = v8::Number::new(scope, e.duration);
                    obj.set(scope, duration_key.into(), duration_val.into());

                    obj.into()
                })
                .collect();

            let array = v8::Array::new_with_elements(scope, &entries);
            retval.set(array.into());
        },
    )
    .unwrap();

    // performance.getEntriesByType(type) - returns entries of the specified type
    let get_entries_by_type_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            if args.length() < 1 {
                let array = v8::Array::new(scope, 0);
                retval.set(array.into());
                return;
            }

            let entry_type = args
                .get(0)
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope);

            let state = PERFORMANCE_STATE.lock().unwrap();
            let entries: Vec<v8::Local<v8::Value>> = state
                .entries
                .iter()
                .filter(|e| {
                    let e_type_str = match e.entry_type {
                        PerformanceEntryType::Mark => "mark",
                        PerformanceEntryType::Measure => "measure",
                        PerformanceEntryType::Histogram => "histogram",
                    };
                    e_type_str == entry_type
                })
                .map(|e| {
                    let obj = v8::Object::new(scope);

                    let name_key = v8::String::new(scope, "name").unwrap();
                    let name_val = v8::String::new(scope, &e.name).unwrap();
                    obj.set(scope, name_key.into(), name_val.into());

                    let type_key = v8::String::new(scope, "entryType").unwrap();
                    let type_val = v8::String::new(
                        scope,
                        match e.entry_type {
                            PerformanceEntryType::Mark => "mark",
                            PerformanceEntryType::Measure => "measure",
                            PerformanceEntryType::Histogram => "histogram",
                        },
                    )
                    .unwrap();
                    obj.set(scope, type_key.into(), type_val.into());

                    let start_time_key = v8::String::new(scope, "startTime").unwrap();
                    let start_time_val = v8::Number::new(scope, e.start_time);
                    obj.set(scope, start_time_key.into(), start_time_val.into());

                    let duration_key = v8::String::new(scope, "duration").unwrap();
                    let duration_val = v8::Number::new(scope, e.duration);
                    obj.set(scope, duration_key.into(), duration_val.into());

                    obj.into()
                })
                .collect();

            let array = v8::Array::new_with_elements(scope, &entries);
            retval.set(array.into());
        },
    )
    .unwrap();

    // performance.toJSON() - returns a JSON representation
    let to_json_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let obj = v8::Object::new(scope);

            let now_key = v8::String::new(scope, "now").unwrap();
            let now_val = v8::Number::new(scope, get_high_res_time());
            obj.set(scope, now_key.into(), now_val.into());

            let time_origin_key = v8::String::new(scope, "timeOrigin").unwrap();
            let time_origin_val = v8::Number::new(scope, get_time_origin_ms());
            obj.set(scope, time_origin_key.into(), time_origin_val.into());

            retval.set(obj.into());
        },
    )
    .unwrap();

    // Create the performance object
    let perf_obj = v8::Object::new(scope);

    // Set methods
    let now_key = v8::String::new(scope, "now").unwrap();
    perf_obj.set(scope, now_key.into(), now_fn.into());

    let time_origin_key = v8::String::new(scope, "timeOrigin").unwrap();
    let time_origin_value = v8::Number::new(scope, get_time_origin_ms());
    perf_obj.set(scope, time_origin_key.into(), time_origin_value.into());

    let mark_key = v8::String::new(scope, "mark").unwrap();
    perf_obj.set(scope, mark_key.into(), mark_fn.into());

    let measure_key = v8::String::new(scope, "measure").unwrap();
    perf_obj.set(scope, measure_key.into(), measure_fn.into());

    let clear_marks_key = v8::String::new(scope, "clearMarks").unwrap();
    perf_obj.set(scope, clear_marks_key.into(), clear_marks_fn.into());

    let clear_measures_key = v8::String::new(scope, "clearMeasures").unwrap();
    perf_obj.set(scope, clear_measures_key.into(), clear_measures_fn.into());

    let clear_all_marks_key = v8::String::new(scope, "clearAllMarks").unwrap();
    perf_obj.set(scope, clear_all_marks_key.into(), clear_all_marks_fn.into());

    let get_entries_key = v8::String::new(scope, "getEntries").unwrap();
    perf_obj.set(scope, get_entries_key.into(), get_entries_fn.into());

    let get_entries_by_name_key = v8::String::new(scope, "getEntriesByName").unwrap();
    perf_obj.set(
        scope,
        get_entries_by_name_key.into(),
        get_entries_by_name_fn.into(),
    );

    let get_entries_by_type_key = v8::String::new(scope, "getEntriesByType").unwrap();
    perf_obj.set(
        scope,
        get_entries_by_type_key.into(),
        get_entries_by_type_fn.into(),
    );

    let to_json_key = v8::String::new(scope, "toJSON").unwrap();
    perf_obj.set(scope, to_json_key.into(), to_json_fn.into());

    // Set as global property
    let perf_key = v8::String::new(scope, "performance").unwrap();
    global.set(scope, perf_key.into(), perf_obj.into());

    Ok(())
}

/// Clear all performance entries (for testing)
pub fn clear_performance_entries() {
    let mut state = PERFORMANCE_STATE.lock().unwrap();
    state.clear_entries();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_state_creation() {
        let state = PerformanceState::new();
        assert!(state.entries.is_empty());
        assert!(state.marks.is_empty());
    }

    #[test]
    fn test_performance_mark() {
        let mut state = PerformanceState::new();
        let mark = PerformanceMark {
            name: "test".to_string(),
            start_time: 100.0,
        };
        state.add_mark(mark.clone());
        assert_eq!(state.marks.len(), 1);
        assert_eq!(state.get_mark("test"), Some(100.0));
        assert_eq!(state.get_mark("nonexistent"), None);
    }

    #[test]
    fn test_performance_measure() {
        let mut state = PerformanceState::new();
        state.add_entry(PerformanceEntry {
            name: "measure1".to_string(),
            entry_type: PerformanceEntryType::Measure,
            start_time: 100.0,
            duration: 50.0,
            detail: None,
        });
        assert_eq!(state.entries.len(), 1);
    }

    #[test]
    fn test_high_res_time() {
        let time_ms = get_high_res_time();
        let time_us = get_high_res_time_us();
        assert!(time_ms >= 0.0);
        assert!(time_us >= 0.0);
        // Microseconds should be 1000x more precise
        assert!(time_us > time_ms * 1000.0 - 1.0);
    }

    #[test]
    fn test_time_origin_ms() {
        let origin = get_time_origin_ms();
        assert!(origin > 0.0);
        // Should be a reasonable Unix timestamp in milliseconds
        assert!(origin > 1700000000000.0); // After 2023
    }
}
