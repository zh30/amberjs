use amberjs::permissions::{
    global_resource_broker, PermissionAction, PermissionDecision, PermissionKind, ResourceBroker,
    ResourceId,
};
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;

fn path_for_js(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

fn reset_global_broker() {
    amberjs::permissions::reset_runtime_permission_state();
    *global_resource_broker()
        .write()
        .expect("resource broker lock should not be poisoned") = ResourceBroker::default();
}

struct CurrentDirGuard {
    original: std::path::PathBuf,
}

impl CurrentDirGuard {
    fn switch_to(path: &std::path::Path) -> Self {
        let original = std::env::current_dir().expect("failed to read current directory");
        std::env::set_current_dir(path).expect("failed to switch current directory");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

#[test]
fn broker_allows_by_default_for_compatibility() {
    let broker = ResourceBroker::default();
    let decision = broker.check(
        PermissionKind::FileSystem,
        PermissionAction::Read,
        ResourceId::Path("/tmp/amberjs.txt".into()),
    );

    assert_eq!(decision, PermissionDecision::Allow);
}

#[test]
fn broker_denies_exact_resource_rule() {
    let mut broker = ResourceBroker::default();
    broker.deny(
        PermissionKind::FileSystem,
        PermissionAction::Read,
        ResourceId::Path("/tmp/secret.txt".into()),
    );

    assert_eq!(
        broker.check(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path("/tmp/secret.txt".into()),
        ),
        PermissionDecision::Deny
    );
    assert_eq!(
        broker.check(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path("/tmp/public.txt".into()),
        ),
        PermissionDecision::Allow
    );
}

#[test]
fn broker_keeps_read_and_write_rules_separate() {
    let mut broker = ResourceBroker::default();
    let path = ResourceId::Path("/tmp/read-only.txt".into());
    broker.deny(
        PermissionKind::FileSystem,
        PermissionAction::Write,
        path.clone(),
    );

    assert_eq!(
        broker.check(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            path.clone(),
        ),
        PermissionDecision::Allow
    );
    assert_eq!(
        broker.check(PermissionKind::FileSystem, PermissionAction::Write, path),
        PermissionDecision::Deny
    );
}

#[test]
fn broker_allows_path_prefix_under_deny_all() {
    let temp = tempfile::tempdir().unwrap();
    let jail = temp.path().join("workspace");
    fs::create_dir_all(jail.join("nested")).unwrap();
    fs::write(jail.join("nested/ok.txt"), "ok").unwrap();
    let outside = temp.path().join("secret.txt");
    fs::write(&outside, "nope").unwrap();

    let mut broker = ResourceBroker::default();
    broker.deny_all();
    broker.allow(
        PermissionKind::FileSystem,
        PermissionAction::Read,
        ResourceId::Path(jail.clone()),
    );

    assert_eq!(
        broker.check(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(jail.join("nested/ok.txt")),
        ),
        PermissionDecision::Allow
    );
    assert_eq!(
        broker.check(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(outside),
        ),
        PermissionDecision::Deny
    );
}

#[test]
fn broker_allows_exact_exception_to_wildcard_deny() {
    let mut broker = ResourceBroker::default();
    let allowed_path = ResourceId::Path("/tmp/allowed.txt".into());
    broker.deny(
        PermissionKind::FileSystem,
        PermissionAction::Read,
        ResourceId::Any,
    );
    broker.allow(
        PermissionKind::FileSystem,
        PermissionAction::Read,
        allowed_path.clone(),
    );

    assert_eq!(
        broker.check(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            allowed_path
        ),
        PermissionDecision::Allow
    );
    assert_eq!(
        broker.check(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path("/tmp/other.txt".into()),
        ),
        PermissionDecision::Deny
    );
}

#[test]
fn broker_normalizes_equivalent_paths_before_checking_rules() {
    let temp = tempfile::tempdir().unwrap();
    let nested = temp.path().join("nested");
    fs::create_dir_all(&nested).unwrap();
    let secret_path = temp.path().join("secret.txt");
    fs::write(&secret_path, "classified-value").unwrap();
    let equivalent_path = nested.join("..").join("secret.txt");

    let mut broker = ResourceBroker::default();
    broker.deny(
        PermissionKind::FileSystem,
        PermissionAction::Read,
        ResourceId::Path(secret_path),
    );

    assert_eq!(
        broker.check(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(equivalent_path),
        ),
        PermissionDecision::Deny
    );
}

#[test]
fn broker_normalizes_relative_nonexistent_path_against_absolute_rule() {
    let temp = tempfile::tempdir().unwrap();
    let _cwd = CurrentDirGuard::switch_to(temp.path());
    let cache_path = temp.path().join(".amberjs_cache");

    let mut broker = ResourceBroker::default();
    broker.deny(
        PermissionKind::FileSystem,
        PermissionAction::Write,
        ResourceId::Any,
    );
    broker.allow(
        PermissionKind::FileSystem,
        PermissionAction::Write,
        ResourceId::Path(cache_path),
    );

    assert_eq!(
        broker.check(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Path(".amberjs_cache".into()),
        ),
        PermissionDecision::Allow
    );
}

#[test]
fn broker_can_gate_environment_variables_by_key() {
    let mut broker = ResourceBroker::default();
    broker.deny(
        PermissionKind::Environment,
        PermissionAction::Read,
        ResourceId::Name("SECRET_TOKEN".into()),
    );

    assert_eq!(
        broker.check(
            PermissionKind::Environment,
            PermissionAction::Read,
            ResourceId::Name("SECRET_TOKEN".into()),
        ),
        PermissionDecision::Deny
    );
    assert_eq!(
        broker.check(
            PermissionKind::Environment,
            PermissionAction::Read,
            ResourceId::Name("PATH".into()),
        ),
        PermissionDecision::Allow
    );
}

#[test]
fn broker_allows_network_host_exception_to_wildcard_deny() {
    let mut broker = ResourceBroker::default();
    broker.deny(
        PermissionKind::Network,
        PermissionAction::Connect,
        ResourceId::Any,
    );
    broker.allow(
        PermissionKind::Network,
        PermissionAction::Connect,
        ResourceId::Name("allowed.example".to_string()),
    );

    assert_eq!(
        broker.check(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Url("wss://allowed.example/socket".to_string()),
        ),
        PermissionDecision::Allow
    );
    assert_eq!(
        broker.check(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Url("wss://blocked.example/socket".to_string()),
        ),
        PermissionDecision::Deny
    );
}

#[test]
fn broker_allows_exact_network_url_exception_to_wildcard_deny() {
    let mut broker = ResourceBroker::default();
    broker.deny(
        PermissionKind::Network,
        PermissionAction::Connect,
        ResourceId::Any,
    );
    broker.allow(
        PermissionKind::Network,
        PermissionAction::Connect,
        ResourceId::Url("wss://allowed.example/socket".to_string()),
    );

    assert_eq!(
        broker.check(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Url("wss://allowed.example/socket".to_string()),
        ),
        PermissionDecision::Allow
    );
    assert_eq!(
        broker.check(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Url("wss://allowed.example/other".to_string()),
        ),
        PermissionDecision::Deny
    );
}

#[test]
fn broker_keeps_network_connect_and_listen_rules_separate() {
    let mut broker = ResourceBroker::default();
    broker.deny(
        PermissionKind::Network,
        PermissionAction::Listen,
        ResourceId::Any,
    );
    broker.allow(
        PermissionKind::Network,
        PermissionAction::Connect,
        ResourceId::Name("127.0.0.1".to_string()),
    );

    assert_eq!(
        broker.check(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Url("http://127.0.0.1:3000/".to_string()),
        ),
        PermissionDecision::Allow
    );
    assert_eq!(
        broker.check(
            PermissionKind::Network,
            PermissionAction::Listen,
            ResourceId::Url("http://127.0.0.1:3000/".to_string()),
        ),
        PermissionDecision::Deny
    );
}

#[test]
#[serial]
fn process_env_filters_denied_environment_variables() {
    const SECRET_NAME: &str = "AMBER_PERMISSION_TEST_SECRET";

    std::env::set_var(SECRET_NAME, "classified-value");
    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::Environment,
            PermissionAction::Read,
            ResourceId::Name(SECRET_NAME.into()),
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code(&format!(
            r#"
            process.env.{SECRET_NAME} === undefined &&
            !Object.prototype.hasOwnProperty.call(process.env, "{SECRET_NAME}");
            "#
        ))
        .unwrap();

    reset_global_broker();
    std::env::remove_var(SECRET_NAME);

    assert_eq!(
        result.trim(),
        "true",
        "process.env must not expose environment variables denied by the broker"
    );
}

#[test]
#[serial]
fn process_env_rechecks_permission_after_runtime_initialization() {
    const SECRET_NAME: &str = "AMBER_PERMISSION_DYNAMIC_SECRET";

    std::env::set_var(SECRET_NAME, "classified-value");
    reset_global_broker();

    let mut runtime = MinimalRuntime::new().unwrap();
    let before_deny = runtime
        .execute_code(&format!("process.env.{SECRET_NAME};"))
        .unwrap();
    assert_eq!(
        before_deny.trim(),
        "classified-value",
        "test setup should expose the env var before broker deny"
    );

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        broker.deny(
            PermissionKind::Environment,
            PermissionAction::Read,
            ResourceId::Name(SECRET_NAME.into()),
        );
    }

    let result = runtime
        .execute_code(&format!(
            r#"
            [
                process.env.{SECRET_NAME} === undefined,
                !Object.prototype.hasOwnProperty.call(process.env, "{SECRET_NAME}"),
                !Object.keys(process.env).includes("{SECRET_NAME}")
            ].every(Boolean);
            "#
        ))
        .unwrap();

    reset_global_broker();
    std::env::remove_var(SECRET_NAME);

    assert_eq!(
        result.trim(),
        "true",
        "process.env must re-check environment broker after runtime initialization"
    );
}

#[test]
#[serial]
fn require_fs_read_file_sync_uses_global_permission_broker() {
    let temp = tempfile::tempdir().unwrap();
    let secret_path = temp.path().join("secret.txt");
    fs::write(&secret_path, "secret").unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(secret_path.clone()),
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        try {{
            require("fs").readFileSync("{}");
            "allowed";
        }} catch (error) {{
            String(error && error.message ? error.message : error);
        }}
        "#,
        path_for_js(&secret_path)
    );
    let result = runtime.execute_code(&code).unwrap();

    reset_global_broker();

    assert!(
        result.contains("permission denied"),
        "require(\"fs\") must not bypass broker deny, got: {result}"
    );
}

#[test]
#[serial]
fn esm_dependency_import_uses_global_file_read_broker() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    let dependency_path = app_dir.join("secret.mjs");
    fs::write(
        &dependency_path,
        r#"
        globalThis.__esmDeniedDependencyExecuted = true;
        export const secret = "classified";
        "#,
    )
    .unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(dependency_path.clone()),
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    runtime.set_main_module_path(&main_path);
    let error = runtime
        .execute_code(
            r#"
            import { secret } from './secret.mjs';
            globalThis.__esmDeniedSecret = secret;
            "#,
        )
        .expect_err("denied ESM dependency import must fail")
        .to_string();

    reset_global_broker();

    assert!(
        error.contains("permission denied"),
        "ESM dependency loading must surface broker deny, got: {error}"
    );

    let leak_check = runtime
        .execute_code(
            r#"
            globalThis.__esmDeniedDependencyExecuted === undefined &&
            globalThis.__esmDeniedSecret === undefined
            "#,
        )
        .unwrap();
    assert_eq!(
        leak_check.trim(),
        "true",
        "denied ESM dependency must not execute side effects"
    );
}

#[test]
#[serial]
fn esm_fs_builtin_import_uses_global_file_read_broker() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    let secret_path = app_dir.join("secret.txt");
    fs::write(&secret_path, "classified-value").unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(secret_path.clone()),
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    runtime.set_main_module_path(&main_path);
    let error = runtime
        .execute_code(&format!(
            r#"
            import {{ readFileSync }} from 'fs';
            export const forceNativeModule = true;
            globalThis.__esmDeniedFsRead = readFileSync("{}", "utf8");
            "#,
            path_for_js(&secret_path)
        ))
        .expect_err("denied ESM fs read must fail")
        .to_string();

    reset_global_broker();

    assert!(
        error.contains("permission denied"),
        "ESM fs builtin import must surface broker deny, got: {error}"
    );

    let leak_check = runtime
        .execute_code("globalThis.__esmDeniedFsRead === undefined")
        .unwrap();
    assert_eq!(
        leak_check.trim(),
        "true",
        "denied ESM fs read must not leak file contents"
    );
}

#[test]
#[serial]
fn require_fs_promises_read_file_uses_global_permission_broker() {
    let temp = tempfile::tempdir().unwrap();
    let secret_path = temp.path().join("secret.txt");
    fs::write(&secret_path, "secret").unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(secret_path.clone()),
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        try {{
            require("fs/promises").readFile("{}");
            "allowed";
        }} catch (error) {{
            String(error && error.message ? error.message : error);
        }}
        "#,
        path_for_js(&secret_path)
    );
    let result = runtime.execute_code(&code).unwrap();

    reset_global_broker();

    assert!(
        result.contains("permission denied"),
        "require(\"fs/promises\") must not bypass broker deny, got: {result}"
    );
}

#[test]
#[serial]
fn require_fs_sync_methods_use_global_permission_broker() {
    let temp = tempfile::tempdir().unwrap();
    let file_path = temp.path().join("secret.txt");
    let renamed_path = temp.path().join("renamed.txt");
    let new_dir_path = temp.path().join("new-dir");
    let remove_dir_path = temp.path().join("remove-dir");
    fs::write(&file_path, "secret").unwrap();
    fs::create_dir(&remove_dir_path).unwrap();

    let cases = [
        (
            PermissionAction::Read,
            file_path.clone(),
            format!(
                r#"require("fs").existsSync("{}");"#,
                path_for_js(&file_path)
            ),
        ),
        (
            PermissionAction::Write,
            new_dir_path.clone(),
            format!(
                r#"require("fs").mkdirSync("{}");"#,
                path_for_js(&new_dir_path)
            ),
        ),
        (
            PermissionAction::Read,
            temp.path().to_path_buf(),
            format!(
                r#"require("fs").readdirSync("{}");"#,
                path_for_js(temp.path())
            ),
        ),
        (
            PermissionAction::Read,
            file_path.clone(),
            format!(r#"require("fs").statSync("{}");"#, path_for_js(&file_path)),
        ),
        (
            PermissionAction::Write,
            file_path.clone(),
            format!(
                r#"require("fs").unlinkSync("{}");"#,
                path_for_js(&file_path)
            ),
        ),
        (
            PermissionAction::Write,
            file_path.clone(),
            format!(
                r#"require("fs").renameSync("{}", "{}");"#,
                path_for_js(&file_path),
                path_for_js(&renamed_path)
            ),
        ),
        (
            PermissionAction::Write,
            remove_dir_path.clone(),
            format!(
                r#"require("fs").rmdirSync("{}");"#,
                path_for_js(&remove_dir_path)
            ),
        ),
    ];

    for (action, denied_path, statement) in cases {
        {
            let mut broker = global_resource_broker()
                .write()
                .expect("resource broker lock should not be poisoned");
            *broker = ResourceBroker::default();
            broker.deny(
                PermissionKind::FileSystem,
                action,
                ResourceId::Path(denied_path),
            );
        }

        let mut runtime = MinimalRuntime::new().unwrap();
        let code = format!(
            r#"
            try {{
                {statement}
                "allowed";
            }} catch (error) {{
                String(error && error.message ? error.message : error);
            }}
            "#
        );
        let result = runtime.execute_code(&code).unwrap();

        assert!(
            result.contains("permission denied"),
            "fs method must not bypass broker deny, statement: {statement}, got: {result}"
        );
    }

    reset_global_broker();
}

#[test]
#[serial]
fn require_fs_callback_methods_use_global_permission_broker() {
    let temp = tempfile::tempdir().unwrap();
    let read_path = temp.path().join("read.txt");
    let write_path = temp.path().join("write.txt");
    let append_path = temp.path().join("append.txt");
    fs::write(&read_path, "secret").unwrap();
    fs::write(&append_path, "seed").unwrap();

    let cases = [
        (
            PermissionAction::Read,
            read_path.clone(),
            format!(
                r#"require("fs").readFile("{}", "utf8", () => {{}});"#,
                path_for_js(&read_path)
            ),
        ),
        (
            PermissionAction::Write,
            write_path.clone(),
            format!(
                r#"require("fs").writeFile("{}", "data", () => {{}});"#,
                path_for_js(&write_path)
            ),
        ),
        (
            PermissionAction::Write,
            append_path.clone(),
            format!(
                r#"require("fs").appendFile("{}", "data", () => {{}});"#,
                path_for_js(&append_path)
            ),
        ),
    ];

    for (action, denied_path, statement) in cases {
        {
            let mut broker = global_resource_broker()
                .write()
                .expect("resource broker lock should not be poisoned");
            *broker = ResourceBroker::default();
            broker.deny(
                PermissionKind::FileSystem,
                action,
                ResourceId::Path(denied_path),
            );
        }

        let mut runtime = MinimalRuntime::new().unwrap();
        let code = format!(
            r#"
            try {{
                {statement}
                "allowed";
            }} catch (error) {{
                String(error && error.message ? error.message : error);
            }}
            "#
        );
        let result = runtime.execute_code(&code).unwrap();

        assert!(
            result.contains("permission denied"),
            "fs callback method must not bypass broker deny, statement: {statement}, got: {result}"
        );
    }

    reset_global_broker();
}

#[test]
#[serial]
fn require_fs_promises_methods_use_global_permission_broker() {
    let temp = tempfile::tempdir().unwrap();
    let file_path = temp.path().join("secret.txt");
    let renamed_path = temp.path().join("renamed.txt");
    let new_dir_path = temp.path().join("new-dir");
    fs::write(&file_path, "secret").unwrap();

    let cases = [
        (
            PermissionAction::Write,
            new_dir_path.clone(),
            format!(
                r#"require("fs/promises").mkdir("{}");"#,
                path_for_js(&new_dir_path)
            ),
        ),
        (
            PermissionAction::Read,
            temp.path().to_path_buf(),
            format!(
                r#"require("fs/promises").readdir("{}");"#,
                path_for_js(temp.path())
            ),
        ),
        (
            PermissionAction::Read,
            file_path.clone(),
            format!(
                r#"require("fs/promises").stat("{}");"#,
                path_for_js(&file_path)
            ),
        ),
        (
            PermissionAction::Write,
            file_path.clone(),
            format!(
                r#"require("fs/promises").unlink("{}");"#,
                path_for_js(&file_path)
            ),
        ),
        (
            PermissionAction::Write,
            file_path.clone(),
            format!(
                r#"require("fs/promises").rename("{}", "{}");"#,
                path_for_js(&file_path),
                path_for_js(&renamed_path)
            ),
        ),
    ];

    for (action, denied_path, statement) in cases {
        {
            let mut broker = global_resource_broker()
                .write()
                .expect("resource broker lock should not be poisoned");
            *broker = ResourceBroker::default();
            broker.deny(
                PermissionKind::FileSystem,
                action,
                ResourceId::Path(denied_path),
            );
        }

        let mut runtime = MinimalRuntime::new().unwrap();
        let code = format!(
            r#"
            try {{
                {statement}
                "allowed";
            }} catch (error) {{
                String(error && error.message ? error.message : error);
            }}
            "#
        );
        let result = runtime.execute_code(&code).unwrap();

        assert!(
            result.contains("permission denied"),
            "fs.promises method must not bypass broker deny, statement: {statement}, got: {result}"
        );
    }

    reset_global_broker();
}

#[test]
#[serial]
fn fs_promises_read_file_rechecks_permission_after_thenable_path_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let allowed_path = temp.path().join("allowed.txt");
    let secret_path = temp.path().join("secret.txt");
    fs::write(&allowed_path, "allowed").unwrap();
    fs::write(&secret_path, "secret").unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(secret_path.clone()),
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        const p = require("fs/promises").readFile("{}", "utf8");
        p.__path = "{}";
        try {{
            p.then((content) => content, (error) => String(error));
            p.__result__ || "no-result";
        }} catch (error) {{
            String(error && error.message ? error.message : error);
        }}
        "#,
        path_for_js(&allowed_path),
        path_for_js(&secret_path)
    );
    let result = runtime.execute_code(&code).unwrap();

    reset_global_broker();

    assert!(
        result.contains("permission denied"),
        "fs.promises.readFile must recheck mutated thenable paths, got: {result}"
    );
    assert!(
        !result.contains("classified-value"),
        "denied mutated read must not leak file contents, got: {result}"
    );
}

#[test]
#[serial]
fn fs_promises_write_file_rechecks_permission_after_thenable_path_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let allowed_path = temp.path().join("allowed.txt");
    let denied_path = temp.path().join("denied.txt");

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Path(denied_path.clone()),
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        const p = require("fs/promises").writeFile("{}", "created");
        p.__path = "{}";
        try {{
            p.then(() => "written", (error) => String(error));
            p.__result__ || "no-result";
        }} catch (error) {{
            String(error && error.message ? error.message : error);
        }}
        "#,
        path_for_js(&allowed_path),
        path_for_js(&denied_path)
    );
    let result = runtime.execute_code(&code).unwrap();

    reset_global_broker();

    assert!(
        result.contains("permission denied"),
        "fs.promises.writeFile must recheck mutated thenable paths, got: {result}"
    );
    assert!(
        !denied_path.exists(),
        "denied mutated write must not create the target file, got: {result}"
    );
}

#[test]
#[serial]
fn commonjs_package_json_read_uses_global_permission_broker() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    let package_json_path = package_dir.join("package.json");
    fs::write(&package_json_path, r#"{"name":"pkg","main":"main.js"}"#).unwrap();
    fs::write(
        package_dir.join("main.js"),
        r#"module.exports = { answer: "allowed" };"#,
    )
    .unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(package_json_path),
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        try {{
            require("pkg").answer;
        }} catch (error) {{
            String(error && error.message ? error.message : error);
        }}
        "#,
        path_for_js(&app_dir)
    );
    let result = runtime.execute_code(&code).unwrap();

    reset_global_broker();

    assert!(
        result.contains("permission denied"),
        "CommonJS package.json reads must not bypass broker deny, got: {result}"
    );
    assert!(
        !result.contains("allowed"),
        "denied package.json read must not load package main, got: {result}"
    );
}

#[test]
#[serial]
fn fetch_uses_global_network_permission_broker() {
    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Any,
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code(
            r#"
            try {
                fetch("http://127.0.0.1:9/permission-test");
                "allowed";
            } catch (error) {
                String(error && error.message ? error.message : error);
            }
            "#,
        )
        .unwrap();

    reset_global_broker();

    assert!(
        result.contains("permission denied"),
        "fetch must fail before network I/O when Network/Connect is denied, got: {result}"
    );
}

#[test]
#[serial]
fn websocket_uses_global_network_permission_broker() {
    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Any,
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code(
            r#"
            try {
                new WebSocket("ws://127.0.0.1:9/permission-test");
                "allowed";
            } catch (error) {
                String(error && error.message ? error.message : error);
            }
            "#,
        )
        .unwrap();

    reset_global_broker();

    assert!(
        result.contains("permission denied"),
        "WebSocket must fail before network I/O when Network/Connect is denied, got: {result}"
    );
}

#[test]
#[serial]
fn net_connect_uses_global_network_permission_broker() {
    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Any,
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code(
            r#"
            try {
                net.connect({ host: "127.0.0.1", port: 9 });
                "allowed";
            } catch (error) {
                String(error && error.message ? error.message : error);
            }
            "#,
        )
        .unwrap();

    reset_global_broker();

    assert!(
        result.contains("permission denied"),
        "net.connect must fail before network I/O when Network/Connect is denied, got: {result}"
    );
}

#[test]
#[serial]
fn dns_lookup_uses_global_network_permission_broker() {
    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Any,
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code(
            r#"
            dns.lookup("localhost");
            "#,
        )
        .unwrap();

    reset_global_broker();

    assert!(
        result.contains("permission denied"),
        "dns.lookup must fail before resolver I/O when Network/Connect is denied, got: {result}"
    );
}

#[test]
#[serial]
fn node_http_request_uses_global_network_permission_broker() {
    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Any,
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code(
            r#"
            try {
                const req = http.request({
                    hostname: "127.0.0.1",
                    port: 9,
                    path: "/permission-test"
                });
                req.end();
                "allowed";
            } catch (error) {
                String(error && error.message ? error.message : error);
            }
            "#,
        )
        .unwrap();

    reset_global_broker();

    assert!(
        result.contains("permission denied"),
        "http.request().end must fail before network I/O when Network/Connect is denied, got: {result}"
    );
}

#[test]
#[serial]
fn node_http_server_listen_uses_global_network_permission_broker() {
    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::Network,
            PermissionAction::Listen,
            ResourceId::Any,
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code(
            r#"
            const server = http.createServer();
            try {
                server.listen(0, "127.0.0.1");
                `allowed:${server.listening}`;
            } catch (error) {
                `${String(error && error.message ? error.message : error)}:${server.listening}`;
            }
            "#,
        )
        .unwrap();

    reset_global_broker();

    assert!(
        result.contains("permission denied"),
        "http.createServer().listen must fail before bind when Network/Listen is denied, got: {result}"
    );
    assert!(
        result.ends_with(":false"),
        "denied http server listen must not mark the server as listening, got: {result}"
    );
}

#[test]
#[serial]
fn child_process_uses_global_process_permission_broker() {
    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::Process,
            PermissionAction::Execute,
            ResourceId::Any,
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code(
            r#"
            const childProcess = require("child_process");
            function capture(callback) {
                try {
                    callback();
                    return "allowed";
                } catch (error) {
                    return String(error && error.message ? error.message : error);
                }
            }
            [
                capture(() => childProcess.exec("echo amberjs")),
                capture(() => childProcess.spawn("echo", ["amberjs"])),
                capture(() => childProcess.execFile("echo", ["amberjs"])),
            ].join("\n");
            "#,
        )
        .unwrap();

    reset_global_broker();

    assert!(
        !result.contains("allowed"),
        "child_process must not allow denied process execution, got: {result}"
    );
    assert_eq!(
        result.matches("permission denied").count(),
        3,
        "child_process exec/spawn/execFile must all use broker, got: {result}"
    );
}

#[test]
#[serial]
fn child_process_exec_matches_argv0_not_full_string() {
    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::Process,
            PermissionAction::Execute,
            ResourceId::Any,
        );
        broker.allow(
            PermissionKind::Process,
            PermissionAction::Execute,
            ResourceId::Name("echo".into()),
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code(
            r#"
            try {
                require("child_process").exec("echo amberjs-argv0");
                "allowed";
            } catch (error) {
                String(error && error.message ? error.message : error);
            }
            "#,
        )
        .unwrap();

    reset_global_broker();

    assert_eq!(
        result.trim(),
        "allowed",
        "exec('echo ...') must match --allow-run echo, got: {result}"
    );
}

#[test]
#[serial]
fn process_chdir_uses_global_process_permission_broker() {
    let original_cwd = std::env::current_dir().unwrap();
    let temp = tempfile::tempdir().unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::Process,
            PermissionAction::Execute,
            ResourceId::Any,
        );
    }

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        try {{
            process.chdir("{}");
            "allowed";
        }} catch (error) {{
            String(error && error.message ? error.message : error);
        }}
        "#,
        path_for_js(temp.path())
    );
    let result = runtime.execute_code(&code).unwrap();
    let cwd_after_chdir = std::env::current_dir().unwrap();
    if cwd_after_chdir != original_cwd {
        std::env::set_current_dir(&original_cwd).unwrap();
    }

    reset_global_broker();

    assert!(
        result.contains("permission denied"),
        "process.chdir must use Process/Execute broker, got: {result}"
    );
    assert_eq!(
        cwd_after_chdir, original_cwd,
        "denied process.chdir must not change the host cwd"
    );
}

#[test]
#[serial]
fn require_fs_does_not_fall_back_to_unbrokered_implementation_when_global_binding_is_missing() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code(
            r#"
            try {
                globalThis.fs = null;
                require("fs");
                "allowed";
            } catch (error) {
                String(error && error.message ? error.message : error);
            }
            "#,
        )
        .unwrap();

    assert!(
        result.contains("global fs binding is unavailable"),
        "require(\"fs\") must fail closed when global fs is missing, got: {result}"
    );
}

#[test]
#[serial]
fn require_fs_promises_does_not_fall_back_to_unbrokered_implementation_when_binding_is_missing() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code(
            r#"
            try {
                globalThis.fs = {};
                require("fs/promises");
                "allowed";
            } catch (error) {
                String(error && error.message ? error.message : error);
            }
            "#,
        )
        .unwrap();

    assert!(
        result.contains("global fs.promises binding is unavailable"),
        "require(\"fs/promises\") must fail closed when global fs.promises is missing, got: {result}"
    );
}
