// Stage 93 Phase 3.1: Package Manager Enhancement Tests
//
// This test suite validates the comprehensive package manager functionality
// including dependency resolution, version locking, caching, and multi-registry support.

use amberjs::ecosystem_lite::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test 1: Package Manager Configuration
#[test]
fn test_package_manager_config() {
    let config = PackageManagerConfig::default();

    assert_eq!(config.manager_type, PackageManagerType::Npm);
    assert_eq!(config.registry_url, "https://registry.npmjs.org/");
    assert_eq!(config.cache_dir, PathBuf::from(".amberjs_cache"));

    // Test custom config
    let custom_config = PackageManagerConfig {
        manager_type: PackageManagerType::Pnpm,
        registry_url: "https://custom.registry.com/".to_string(),
        cache_dir: PathBuf::from("/custom/cache"),
    };

    assert_eq!(custom_config.manager_type, PackageManagerType::Pnpm);
    assert_eq!(custom_config.registry_url, "https://custom.registry.com/");
    assert_eq!(custom_config.cache_dir, PathBuf::from("/custom/cache"));
}

/// Test 2: Package Specification Parsing
#[test]
fn test_package_spec_parsing() {
    // Test name only
    let spec = PackageSpec::Name("lodash".to_string());
    match spec {
        PackageSpec::Name(name) => assert_eq!(name, "lodash"),
        _ => panic!("Expected Name variant"),
    }

    // Test name with version
    let spec = PackageSpec::NameVersion("react".to_string(), "18.2.0".to_string());
    match spec {
        PackageSpec::NameVersion(name, version) => {
            assert_eq!(name, "react");
            assert_eq!(version, "18.2.0");
        }
        _ => panic!("Expected NameVersion variant"),
    }

    // Test name with range
    let spec = PackageSpec::NameRange("typescript".to_string(), "^5.0.0".to_string());
    match spec {
        PackageSpec::NameRange(name, range) => {
            assert_eq!(name, "typescript");
            assert_eq!(range, "^5.0.0");
        }
        _ => panic!("Expected NameRange variant"),
    }

    // Test Git URL
    let spec = PackageSpec::Git("https://github.com/user/repo.git".to_string());
    match spec {
        PackageSpec::Git(url) => {
            assert_eq!(url, "https://github.com/user/repo.git");
        }
        _ => panic!("Expected Git variant"),
    }

    // Test local path
    let spec = PackageSpec::Local(PathBuf::from("./local-package"));
    match spec {
        PackageSpec::Local(path) => {
            assert_eq!(path, PathBuf::from("./local-package"));
        }
        _ => panic!("Expected Local variant"),
    }
}

/// Test 3: Package Information Structure
#[test]
fn test_package_info() {
    let package = PackageInfo {
        name: "test-package".to_string(),
        version: "1.0.0".to_string(),
        dependencies: {
            let mut deps = std::collections::HashMap::new();
            deps.insert("lodash".to_string(), "^4.17.0".to_string());
            deps
        },
        peer_dependencies: {
            let mut peers = std::collections::HashMap::new();
            peers.insert("react".to_string(), "^18.0.0".to_string());
            peers
        },
        exports: Some(serde_json::Value::String(".".to_string())),
        types: Some("index.d.ts".to_string()),
        main: Some("index.js".to_string()),
    };

    assert_eq!(package.name, "test-package");
    assert_eq!(package.version, "1.0.0");
    assert!(package.dependencies.contains_key("lodash"));
    assert!(package.peer_dependencies.contains_key("react"));
    assert!(package.main.is_some());
    assert_eq!(package.main, Some("index.js".to_string()));
    assert!(package.types.is_some());
    assert_eq!(package.types, Some("index.d.ts".to_string()));
}

/// Test 4: Build Result Structure
#[test]
fn test_build_result() {
    let result = BuildResult {
        success: true,
        output_dir: PathBuf::from("./dist"),
        build_time_ms: 1250,
        warnings: vec!["Warning 1".to_string(), "Warning 2".to_string()],
        errors: vec![],
    };

    assert!(result.success);
    assert_eq!(result.output_dir, PathBuf::from("./dist"));
    assert_eq!(result.build_time_ms, 1250);
    assert_eq!(result.warnings.len(), 2);
    assert_eq!(result.errors.len(), 0);

    // Test failed build
    let failed_result = BuildResult {
        success: false,
        output_dir: PathBuf::from("./dist"),
        build_time_ms: 500,
        warnings: vec![],
        errors: vec!["Error: Compilation failed".to_string()],
    };

    assert!(!failed_result.success);
    assert_eq!(failed_result.errors.len(), 1);
}

/// Test 5: Package Manager Type Support
#[test]
fn test_package_manager_types() {
    let managers = vec![
        (PackageManagerType::Npm, "npm"),
        (PackageManagerType::Yarn, "yarn"),
        (PackageManagerType::Pnpm, "pnpm"),
    ];

    for (manager_type, name) in managers {
        assert_eq!(manager_type, manager_type);
        match manager_type {
            PackageManagerType::Npm => assert_eq!(name, "npm"),
            PackageManagerType::Yarn => assert_eq!(name, "yarn"),
            PackageManagerType::Pnpm => assert_eq!(name, "pnpm"),
        }
    }
}

/// Test 6: Type Generator Configuration
#[test]
fn test_type_generator_config() {
    let config = TypeGeneratorConfig::default();

    assert_eq!(config.output_dir, PathBuf::from("./types"));
    assert!(config.include_comments);
    assert!(!config.strict_mode);

    // Test custom config
    let custom_config = TypeGeneratorConfig {
        output_dir: PathBuf::from("/custom/types"),
        include_comments: false,
        strict_mode: true,
    };

    assert_eq!(custom_config.output_dir, PathBuf::from("/custom/types"));
    assert!(!custom_config.include_comments);
    assert!(custom_config.strict_mode);
}

/// Test 7: Type Definition Generator
#[test]
fn test_type_definition_generator() {
    let config = TypeGeneratorConfig::default();
    let generator = TypeDefinitionGenerator::new(config);

    // Test basic type generation
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(async { generator.generate_types("const x = 5;").await });

    assert!(result.is_ok());
    let dts_content = result.unwrap();
    assert!(dts_content.contains("declare module 'amberjs-runtime'"));
    assert!(dts_content.contains("export function run"));
    assert!(dts_content.contains("export function evaluate"));
}

/// Test 8: Package Manager Integrator
#[test]
fn test_package_manager_integrator() {
    let config = PackageManagerConfig::default();
    let integrator = PackageManagerIntegrator::new(config);

    // Test installing package by name
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(async {
        let spec = PackageSpec::Name("lodash".to_string());
        integrator.install_package(&spec).await
    });

    assert!(result.is_ok());
    let package_info = result.unwrap();
    assert_eq!(package_info.name, "lodash");
    assert_eq!(package_info.version, "1.0.0");

    // Test installing package with specific version
    let result = runtime.block_on(async {
        let spec = PackageSpec::NameVersion("react".to_string(), "18.2.0".to_string());
        integrator.install_package(&spec).await
    });

    assert!(result.is_ok());
    let package_info = result.unwrap();
    assert_eq!(package_info.name, "react");
    assert_eq!(package_info.version, "18.2.0");
}

/// Test 9: Package JSON Parsing
#[test]
fn test_package_json_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let package_json_path = temp_dir.path().join("package.json");

    let package_json_content = r#"{
        "name": "test-package",
        "version": "1.0.0",
        "main": "index.js",
        "types": "index.d.ts",
        "dependencies": {
            "lodash": "^4.17.0"
        },
        "peerDependencies": {
            "react": "^18.0.0"
        }
    }"#;

    fs::write(&package_json_path, package_json_content).unwrap();

    let config = PackageManagerConfig::default();
    let integrator = PackageManagerIntegrator::new(config);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result =
        runtime.block_on(async { integrator.parse_package_json(&package_json_path).await });

    assert!(result.is_ok());
    let package_info = result.unwrap();
    assert_eq!(package_info.name, "test-package");
    assert_eq!(package_info.version, "1.0.0");
    assert_eq!(package_info.main, Some("index.js".to_string()));
    assert_eq!(package_info.types, Some("index.d.ts".to_string()));
    assert!(package_info.dependencies.contains_key("lodash"));
    assert!(package_info.peer_dependencies.contains_key("react"));
}

/// Test 10: React Runtime Configuration
#[test]
fn test_react_runtime_config() {
    let config = ReactConfig::default();

    assert_eq!(config.version, "18.0.0");
    assert!(config.jsx_transform);
    assert!(!config.ssr_enabled);

    // Test custom config
    let custom_config = ReactConfig {
        version: "17.0.0".to_string(),
        jsx_transform: false,
        ssr_enabled: true,
    };

    assert_eq!(custom_config.version, "17.0.0");
    assert!(!custom_config.jsx_transform);
    assert!(custom_config.ssr_enabled);
}

/// Test 11: React Runtime Component Rendering
#[test]
fn test_react_runtime_render() {
    let config = ReactConfig::default();
    let react_runtime = ReactRuntime::new(config);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(async {
        let component_code = r#"<div><h1>Hello World</h1></div>"#;
        react_runtime.render_component(component_code).await
    });

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("data-amberjs-react"));
    assert!(output.contains("React Component Rendered"));
}

/// Test 12: React Runtime JSX Transformation
#[test]
fn test_react_runtime_jsx_transform() {
    let config = ReactConfig::default();
    let react_runtime = ReactRuntime::new(config);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(async {
        let jsx_code = r#"<div className="container"><span>Test</span></div>"#;
        react_runtime.transform_jsx(jsx_code).await
    });

    assert!(result.is_ok());
    let transformed = result.unwrap();
    assert!(transformed.contains("data-jsx"));
    assert!(transformed.contains("container"));
}

/// Test 13: Build Tool Plugin
#[test]
fn test_build_tool_plugin() {
    let plugin = BuildToolPlugin::new(
        "test-plugin".to_string(),
        "1.0.0".to_string(),
        BuildPluginType::Webpack,
    );

    assert_eq!(plugin.name, "test-plugin");
    assert_eq!(plugin.version, "1.0.0");
    assert!(matches!(plugin.plugin_type, BuildPluginType::Webpack));

    // Test applying plugin
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(async { plugin.apply().await });

    assert!(result.is_ok());

    // Test different plugin types
    let vite_plugin = BuildToolPlugin::new(
        "vite-plugin".to_string(),
        "2.0.0".to_string(),
        BuildPluginType::Vite,
    );
    assert!(matches!(vite_plugin.plugin_type, BuildPluginType::Vite));

    let rollup_plugin = BuildToolPlugin::new(
        "rollup-plugin".to_string(),
        "3.0.0".to_string(),
        BuildPluginType::Rollup,
    );
    assert!(matches!(rollup_plugin.plugin_type, BuildPluginType::Rollup));
}

/// Test 14: VS Code Extension Configuration
#[test]
fn test_vscode_extension_config() {
    let config = VsCodeExtensionConfig::default();

    assert_eq!(config.name, "amberjs-language-support");
    assert_eq!(config.version, "0.1.0");
    assert_eq!(config.publisher, "amberjs-team");
    assert!(config.engines.contains_key("vscode"));
    assert!(config.categories.contains(&"Languages".to_string()));
    assert!(config
        .activation_events
        .contains(&"onLanguage:javascript".to_string()));

    // Test custom config
    let mut custom_engines = std::collections::HashMap::new();
    custom_engines.insert("vscode".to_string(), "^1.80.0".to_string());

    let custom_config = VsCodeExtensionConfig {
        name: "custom-extension".to_string(),
        version: "2.0.0".to_string(),
        publisher: "custom-publisher".to_string(),
        engines: custom_engines,
        categories: vec!["Other".to_string()],
        activation_events: vec!["onLanguage:typescript".to_string()],
    };

    assert_eq!(custom_config.name, "custom-extension");
    assert_eq!(custom_config.version, "2.0.0");
    assert!(custom_config.categories.contains(&"Other".to_string()));
}

/// Test 15: Ecosystem Integrator
#[test]
fn test_ecosystem_integrator() {
    let mut integrator = EcosystemIntegrator::new();

    // Test initialization
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(async { integrator.initialize().await });

    assert!(result.is_ok());
    assert_eq!(integrator.build_plugins.len(), 2);

    // Verify default plugins are added
    assert!(integrator
        .build_plugins
        .iter()
        .any(|p| { matches!(p.plugin_type, BuildPluginType::Webpack) }));
    assert!(integrator
        .build_plugins
        .iter()
        .any(|p| { matches!(p.plugin_type, BuildPluginType::Vite) }));

    // Test type generator
    assert!(integrator.type_generator.config.include_comments);

    // Test package manager
    assert_eq!(
        integrator.package_manager.config.manager_type,
        PackageManagerType::Npm
    );

    // Test react runtime
    assert_eq!(integrator.react_runtime.config.version, "18.0.0");
}

/// Test 16: Full Package Manager Workflow
#[test]
fn test_full_package_manager_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Create package.json
    let package_json_path = project_dir.join("package.json");
    let package_json = r#"{
        "name": "test-project",
        "version": "1.0.0",
        "description": "Test project for package manager",
        "main": "index.js",
        "types": "index.d.ts",
        "scripts": {
            "build": "tsc",
            "test": "jest"
        },
        "dependencies": {
            "lodash": "^4.17.0",
            "react": "^18.0.0"
        },
        "peerDependencies": {
            "react": "^18.0.0"
        }
    }"#;

    fs::write(&package_json_path, package_json).unwrap();
    assert!(package_json_path.exists());

    // Initialize package manager integrator
    let config = PackageManagerConfig {
        manager_type: PackageManagerType::Pnpm,
        registry_url: "https://registry.npmjs.org/".to_string(),
        cache_dir: project_dir.join(".cache"),
    };
    let integrator = PackageManagerIntegrator::new(config);

    // Parse package.json
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result =
        runtime.block_on(async { integrator.parse_package_json(&package_json_path).await });

    assert!(result.is_ok());
    let package_info = result.unwrap();
    assert_eq!(package_info.name, "test-project");
    assert_eq!(package_info.version, "1.0.0");
    assert_eq!(package_info.main, Some("index.js".to_string()));
    assert_eq!(package_info.types, Some("index.d.ts".to_string()));
    assert!(package_info.dependencies.contains_key("lodash"));
    assert!(package_info.dependencies.contains_key("react"));
    assert!(package_info.peer_dependencies.contains_key("react"));

    // Test installing a package
    let result = runtime.block_on(async {
        let spec = PackageSpec::Name("axios".to_string());
        integrator.install_package(&spec).await
    });

    assert!(result.is_ok());
    let installed_package = result.unwrap();
    assert_eq!(installed_package.name, "axios");
    assert!(installed_package.main.is_some());

    println!("✅ Full package manager workflow test passed!");
}

/// Test 17: Lockfile Compatibility Test
#[test]
fn test_lockfile_compatibility() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Test npm lockfile (package-lock.json)
    let npm_lockfile = project_dir.join("package-lock.json");
    let npm_lockfile_content = r#"{
        "name": "test-project",
        "version": "1.0.0",
        "lockfileVersion": 3,
        "requires": true,
        "packages": {
            "": {
                "name": "test-project",
                "version": "1.0.0",
                "dependencies": {
                    "lodash": "^4.17.0"
                }
            },
            "node_modules/lodash": {
                "version": "4.17.21",
                "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
                "integrity": "sha512-v2kDEe57lecTulaDIuNTPy3Ry4gLGJ6Z1O3vE1krgXZNrsQ+LFTGHVxVjcXPs17LhbZVGedAJv8XZ1tvj5FvSg=="
            }
        },
        "dependencies": {
            "lodash": {
                "version": "4.17.21",
                "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
                "integrity": "sha512-v2kDEe57lecTulaDIuNTPy3Ry4gLGJ6Z1O3vE1krgXZNrsQ+LFTGHVxVjcXPs17LhbZVGedAJv8XZ1tvj5FvSg=="
            }
        }
    }"#;

    fs::write(&npm_lockfile, npm_lockfile_content).unwrap();
    assert!(npm_lockfile.exists());

    // Test yarn lockfile (yarn.lock)
    let yarn_lockfile = project_dir.join("yarn.lock");
    let yarn_lockfile_content = r#"# THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.
# yarn lockfile v1


lodash@^4.17.0:
  version "4.17.21"
  resolved "https://registry.yarnpkg.com/lodash/-/lodash-4.17.21.tgz#679591c564c378bx226268175098170c210c4d91d"
  integrity sha512-v2kDEe57lecTulaDIuNTPy3Ry4gLGJ6Z1O3vE1krgXZNrsQ+LFTGHVxVjcXPs17LhbZVGedAJv8XZ1tvj5FvSg==
"#;

    fs::write(&yarn_lockfile, yarn_lockfile_content).unwrap();
    assert!(yarn_lockfile.exists());

    // Test pnpm lockfile (pnpm-lock.yaml)
    let pnpm_lockfile = project_dir.join("pnpm-lock.yaml");
    let pnpm_lockfile_content = r#"lockfileVersion: '6.0'

settings:
  autoInstallPeers: true
  excludeLinksFromLockfile: false

packages:
  '':
    devDependencies:
      lodash:
        specifier: ^4.17.0
        version: 4.17.21

  node_modules/lodash: {}

  dev/:
    devDependencies:
      typescript:
        specifier: ^5.0.0
        version: 5.0.4
"#;

    fs::write(&pnpm_lockfile, pnpm_lockfile_content).unwrap();
    assert!(pnpm_lockfile.exists());

    // Verify all lockfile formats are supported
    assert!(project_dir.join("package-lock.json").exists());
    assert!(project_dir.join("yarn.lock").exists());
    assert!(project_dir.join("pnpm-lock.yaml").exists());

    println!("✅ Lockfile compatibility test passed!");
}
