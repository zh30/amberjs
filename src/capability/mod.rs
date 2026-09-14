// Beejs v1.6.0: Enterprise Capability-Based Security (`bee:security` / `bee:permissions`)
//
// Provides JavaScript runtime introspection and enforcement of fine-grained capabilities:
// - Query and test permissions (fs, net, env, run)
// - Introspect active allow/deny rule sets and sandbox mode
// - Dynamic revocation of permissions for defense-in-depth
// - Capability policy construction and attenuation for multi-tenant workers

use std::path::PathBuf;

use rusty_v8 as v8;
use serde::{Deserialize, Serialize};

use crate::permissions::{
    global_resource_broker, has_restrictions, sandbox_strict_env, PermissionAction, PermissionKind,
    ResourceId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionQueryDescriptor {
    pub name: String,
    pub path: Option<String>,
    pub host: Option<String>,
    pub var_name: Option<String>,
    pub command: Option<String>,
}

fn parse_descriptor_to_parts(
    name: &str,
    path: Option<String>,
    host: Option<String>,
    var_name: Option<String>,
    command: Option<String>,
) -> (PermissionKind, PermissionAction, ResourceId) {
    match name.to_lowercase().as_str() {
        "read" => {
            let res = path
                .map(|p| ResourceId::Path(PathBuf::from(p)))
                .unwrap_or(ResourceId::Any);
            (PermissionKind::FileSystem, PermissionAction::Read, res)
        }
        "write" => {
            let res = path
                .map(|p| ResourceId::Path(PathBuf::from(p)))
                .unwrap_or(ResourceId::Any);
            (PermissionKind::FileSystem, PermissionAction::Write, res)
        }
        "fs" | "filesystem" => {
            let res = path
                .map(|p| ResourceId::Path(PathBuf::from(p)))
                .unwrap_or(ResourceId::Any);
            (PermissionKind::FileSystem, PermissionAction::Read, res)
        }
        "net" | "network" | "connect" => {
            let res = host
                .map(|h| {
                    if h.starts_with("http://") || h.starts_with("https://") {
                        ResourceId::Url(h)
                    } else {
                        ResourceId::Url(format!("https://{}", h))
                    }
                })
                .unwrap_or(ResourceId::Any);
            (PermissionKind::Network, PermissionAction::Connect, res)
        }
        "listen" => {
            let res = host.map(ResourceId::Name).unwrap_or(ResourceId::Any);
            (PermissionKind::Network, PermissionAction::Listen, res)
        }
        "env" | "environment" => {
            let res = var_name.map(ResourceId::Name).unwrap_or(ResourceId::Any);
            (PermissionKind::Environment, PermissionAction::Read, res)
        }
        "run" | "exec" | "process" => {
            let res = command.map(ResourceId::Name).unwrap_or(ResourceId::Any);
            (PermissionKind::Process, PermissionAction::Execute, res)
        }
        _ => (
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Any,
        ),
    }
}

/// Sets up `bee:security` / `bee:permissions` inside V8 context
pub fn setup_security_api(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> anyhow::Result<()> {
    let security_obj = v8::Object::new(scope);
    let permissions_obj = v8::Object::new(scope);

    // 1. permissions.query({ name, path, host, varName, command })
    let query_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() == 0 || !args.get(0).is_object() {
                let msg = v8::String::new(scope, "permissions.query requires a descriptor object")
                    .unwrap();
                let exc = v8::Exception::type_error(scope, msg);
                scope.throw_exception(exc);
                return;
            }

            let obj = args.get(0).to_object(scope).unwrap();
            let name_key = v8::String::new(scope, "name").unwrap();
            let name_val = obj
                .get(scope, name_key.into())
                .map(|v| v.to_rust_string_lossy(scope))
                .unwrap_or_else(|| "read".to_string());

            let path_key = v8::String::new(scope, "path").unwrap();
            let path_val = obj.get(scope, path_key.into()).and_then(|v| {
                if v.is_string() {
                    Some(v.to_rust_string_lossy(scope))
                } else {
                    None
                }
            });

            let host_key = v8::String::new(scope, "host").unwrap();
            let host_val = obj.get(scope, host_key.into()).and_then(|v| {
                if v.is_string() {
                    Some(v.to_rust_string_lossy(scope))
                } else {
                    None
                }
            });

            let var_key = v8::String::new(scope, "varName").unwrap();
            let var_val = obj.get(scope, var_key.into()).and_then(|v| {
                if v.is_string() {
                    Some(v.to_rust_string_lossy(scope))
                } else {
                    None
                }
            });

            let cmd_key = v8::String::new(scope, "command").unwrap();
            let cmd_val = obj.get(scope, cmd_key.into()).and_then(|v| {
                if v.is_string() {
                    Some(v.to_rust_string_lossy(scope))
                } else {
                    None
                }
            });

            let (kind, action, resource) =
                parse_descriptor_to_parts(&name_val, path_val, host_val, var_val, cmd_val);

            let broker = global_resource_broker().read().unwrap();
            let decision = broker.check(kind, action, resource);
            let state_str = if decision.is_allowed() {
                "granted"
            } else {
                "denied"
            };

            let res_obj = v8::Object::new(scope);
            let state_key = v8::String::new(scope, "state").unwrap();
            let state_val = v8::String::new(scope, state_str).unwrap();
            res_obj.set(scope, state_key.into(), state_val.into());

            let name_prop = v8::String::new(scope, &name_val).unwrap();
            res_obj.set(scope, name_key.into(), name_prop.into());

            rv.set(res_obj.into());
        },
    )
    .unwrap();
    let query_key = v8::String::new(scope, "query").unwrap();
    permissions_obj.set(scope, query_key.into(), query_fn.into());

    // 2. permissions.has(descriptor) -> boolean
    let has_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() == 0 || !args.get(0).is_object() {
                rv.set(v8::Boolean::new(scope, false).into());
                return;
            }

            let obj = args.get(0).to_object(scope).unwrap();
            let name_key = v8::String::new(scope, "name").unwrap();
            let name_val = obj
                .get(scope, name_key.into())
                .map(|v| v.to_rust_string_lossy(scope))
                .unwrap_or_else(|| "read".to_string());

            let path_key = v8::String::new(scope, "path").unwrap();
            let path_val = obj.get(scope, path_key.into()).and_then(|v| {
                if v.is_string() {
                    Some(v.to_rust_string_lossy(scope))
                } else {
                    None
                }
            });

            let host_key = v8::String::new(scope, "host").unwrap();
            let host_val = obj.get(scope, host_key.into()).and_then(|v| {
                if v.is_string() {
                    Some(v.to_rust_string_lossy(scope))
                } else {
                    None
                }
            });

            let var_key = v8::String::new(scope, "varName").unwrap();
            let var_val = obj.get(scope, var_key.into()).and_then(|v| {
                if v.is_string() {
                    Some(v.to_rust_string_lossy(scope))
                } else {
                    None
                }
            });

            let cmd_key = v8::String::new(scope, "command").unwrap();
            let cmd_val = obj.get(scope, cmd_key.into()).and_then(|v| {
                if v.is_string() {
                    Some(v.to_rust_string_lossy(scope))
                } else {
                    None
                }
            });

            let (kind, action, resource) =
                parse_descriptor_to_parts(&name_val, path_val, host_val, var_val, cmd_val);

            let broker = global_resource_broker().read().unwrap();
            let decision = broker.check(kind, action, resource);
            rv.set(v8::Boolean::new(scope, decision.is_allowed()).into());
        },
    )
    .unwrap();
    let has_key = v8::String::new(scope, "has").unwrap();
    permissions_obj.set(scope, has_key.into(), has_fn.into());

    // 3. permissions.list()
    let list_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            let broker = global_resource_broker().read().unwrap();
            let has_res = has_restrictions();
            let strict_env = sandbox_strict_env();

            let res_obj = v8::Object::new(scope);

            let has_res_key = v8::String::new(scope, "hasRestrictions").unwrap();
            let has_res_val = v8::Boolean::new(scope, has_res);
            res_obj.set(scope, has_res_key.into(), has_res_val.into());

            let strict_env_key = v8::String::new(scope, "sandboxStrictEnv").unwrap();
            let strict_env_val = v8::Boolean::new(scope, strict_env);
            res_obj.set(scope, strict_env_key.into(), strict_env_val.into());

            // Allow rules
            let allow_rules = broker.allow_rules();
            let allow_arr = v8::Array::new(scope, allow_rules.len() as i32);
            for (i, rule) in allow_rules.iter().enumerate() {
                let rule_obj = v8::Object::new(scope);
                let k_key = v8::String::new(scope, "kind").unwrap();
                let k_val = v8::String::new(scope, rule.kind.as_str()).unwrap();
                rule_obj.set(scope, k_key.into(), k_val.into());

                let a_key = v8::String::new(scope, "action").unwrap();
                let a_val = v8::String::new(scope, rule.action.as_str()).unwrap();
                rule_obj.set(scope, a_key.into(), a_val.into());

                let r_key = v8::String::new(scope, "resource").unwrap();
                let r_val = v8::String::new(scope, &rule.resource.display_for_audit()).unwrap();
                rule_obj.set(scope, r_key.into(), r_val.into());

                allow_arr.set_index(scope, i as u32, rule_obj.into());
            }
            let allow_key = v8::String::new(scope, "allowRules").unwrap();
            res_obj.set(scope, allow_key.into(), allow_arr.into());

            // Deny rules
            let deny_rules = broker.deny_rules();
            let deny_arr = v8::Array::new(scope, deny_rules.len() as i32);
            for (i, rule) in deny_rules.iter().enumerate() {
                let rule_obj = v8::Object::new(scope);
                let k_key = v8::String::new(scope, "kind").unwrap();
                let k_val = v8::String::new(scope, rule.kind.as_str()).unwrap();
                rule_obj.set(scope, k_key.into(), k_val.into());

                let a_key = v8::String::new(scope, "action").unwrap();
                let a_val = v8::String::new(scope, rule.action.as_str()).unwrap();
                rule_obj.set(scope, a_key.into(), a_val.into());

                let r_key = v8::String::new(scope, "resource").unwrap();
                let r_val = v8::String::new(scope, &rule.resource.display_for_audit()).unwrap();
                rule_obj.set(scope, r_key.into(), r_val.into());

                deny_arr.set_index(scope, i as u32, rule_obj.into());
            }
            let deny_key = v8::String::new(scope, "denyRules").unwrap();
            res_obj.set(scope, deny_key.into(), deny_arr.into());

            rv.set(res_obj.into());
        },
    )
    .unwrap();
    let list_key = v8::String::new(scope, "list").unwrap();
    permissions_obj.set(scope, list_key.into(), list_fn.into());

    // 4. permissions.revoke(descriptor)
    let revoke_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() == 0 || !args.get(0).is_object() {
                let msg = v8::String::new(scope, "permissions.revoke requires a descriptor object")
                    .unwrap();
                let exc = v8::Exception::type_error(scope, msg);
                scope.throw_exception(exc);
                return;
            }

            let obj = args.get(0).to_object(scope).unwrap();
            let name_key = v8::String::new(scope, "name").unwrap();
            let name_val = obj
                .get(scope, name_key.into())
                .map(|v| v.to_rust_string_lossy(scope))
                .unwrap_or_else(|| "read".to_string());

            let path_key = v8::String::new(scope, "path").unwrap();
            let path_val = obj.get(scope, path_key.into()).and_then(|v| {
                if v.is_string() {
                    Some(v.to_rust_string_lossy(scope))
                } else {
                    None
                }
            });

            let host_key = v8::String::new(scope, "host").unwrap();
            let host_val = obj.get(scope, host_key.into()).and_then(|v| {
                if v.is_string() {
                    Some(v.to_rust_string_lossy(scope))
                } else {
                    None
                }
            });

            let var_key = v8::String::new(scope, "varName").unwrap();
            let var_val = obj.get(scope, var_key.into()).and_then(|v| {
                if v.is_string() {
                    Some(v.to_rust_string_lossy(scope))
                } else {
                    None
                }
            });

            let cmd_key = v8::String::new(scope, "command").unwrap();
            let cmd_val = obj.get(scope, cmd_key.into()).and_then(|v| {
                if v.is_string() {
                    Some(v.to_rust_string_lossy(scope))
                } else {
                    None
                }
            });

            let (kind, action, resource) =
                parse_descriptor_to_parts(&name_val, path_val, host_val, var_val, cmd_val);

            let mut broker = global_resource_broker().write().unwrap();
            broker.deny(kind, action, resource);
            rv.set(v8::Boolean::new(scope, true).into());
        },
    )
    .unwrap();
    let revoke_key = v8::String::new(scope, "revoke").unwrap();
    permissions_obj.set(scope, revoke_key.into(), revoke_fn.into());

    // 5. createSandboxPolicy(rules)
    let create_policy_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let policy_obj = if args.length() > 0 && args.get(0).is_object() {
                args.get(0)
            } else {
                v8::Object::new(scope).into()
            };
            rv.set(policy_obj);
        },
    )
    .unwrap();
    let cp_key = v8::String::new(scope, "createSandboxPolicy").unwrap();
    security_obj.set(scope, cp_key.into(), create_policy_fn.into());

    // 6. attenuate(parentPolicy, restrictions)
    let attenuate_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let base_obj = if args.length() > 0 && args.get(0).is_object() {
                args.get(0).to_object(scope).unwrap()
            } else {
                v8::Object::new(scope)
            };

            let rest_obj = if args.length() > 1 && args.get(1).is_object() {
                args.get(1).to_object(scope).unwrap()
            } else {
                v8::Object::new(scope)
            };

            // Merge rest_obj properties into new result object
            let merged = v8::Object::new(scope);

            if let Some(prop_names) = base_obj.get_property_names(scope, Default::default()) {
                let len = prop_names.length();
                for i in 0..len {
                    if let Some(k) = prop_names.get_index(scope, i) {
                        if let Some(v) = base_obj.get(scope, k) {
                            merged.set(scope, k, v);
                        }
                    }
                }
            }

            if let Some(prop_names) = rest_obj.get_property_names(scope, Default::default()) {
                let len = prop_names.length();
                for i in 0..len {
                    if let Some(k) = prop_names.get_index(scope, i) {
                        if let Some(v) = rest_obj.get(scope, k) {
                            merged.set(scope, k, v);
                        }
                    }
                }
            }

            rv.set(merged.into());
        },
    )
    .unwrap();
    let att_key = v8::String::new(scope, "attenuate").unwrap();
    security_obj.set(scope, att_key.into(), attenuate_fn.into());

    let perm_key = v8::String::new(scope, "permissions").unwrap();
    security_obj.set(scope, perm_key.into(), permissions_obj.into());

    // Register globally as `__bee_security`, `security`, `__bee_permissions`, `permissions`
    let global = context.global(scope);
    let sec_global_key = v8::String::new(scope, "__bee_security").unwrap();
    global.set(scope, sec_global_key.into(), security_obj.into());
    let sec_plain_key = v8::String::new(scope, "security").unwrap();
    global.set(scope, sec_plain_key.into(), security_obj.into());

    let perm_global_key = v8::String::new(scope, "__bee_permissions").unwrap();
    global.set(scope, perm_global_key.into(), permissions_obj.into());
    let perm_plain_key = v8::String::new(scope, "permissions").unwrap();
    global.set(scope, perm_plain_key.into(), permissions_obj.into());

    Ok(())
}
