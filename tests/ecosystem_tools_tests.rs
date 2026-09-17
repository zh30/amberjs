//! Integration tests for Amber high-leverage ecosystem tools:
//! - `amber:db` / `amber:sqlite`: embedded SQLite with CRUD, prepared statements & transactions
//! - `amber:vector`: high-dimensional vector similarity store
//! - `amber:std`: modern standard library (dotenv, cli, fs, crypto, assert)
//! - `amber deploy`: Docker & Kubernetes production deployment scaffolding
//! - `amber x`: dynamic package runner

use std::fs;
use tempfile::tempdir;

#[test]
fn test_amber_sqlite_in_rust_engine() {
    // 1. In-memory database
    let handle = amberjs::database::sqlite::open_database(":memory:", false).expect("open memory");
    assert!(handle > 0);

    // 2. Exec schema creation
    amberjs::database::sqlite::exec_database(
        handle,
        "CREATE TABLE agents (id INTEGER PRIMARY KEY, name TEXT, capability TEXT, score REAL);",
    )
    .expect("create table");

    // 3. Insert rows
    let (changes1, id1) = amberjs::database::sqlite::run_statement(
        handle,
        "INSERT INTO agents (name, capability, score) VALUES (?, ?, ?)",
        r#"["researcher", "web-search", 95.5]"#,
    )
    .expect("insert 1");
    assert_eq!(changes1, 1);
    assert_eq!(id1, 1);

    let (changes2, id2) = amberjs::database::sqlite::run_statement(
        handle,
        "INSERT INTO agents (name, capability, score) VALUES (?, ?, ?)",
        r#"["coder", "rust-codegen", 98.0]"#,
    )
    .expect("insert 2");
    assert_eq!(changes2, 1);
    assert_eq!(id2, 2);

    // 4. Query statement
    let rows_json = amberjs::database::sqlite::query_statement(
        handle,
        "SELECT name, score FROM agents WHERE score > ? ORDER BY score DESC",
        r#"[90.0]"#,
    )
    .expect("query");

    let rows: serde_json::Value = serde_json::from_str(&rows_json).expect("parse rows json");
    let arr = rows.as_array().expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"], "coder");
    assert_eq!(arr[0]["score"], 98.0);
    assert_eq!(arr[1]["name"], "researcher");

    // 5. Close database
    let closed = amberjs::database::sqlite::close_database(handle);
    assert!(closed);
}

#[test]
fn test_amber_vector_similarity_search() {
    use amberjs::database::vector::{VectorDB, VectorMetric};

    let mut vdb = VectorDB::new(3, VectorMetric::Cosine);

    // Insert 3 vectors:
    // v1: [1.0, 0.0, 0.0]
    // v2: [0.0, 1.0, 0.0]
    // v3: [0.9, 0.1, 0.0] (very close to v1)
    vdb.insert(
        "item1".to_string(),
        vec![1.0, 0.0, 0.0],
        serde_json::json!({ "tag": "science" }),
    )
    .expect("insert 1");

    vdb.insert(
        "item2".to_string(),
        vec![0.0, 1.0, 0.0],
        serde_json::json!({ "tag": "history" }),
    )
    .expect("insert 2");

    vdb.insert(
        "item3".to_string(),
        vec![0.9, 0.1, 0.0],
        serde_json::json!({ "tag": "ai" }),
    )
    .expect("insert 3");

    assert_eq!(vdb.size(), 3);

    // Query close to v1
    let results = vdb.search(&[1.0, 0.05, 0.0], 2, None).expect("search");
    assert_eq!(results.len(), 2);
    // Top-1 must be item1
    assert_eq!(results[0].id, "item1");
    assert!(results[0].score > 0.99);

    // Delete item1
    assert!(vdb.delete("item1"));
    assert_eq!(vdb.size(), 2);

    // Next search top-1 should be item3
    let results2 = vdb.search(&[1.0, 0.0, 0.0], 1, None).expect("search 2");
    assert_eq!(results2[0].id, "item3");
}

#[test]
fn test_amber_std_dotenv_parser() {
    let raw = r#"
    # Comment line
    PORT=8080
    export HOST=127.0.0.1
    APP_NAME='Amber Modern App'
    SECRET="line1\nline2"
    BASE_URL=http://${HOST}:${PORT}
    INLINE_COMMENT=active # this is a comment
    "#;

    let envs = amberjs::std_lib::dotenv::parse_dotenv(raw);
    assert_eq!(envs.get("PORT").unwrap(), "8080");
    assert_eq!(envs.get("HOST").unwrap(), "127.0.0.1");
    assert_eq!(envs.get("APP_NAME").unwrap(), "Amber Modern App");
    assert_eq!(envs.get("SECRET").unwrap(), "line1\nline2");
    assert_eq!(envs.get("BASE_URL").unwrap(), "http://127.0.0.1:8080");
    assert_eq!(envs.get("INLINE_COMMENT").unwrap(), "active");
}

#[test]
fn test_amber_std_cli_table_formatter() {
    let headers = vec!["ID".to_string(), "Name".to_string(), "Status".to_string()];
    let rows = vec![
        vec![
            "1".to_string(),
            "Worker-A".to_string(),
            "Active".to_string(),
        ],
        vec!["2".to_string(), "Worker-B".to_string(), "Idle".to_string()],
    ];

    let table = amberjs::std_lib::cli::format_table(&headers, &rows);
    assert!(table.contains("Worker-A"));
    assert!(table.contains("Active"));
    assert!(table.contains("┌"));
    assert!(table.contains("├"));
    assert!(table.contains("└"));
}

#[test]
fn test_amber_std_fs_walk_copy_and_empty() {
    let dir = tempdir().expect("tempdir");
    let src_dir = dir.path().join("src_folder");
    let sub_dir = src_dir.join("subdir");
    fs::create_dir_all(&sub_dir).expect("create dirs");

    fs::write(src_dir.join("file1.ts"), "const a = 1;").expect("write file1");
    fs::write(sub_dir.join("file2.js"), "const b = 2;").expect("write file2");
    fs::write(sub_dir.join("readme.md"), "# docs").expect("write doc");

    // 1. Test walk_dir_sync
    let entries = amberjs::std_lib::fs::walk_dir_sync(&src_dir, None, None).expect("walk");
    assert_eq!(entries.len(), 4); // 1 directory + 3 files
    let files_only: Vec<_> = entries.iter().filter(|e| e.is_file).collect();
    assert_eq!(files_only.len(), 3);

    // 2. Test walk with extensions filter
    let ts_entries = amberjs::std_lib::fs::walk_dir_sync(
        &src_dir,
        None,
        Some(&["ts".to_string(), "js".to_string()]),
    )
    .expect("walk filter");
    assert_eq!(ts_entries.len(), 2);

    // 3. Test copy_dir_sync
    let dest_dir = dir.path().join("dest_folder");
    amberjs::std_lib::fs::copy_dir_sync(&src_dir, &dest_dir).expect("copy dir");
    assert!(dest_dir.join("file1.ts").exists());
    assert!(dest_dir.join("subdir").join("file2.js").exists());

    // 4. Test empty_dir_sync
    amberjs::std_lib::fs::empty_dir_sync(&dest_dir).expect("empty dir");
    assert!(dest_dir.exists());
    assert!(!dest_dir.join("file1.ts").exists());
}

#[test]
fn test_amber_std_crypto_uuid_generation() {
    let u4 = amberjs::std_lib::crypto::generate_uuid_v4();
    assert_eq!(u4.len(), 36);
    assert_eq!(&u4[14..15], "4"); // UUID v4 version nibble

    let u7 = amberjs::std_lib::crypto::generate_uuid_v7();
    assert_eq!(u7.len(), 36);
    assert_eq!(&u7[14..15], "7"); // UUID v7 version nibble
}

#[test]
fn test_amber_deploy_docker_and_k8s_scaffolding() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("app.ts");
    fs::write(
        &entry,
        "export default { fetch(req) { return new Response('ok'); } }",
    )
    .expect("write app");

    // Test Docker scaffold
    let docker_files = amberjs::tooling::deploy::generate_docker_scaffold(
        dir.path(),
        "my-service",
        std::path::Path::new("app.ts"),
        3000,
    )
    .expect("docker scaffold");

    assert_eq!(docker_files.len(), 3);
    assert!(dir.path().join("Dockerfile").exists());
    assert!(dir.path().join(".dockerignore").exists());
    assert!(dir.path().join("docker-compose.yml").exists());

    let dockerfile_str =
        fs::read_to_string(dir.path().join("Dockerfile")).expect("read dockerfile");
    assert!(dockerfile_str.contains("FROM alpine"));
    assert!(dockerfile_str.contains("amber"));
    assert!(dockerfile_str.contains("serve"));

    // Test K8s scaffold
    let k8s_files = amberjs::tooling::deploy::generate_k8s_scaffold(dir.path(), "my-service", 3000)
        .expect("k8s scaffold");

    assert_eq!(k8s_files.len(), 2);
    assert!(dir.path().join("k8s").join("deployment.yaml").exists());
    assert!(dir.path().join("k8s").join("service.yaml").exists());

    let deploy_yaml =
        fs::read_to_string(dir.path().join("k8s").join("deployment.yaml")).expect("read yaml");
    assert!(deploy_yaml.contains("kind: Deployment"));
    assert!(deploy_yaml.contains("my-service"));
    assert!(deploy_yaml.contains("livenessProbe"));
}

#[test]
fn test_dlx_spec_parser() {
    let (name1, ver1) = amberjs::tooling::dlx::parse_pkg_spec("cowsay");
    assert_eq!(name1, "cowsay");
    assert_eq!(ver1, None);

    let (name2, ver2) = amberjs::tooling::dlx::parse_pkg_spec("cowsay@1.5.0");
    assert_eq!(name2, "cowsay");
    assert_eq!(ver2, Some("1.5.0".to_string()));

    let (name3, ver3) = amberjs::tooling::dlx::parse_pkg_spec("@biomejs/biome@1.8.0");
    assert_eq!(name3, "@biomejs/biome");
    assert_eq!(ver3, Some("1.8.0".to_string()));
}

#[test]
fn test_minimal_runtime_amber_db_and_std_js_integration() {
    let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new().expect("create runtime");

    let script = r#"
        // 1. Test amber:db / amber:sqlite
        const { Database, VectorDB } = require('amber:db');
        const db = new Database(':memory:');
        db.exec('CREATE TABLE test (id INTEGER PRIMARY KEY, msg TEXT)');
        const ins = db.run('INSERT INTO test (msg) VALUES (?)', 'hello sqlite');
        if (ins.changes !== 1) throw new Error('Expected 1 change');
        const row = db.query('SELECT msg FROM test WHERE id = 1')[0];
        if (row.msg !== 'hello sqlite') throw new Error('Query mismatch: ' + JSON.stringify(row));
        db.close();

        // 2. Test amber:vector
        const vdb = new VectorDB({ dimensions: 2, metric: 'cosine' });
        vdb.insert('a', [1, 0], { name: 'A' });
        vdb.insert('b', [0, 1], { name: 'B' });
        const res = vdb.search([0.9, 0.1], { topK: 1 });
        if (res[0].id !== 'a') throw new Error('Vector search failed');

        // 3. Test amber:std
        const { crypto, cli, dotenv } = require('amber:std');
        const u = crypto.uuid();
        if (typeof u !== 'string' || u.length !== 36) throw new Error('Invalid uuid');

        const token = crypto.jwt.sign({ sub: 'user_1' }, 'secret_key');
        const decoded = crypto.jwt.verify(token, 'secret_key');
        if (decoded.sub !== 'user_1') throw new Error('JWT decode mismatch');

        const greenText = cli.colors.green('ok');
        if (!greenText.includes('\x1b[')) throw new Error('Color ANSI missing');

        const parsed = dotenv.parse('FOO=bar\nBAZ="hello world"');
        if (parsed.FOO !== 'bar' || parsed.BAZ !== 'hello world') throw new Error('Dotenv parse mismatch');
    "#;

    runtime
        .execute_code(script)
        .expect("execute integration script");
}
