use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::sync::RwLock;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PermissionKind {
    FileSystem,
    Environment,
    Network,
    Process,
}

impl PermissionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileSystem => "FileSystem",
            Self::Environment => "Environment",
            Self::Network => "Network",
            Self::Process => "Process",
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PermissionAction {
    Read,
    Write,
    Execute,
    Connect,
    Listen,
}

impl PermissionAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "Read",
            Self::Write => "Write",
            Self::Execute => "Execute",
            Self::Connect => "Connect",
            Self::Listen => "Listen",
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ResourceId {
    Any,
    Path(PathBuf),
    Name(String),
    Url(String),
}

impl ResourceId {
    pub fn display_for_audit(&self) -> String {
        match self {
            Self::Any => "*".to_string(),
            Self::Path(path) => path.display().to_string(),
            Self::Name(name) => name.clone(),
            Self::Url(url) => url.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionDecision {
    Allow,
    Deny,
}

impl PermissionDecision {
    pub fn is_allowed(self) -> bool {
        matches!(self, Self::Allow)
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "Allow",
            Self::Deny => "Deny",
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PermissionRule {
    pub kind: PermissionKind,
    pub action: PermissionAction,
    pub resource: ResourceId,
}

impl PermissionRule {
    pub fn kind(&self) -> &PermissionKind {
        &self.kind
    }
    pub fn action(&self) -> &PermissionAction {
        &self.action
    }
    pub fn resource(&self) -> &ResourceId {
        &self.resource
    }
}

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("permission denied: {kind:?} {action:?} {resource:?}")]
    Denied {
        kind: PermissionKind,
        action: PermissionAction,
        resource: ResourceId,
    },
}

#[derive(Clone, Debug, Default)]
pub struct ResourceBroker {
    allow_rules: HashSet<PermissionRule>,
    deny_rules: HashSet<PermissionRule>,
}

impl ResourceBroker {
    pub fn allow_rules(&self) -> &HashSet<PermissionRule> {
        &self.allow_rules
    }

    pub fn deny_rules(&self) -> &HashSet<PermissionRule> {
        &self.deny_rules
    }

    pub fn allow(&mut self, kind: PermissionKind, action: PermissionAction, resource: ResourceId) {
        let rule = PermissionRule {
            kind,
            action,
            resource: normalize_resource(resource),
        };
        self.deny_rules.remove(&rule);
        self.allow_rules.insert(rule);
    }

    pub fn deny(&mut self, kind: PermissionKind, action: PermissionAction, resource: ResourceId) {
        let rule = PermissionRule {
            kind,
            action,
            resource: normalize_resource(resource),
        };
        self.allow_rules.remove(&rule);
        self.deny_rules.insert(rule);
        HAS_RESTRICTIONS.store(true, Ordering::SeqCst);
    }

    /// Deny every kind/action pair. Used by `--sandbox` before allow overlays.
    pub fn deny_all(&mut self) {
        self.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Any,
        );
        self.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Any,
        );
        self.deny(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Any,
        );
        self.deny(
            PermissionKind::Network,
            PermissionAction::Listen,
            ResourceId::Any,
        );
        self.deny(
            PermissionKind::Environment,
            PermissionAction::Read,
            ResourceId::Any,
        );
        self.deny(
            PermissionKind::Process,
            PermissionAction::Execute,
            ResourceId::Any,
        );
    }

    pub fn check(
        &self,
        kind: PermissionKind,
        action: PermissionAction,
        resource: ResourceId,
    ) -> PermissionDecision {
        let normalized_resource = normalize_resource(resource);
        let decision = self.decide(kind.clone(), action.clone(), &normalized_resource);
        write_audit_record(&kind, &action, &normalized_resource, decision);
        decision
    }

    fn decide(
        &self,
        kind: PermissionKind,
        action: PermissionAction,
        normalized_resource: &ResourceId,
    ) -> PermissionDecision {
        let exact = PermissionRule {
            kind: kind.clone(),
            action: action.clone(),
            resource: normalized_resource.clone(),
        };
        let wildcard = PermissionRule {
            kind: kind.clone(),
            action: action.clone(),
            resource: ResourceId::Any,
        };

        if self.deny_rules.contains(&exact)
            || path_rule_matches(&self.deny_rules, &kind, &action, normalized_resource)
        {
            return PermissionDecision::Deny;
        }
        if self.allow_rules.contains(&exact) {
            return PermissionDecision::Allow;
        }

        if matches!(
            (&kind, &action, normalized_resource),
            (
                PermissionKind::Network,
                PermissionAction::Connect | PermissionAction::Listen,
                ResourceId::Url(_)
            )
        ) {
            if let Some(host_rule) =
                network_host_rule(kind.clone(), action.clone(), normalized_resource)
            {
                if self.deny_rules.contains(&host_rule) {
                    return PermissionDecision::Deny;
                }
                if self.allow_rules.contains(&host_rule) {
                    return PermissionDecision::Allow;
                }
            }
        }

        if path_rule_matches(&self.allow_rules, &kind, &action, normalized_resource) {
            return PermissionDecision::Allow;
        }

        if self.deny_rules.contains(&wildcard) {
            return PermissionDecision::Deny;
        }
        if self.allow_rules.contains(&wildcard) {
            return PermissionDecision::Allow;
        }

        PermissionDecision::Allow
    }

    pub fn check_result(
        &self,
        kind: PermissionKind,
        action: PermissionAction,
        resource: ResourceId,
    ) -> Result<(), PermissionError> {
        if self
            .check(kind.clone(), action.clone(), resource.clone())
            .is_allowed()
        {
            Ok(())
        } else {
            Err(PermissionError::Denied {
                kind,
                action,
                resource,
            })
        }
    }
}

fn path_rule_matches(
    rules: &HashSet<PermissionRule>,
    kind: &PermissionKind,
    action: &PermissionAction,
    resource: &ResourceId,
) -> bool {
    let ResourceId::Path(request) = resource else {
        return false;
    };
    rules.iter().any(|rule| {
        rule.kind == *kind
            && rule.action == *action
            && match &rule.resource {
                ResourceId::Path(allowed) => path_is_under(request, allowed),
                _ => false,
            }
    })
}

fn path_is_under(request: &Path, prefix: &Path) -> bool {
    request == prefix || request.starts_with(prefix)
}

fn network_host_rule(
    kind: PermissionKind,
    action: PermissionAction,
    resource: &ResourceId,
) -> Option<PermissionRule> {
    let ResourceId::Url(url) = resource else {
        return None;
    };
    let host = url::Url::parse(url).ok()?.host_str()?.to_string();
    Some(PermissionRule {
        kind,
        action,
        resource: ResourceId::Name(host),
    })
}

fn normalize_resource(resource: ResourceId) -> ResourceId {
    match resource {
        ResourceId::Path(path) => ResourceId::Path(normalize_path(path)),
        other => other,
    }
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(&path))
            .unwrap_or(path)
    };

    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }

    let Some(parent) = path.parent() else {
        return path;
    };
    let Some(file_name) = path.file_name().map(|name| name.to_os_string()) else {
        return parent.canonicalize().unwrap_or(path);
    };

    match parent.canonicalize() {
        Ok(parent) => parent.join(file_name),
        Err(_) => path,
    }
}

/// First token of a shell command (`ls -la` → `ls`). Used for `--allow-run`.
pub fn process_command_name(command: &str) -> String {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let first = match trimmed.as_bytes()[0] {
        b'"' => trimmed
            .split('"')
            .nth(1)
            .unwrap_or("")
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string(),
        b'\'' => trimmed
            .split('\'')
            .nth(1)
            .unwrap_or("")
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string(),
        _ => trimmed.split_whitespace().next().unwrap_or("").to_string(),
    };
    first
}

static SANDBOX_STRICT_ENV: AtomicBool = AtomicBool::new(false);

pub fn set_sandbox_strict_env(enabled: bool) {
    SANDBOX_STRICT_ENV.store(enabled, Ordering::SeqCst);
}

pub fn sandbox_strict_env() -> bool {
    SANDBOX_STRICT_ENV.load(Ordering::SeqCst)
}

#[derive(Serialize)]
struct AuditRecord<'a> {
    ts: String,
    kind: &'a str,
    action: &'a str,
    resource: String,
    decision: &'a str,
}

struct AuditSink {
    file: File,
}

static AUDIT_SINK: Lazy<Mutex<Option<AuditSink>>> = Lazy::new(|| Mutex::new(None));
static CURRENT_AUDIT_PATH: Lazy<RwLock<Option<PathBuf>>> = Lazy::new(|| RwLock::new(None));

pub fn get_audit_log_path() -> Option<PathBuf> {
    CURRENT_AUDIT_PATH.read().ok().and_then(|p| p.clone())
}

pub fn set_audit_log_path(path: Option<PathBuf>) -> Result<(), String> {
    let mut sink = AUDIT_SINK
        .lock()
        .map_err(|_| "audit log lock poisoned".to_string())?;
    if let Ok(mut lock) = CURRENT_AUDIT_PATH.write() {
        *lock = path.clone();
    }
    match path {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        format!(
                            "failed to create audit log directory {}: {e}",
                            parent.display()
                        )
                    })?;
                }
            }
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|e| format!("failed to open audit log {}: {e}", path.display()))?;
            *sink = Some(AuditSink { file });
        }
        None => {
            *sink = None;
        }
    }
    Ok(())
}

fn write_audit_record(
    kind: &PermissionKind,
    action: &PermissionAction,
    resource: &ResourceId,
    decision: PermissionDecision,
) {
    let Ok(mut sink) = AUDIT_SINK.lock() else {
        return;
    };
    let Some(sink) = sink.as_mut() else {
        return;
    };
    let record = AuditRecord {
        ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        kind: kind.as_str(),
        action: action.as_str(),
        resource: resource.display_for_audit(),
        decision: decision.as_str(),
    };
    if let Ok(line) = serde_json::to_string(&record) {
        let _ = writeln!(sink.file, "{line}");
        let _ = sink.file.flush();
    }
}

pub fn record_tool_call_audit(tool_name: &str, is_allowed: bool) {
    let kind = PermissionKind::Process;
    let action = PermissionAction::Execute;
    let resource = ResourceId::Name(format!("tool:{tool_name}"));
    let decision = if is_allowed {
        PermissionDecision::Allow
    } else {
        PermissionDecision::Deny
    };
    write_audit_record(&kind, &action, &resource, decision);
}

static DETERMINISTIC_SEED: Lazy<RwLock<Option<u64>>> = Lazy::new(|| RwLock::new(None));
static FROZEN_TIME_MS: Lazy<RwLock<Option<i64>>> = Lazy::new(|| RwLock::new(None));

pub fn set_deterministic_seed(seed: Option<u64>) {
    if let Ok(mut lock) = DETERMINISTIC_SEED.write() {
        *lock = seed;
    }
}

pub fn get_deterministic_seed() -> Option<u64> {
    DETERMINISTIC_SEED.read().ok().and_then(|lock| *lock)
}

pub fn set_frozen_time_ms(time_ms: Option<i64>) {
    if let Ok(mut lock) = FROZEN_TIME_MS.write() {
        *lock = time_ms;
    }
}

pub fn get_frozen_time_ms() -> Option<i64> {
    FROZEN_TIME_MS.read().ok().and_then(|lock| *lock)
}

pub fn parse_time_spec(spec: &str) -> Result<i64, String> {
    let trimmed = spec.trim();
    if let Ok(ts) = trimmed.parse::<i64>() {
        return Ok(ts);
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.timestamp_millis());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
        return Ok(dt.and_utc().timestamp_millis());
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis());
    }
    Err(format!(
        "invalid time specification '{spec}', expected integer millis or RFC3339/ISO string"
    ))
}

pub fn reset_runtime_permission_state() {
    set_sandbox_strict_env(false);
    let _ = set_audit_log_path(None);
    set_deterministic_seed(None);
    set_frozen_time_ms(None);
    HAS_RESTRICTIONS.store(false, Ordering::SeqCst);
}

pub static HAS_RESTRICTIONS: AtomicBool = AtomicBool::new(false);

#[inline(always)]
pub fn has_restrictions() -> bool {
    HAS_RESTRICTIONS.load(Ordering::Relaxed)
}

pub static GLOBAL_RESOURCE_BROKER: Lazy<RwLock<ResourceBroker>> =
    Lazy::new(|| RwLock::new(ResourceBroker::default()));

pub fn global_resource_broker() -> &'static RwLock<ResourceBroker> {
    &GLOBAL_RESOURCE_BROKER
}

pub fn check_global_permission(
    kind: PermissionKind,
    action: PermissionAction,
    resource: ResourceId,
) -> Result<(), PermissionError> {
    if !has_restrictions() {
        return Ok(());
    }
    GLOBAL_RESOURCE_BROKER
        .read()
        .expect("resource broker lock poisoned")
        .check_result(kind, action, resource)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_command_name_uses_argv0() {
        assert_eq!(process_command_name("ls -la"), "ls");
        assert_eq!(process_command_name("  /usr/bin/echo hi"), "/usr/bin/echo");
        assert_eq!(process_command_name(r#""ls" -la"#), "ls");
    }

    #[test]
    fn path_prefix_allow_beats_wildcard_deny() {
        let mut broker = ResourceBroker::default();
        broker.deny_all();
        let root = std::env::temp_dir().join("amberjs-jail-prefix");
        let _ = std::fs::create_dir_all(root.join("child"));
        broker.allow(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(root.clone()),
        );

        assert_eq!(
            broker.decide(
                PermissionKind::FileSystem,
                PermissionAction::Read,
                &normalize_resource(ResourceId::Path(root.join("child/file.txt"))),
            ),
            PermissionDecision::Allow
        );
        assert_eq!(
            broker.decide(
                PermissionKind::FileSystem,
                PermissionAction::Read,
                &normalize_resource(ResourceId::Path(std::env::temp_dir().join("outside.txt"))),
            ),
            PermissionDecision::Deny
        );
    }
}
