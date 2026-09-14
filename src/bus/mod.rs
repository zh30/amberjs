// Beejs Multi-Agent Message Bus & PubSub Channel Fabric (bee:bus)
// High-throughput in-process message routing, topic wildcard dispatch, request-reply semantics, and DLQ.

use once_cell::sync::Lazy;
use rusty_v8 as v8;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

static BUS_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
static MSG_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub topic: String,
    pub payload: serde_json::Value,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub timestamp: u64,
    #[serde(default)]
    pub priority: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BusMetrics {
    pub published_count: u64,
    pub delivered_count: u64,
    pub dead_letter_count: u64,
    pub active_subscriptions: usize,
}

pub struct MessageBus {
    pub id: u64,
    pub dead_letters: Vec<Message>,
    pub max_dead_letters: usize,
    pub metrics: BusMetrics,
    pub registered_topics: HashMap<String, u64>,
}

impl MessageBus {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            dead_letters: Vec::new(),
            max_dead_letters: 1000,
            metrics: BusMetrics::default(),
            registered_topics: HashMap::new(),
        }
    }

    pub fn record_publish(&mut self, topic: &str, delivered: usize) {
        self.metrics.published_count += 1;
        self.metrics.delivered_count += delivered as u64;
        let count = self.registered_topics.entry(topic.to_string()).or_insert(0);
        *count += 1;
    }

    pub fn add_dead_letter(&mut self, msg: Message) {
        self.metrics.dead_letter_count += 1;
        if self.dead_letters.len() >= self.max_dead_letters {
            self.dead_letters.remove(0);
        }
        self.dead_letters.push(msg);
    }
}

static BUSES: Lazy<RwLock<HashMap<u64, MessageBus>>> = Lazy::new(|| RwLock::new(HashMap::new()));

/// Matches a topic pattern with a concrete topic name.
/// Supports `*` for single token and `#` for zero or more tokens.
/// Tokens are separated by dots (e.g. `agent.planner.response`).
pub fn topic_matches(pattern: &str, topic: &str) -> bool {
    if pattern == "#" || pattern == topic {
        return true;
    }

    let p_tokens: Vec<&str> = pattern.split('.').collect();
    let t_tokens: Vec<&str> = topic.split('.').collect();

    fn match_recursive(p: &[&str], t: &[&str]) -> bool {
        if p.is_empty() {
            return t.is_empty();
        }

        if p[0] == "#" {
            // # can match 0 or more tokens
            if p.len() == 1 {
                return true;
            }
            for i in 0..=t.len() {
                if match_recursive(&p[1..], &t[i..]) {
                    return true;
                }
            }
            return false;
        }

        if t.is_empty() {
            return false;
        }

        if p[0] == "*" || p[0] == t[0] {
            return match_recursive(&p[1..], &t[1..]);
        }

        false
    }

    match_recursive(&p_tokens, &t_tokens)
}

fn bus_native_dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.length() < 1 {
        return;
    }

    let action = args.get(0).to_rust_string_lossy(scope);

    match action.as_str() {
        "create_bus" => {
            let id = BUS_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
            let bus = MessageBus::new(id);
            let mut map = BUSES.write().unwrap();
            map.insert(id, bus);
            rv.set(v8::Integer::new(scope, id as i32).into());
        }
        "next_message_id" => {
            let id = MSG_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
            let id_str = format!("msg_{:x}", id);
            let s = v8::String::new(scope, &id_str).unwrap();
            rv.set(s.into());
        }
        "topic_matches" => {
            let pattern = args.get(1).to_rust_string_lossy(scope);
            let topic = args.get(2).to_rust_string_lossy(scope);
            let is_match = topic_matches(&pattern, &topic);
            rv.set(v8::Boolean::new(scope, is_match).into());
        }
        "record_publish" => {
            let bus_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let topic = args.get(2).to_rust_string_lossy(scope);
            let delivered = args
                .get(3)
                .to_integer(scope)
                .map(|i| i.value() as usize)
                .unwrap_or(0);
            let mut map = BUSES.write().unwrap();
            if let Some(bus) = map.get_mut(&bus_id) {
                bus.record_publish(&topic, delivered);
            }
        }
        "add_dead_letter" => {
            let bus_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let msg_json = args.get(2).to_rust_string_lossy(scope);
            if let Ok(msg) = serde_json::from_str::<Message>(&msg_json) {
                let mut map = BUSES.write().unwrap();
                if let Some(bus) = map.get_mut(&bus_id) {
                    bus.add_dead_letter(msg);
                }
            }
        }
        "get_dead_letters" => {
            let bus_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let map = BUSES.read().unwrap();
            let json = if let Some(bus) = map.get(&bus_id) {
                serde_json::to_string(&bus.dead_letters).unwrap_or_else(|_| "[]".to_string())
            } else {
                "[]".to_string()
            };
            let s = v8::String::new(scope, &json).unwrap();
            rv.set(s.into());
        }
        "clear_dead_letters" => {
            let bus_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let mut map = BUSES.write().unwrap();
            if let Some(bus) = map.get_mut(&bus_id) {
                bus.dead_letters.clear();
                bus.metrics.dead_letter_count = 0;
            }
            rv.set(v8::Boolean::new(scope, true).into());
        }
        "get_metrics" => {
            let bus_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let map = BUSES.read().unwrap();
            let json = if let Some(bus) = map.get(&bus_id) {
                serde_json::to_string(&bus.metrics).unwrap_or_else(|_| "{}".to_string())
            } else {
                "{}".to_string()
            };
            let s = v8::String::new(scope, &json).unwrap();
            rv.set(s.into());
        }
        "get_topics" => {
            let bus_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let map = BUSES.read().unwrap();
            let json = if let Some(bus) = map.get(&bus_id) {
                let topics: Vec<&String> = bus.registered_topics.keys().collect();
                serde_json::to_string(&topics).unwrap_or_else(|_| "[]".to_string())
            } else {
                "[]".to_string()
            };
            let s = v8::String::new(scope, &json).unwrap();
            rv.set(s.into());
        }
        _ => {
            let msg = v8::String::new(scope, &format!("Unknown bus action '{action}'")).unwrap();
            let exc = v8::Exception::type_error(scope, msg);
            scope.throw_exception(exc);
        }
    }
}

/// Sets up the `bee:bus` API in V8 context
pub fn setup_bus_api(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> anyhow::Result<()> {
    let global = context.global(scope);

    // Register native dispatcher callback
    let native_fn = v8::Function::new(scope, bus_native_dispatch).unwrap();
    let k_native = v8::String::new(scope, "__bee_bus_native").unwrap();
    global.set(scope, k_native.into(), native_fn.into());

    let bus_js_bootstrap = r#"
    (function() {
        const native = globalThis.__bee_bus_native;

        class MessageBus {
            #id;
            #subscriptions = new Map(); // subId -> { id, pattern, handler, priority, once }
            #middlewares = [];
            #subIdCounter = 1;

            constructor(id) {
                this.#id = typeof id === 'number' ? id : native('create_bus');
            }

            get id() {
                return this.#id;
            }

            subscribe(pattern, handler, options = {}) {
                if (typeof pattern !== 'string' || !pattern.trim()) {
                    throw new TypeError('subscribe requires a non-empty pattern string');
                }
                if (typeof handler !== 'function') {
                    throw new TypeError('subscribe requires a function handler');
                }

                const subId = `sub_${this.#subIdCounter++}`;
                const priority = typeof options.priority === 'number' ? options.priority : 0;
                const once = Boolean(options.once);

                const sub = {
                    id: subId,
                    pattern: pattern.trim(),
                    handler,
                    priority,
                    once,
                    unsubscribe: () => {
                        this.unsubscribe(subId);
                    }
                };

                this.#subscriptions.set(subId, sub);
                return sub;
            }

            once(pattern, handler, options = {}) {
                return this.subscribe(pattern, handler, { ...options, once: true });
            }

            unsubscribe(subId) {
                if (typeof subId === 'object' && subId && subId.id) {
                    subId = subId.id;
                }
                return this.#subscriptions.delete(String(subId));
            }

            use(middleware) {
                if (typeof middleware !== 'function') {
                    throw new TypeError('use requires a middleware function');
                }
                this.#middlewares.push(middleware);
                return this;
            }

            publish(topic, payload, options = {}) {
                if (typeof topic !== 'string' || !topic.trim()) {
                    throw new TypeError('publish requires a non-empty topic string');
                }

                const msgId = options.id || native('next_message_id');
                const message = {
                    id: msgId,
                    topic: topic.trim(),
                    payload,
                    headers: options.headers || {},
                    timestamp: Date.now(),
                    priority: typeof options.priority === 'number' ? options.priority : 0,
                    replyTo: options.replyTo || undefined,
                    correlationId: options.correlationId || undefined,
                    reply: (replyPayload) => this.reply(message, replyPayload)
                };

                // Execute middlewares
                let cancelled = false;
                for (const mw of this.#middlewares) {
                    try {
                        const res = mw(message, () => {});
                        if (res === false) {
                            cancelled = true;
                            break;
                        }
                    } catch (e) {
                        console.error('[MessageBus Middleware Error]', e);
                    }
                }
                if (cancelled) return message;

                // Find matching subscribers
                const matchingSubs = [];
                for (const sub of this.#subscriptions.values()) {
                    if (native('topic_matches', sub.pattern, message.topic)) {
                        matchingSubs.push(sub);
                    }
                }

                // Sort by priority descending
                matchingSubs.sort((a, b) => b.priority - a.priority);

                if (matchingSubs.length === 0) {
                    // Dead letter
                    native('add_dead_letter', this.#id, JSON.stringify(message));
                    native('record_publish', this.#id, message.topic, 0);
                    return message;
                }

                native('record_publish', this.#id, message.topic, matchingSubs.length);

                // Dispatch to handlers
                for (const sub of matchingSubs) {
                    try {
                        sub.handler(message);
                    } catch (err) {
                        console.error(`[MessageBus Handler Error on '${message.topic}']`, err);
                    }
                    if (sub.once) {
                        this.#subscriptions.delete(sub.id);
                    }
                }

                return message;
            }

            broadcast(topic, payload, options) {
                return this.publish(topic, payload, options);
            }

            async request(topic, payload, options = {}) {
                const timeoutMs = typeof options.timeoutMs === 'number' ? options.timeoutMs : 5000;
                const correlationId = `req_${Math.random().toString(36).slice(2)}_${Date.now()}`;
                const replyTopic = `_reply.${correlationId}`;

                return new Promise((resolve, reject) => {
                    let timer = null;

                    const sub = this.subscribe(replyTopic, (replyMsg) => {
                        if (timer) clearTimeout(timer);
                        sub.unsubscribe();
                        resolve(replyMsg.payload);
                    }, { once: true });

                    if (timeoutMs > 0) {
                        timer = setTimeout(() => {
                            sub.unsubscribe();
                            reject(new Error(`MessageBus request to '${topic}' timed out after ${timeoutMs}ms (correlationId: ${correlationId})`));
                        }, timeoutMs);
                    }

                    this.publish(topic, payload, {
                        ...options,
                        replyTo: replyTopic,
                        correlationId
                    });
                });
            }

            reply(originalMessage, responsePayload) {
                if (!originalMessage || !originalMessage.replyTo) {
                    throw new Error('Cannot reply to a message without replyTo address');
                }
                return this.publish(originalMessage.replyTo, responsePayload, {
                    correlationId: originalMessage.correlationId
                });
            }

            getMetrics() {
                const raw = native('get_metrics', this.#id);
                const m = JSON.parse(raw);
                m.activeSubscriptions = this.#subscriptions.size;
                return m;
            }

            getDeadLetters() {
                const raw = native('get_dead_letters', this.#id);
                return JSON.parse(raw);
            }

            clearDeadLetters() {
                return native('clear_dead_letters', this.#id);
            }

            getTopics() {
                const raw = native('get_topics', this.#id);
                return JSON.parse(raw);
            }

            clear() {
                this.#subscriptions.clear();
                this.#middlewares = [];
                this.clearDeadLetters();
            }
        }

        const defaultBus = new MessageBus();

        const busModule = {
            MessageBus,
            createBus: (id) => new MessageBus(id),
            getDefaultBus: () => defaultBus,
            subscribe: (...args) => defaultBus.subscribe(...args),
            once: (...args) => defaultBus.once(...args),
            unsubscribe: (...args) => defaultBus.unsubscribe(...args),
            publish: (...args) => defaultBus.publish(...args),
            broadcast: (...args) => defaultBus.broadcast(...args),
            request: (...args) => defaultBus.request(...args),
            reply: (...args) => defaultBus.reply(...args),
            use: (...args) => defaultBus.use(...args),
            getMetrics: () => defaultBus.getMetrics(),
            getDeadLetters: () => defaultBus.getDeadLetters(),
            clearDeadLetters: () => defaultBus.clearDeadLetters(),
            getTopics: () => defaultBus.getTopics(),
            topicMatches: (pattern, topic) => native('topic_matches', pattern, topic),
            default: defaultBus
        };

        globalThis.__bee_bus = busModule;
        globalThis.bus = busModule;
    })();
    "#;

    if let Some(code) = v8::String::new(scope, bus_js_bootstrap) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}
