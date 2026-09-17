// Tests for Beejs v1.6.0 Enterprise Capability-Based Security (`bee:security` / `bee:permissions`)

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_security_module_resolution_and_exports() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");
    let code = r#"
        const security = require('bee:security');
        const securityAlias = require('security');
        const permissions = require('bee:permissions');
        const permissionsAlias = require('permissions');

        if (typeof security.permissions.query !== 'function') throw new Error('Missing query');
        if (typeof security.permissions.has !== 'function') throw new Error('Missing has');
        if (typeof security.permissions.list !== 'function') throw new Error('Missing list');
        if (typeof security.permissions.revoke !== 'function') throw new Error('Missing revoke');
        if (typeof security.createSandboxPolicy !== 'function') throw new Error('Missing createSandboxPolicy');
        if (typeof security.attenuate !== 'function') throw new Error('Missing attenuate');

        if (security !== securityAlias) throw new Error('Security alias mismatch');
        if (permissions !== permissionsAlias) throw new Error('Permissions alias mismatch');
        if (security.permissions !== permissions) throw new Error('Permissions property mismatch');

        'OK';
    "#;
    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("OK"));
}

#[test]
#[serial]
fn test_permission_query_and_has() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");
    let code = r#"
        const { permissions } = require('bee:security');

        // Query read permission
        const readQuery = permissions.query({ name: 'read', path: '/tmp/test.txt' });
        if (readQuery.state !== 'granted') throw new Error('Expected granted, got: ' + readQuery.state);
        if (!permissions.has({ name: 'read', path: '/tmp/test.txt' })) {
            throw new Error('Expected has() to return true');
        }

        // Query network permission
        const netQuery = permissions.query({ name: 'net', host: 'api.example.com' });
        if (netQuery.state !== 'granted') throw new Error('Expected net granted');

        // Query env permission
        const envQuery = permissions.query({ name: 'env', varName: 'PATH' });
        if (envQuery.state !== 'granted') throw new Error('Expected env granted');

        JSON.stringify({ queryPassed: true });
    "#;
    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"queryPassed\":true"));
}

#[test]
#[serial]
fn test_permission_list_and_dynamic_revocation() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");
    let code = r#"
        const { permissions } = require('bee:security');

        // List initial rules
        const initial = permissions.list();
        if (typeof initial.hasRestrictions !== 'boolean') throw new Error('Missing hasRestrictions');
        if (!Array.isArray(initial.allowRules)) throw new Error('allowRules should be array');
        if (!Array.isArray(initial.denyRules)) throw new Error('denyRules should be array');

        // Revoke read permission on a sensitive path
        permissions.revoke({ name: 'read', path: '/etc/shadow' });

        // Query after revocation
        const revokedQuery = permissions.query({ name: 'read', path: '/etc/shadow' });
        if (revokedQuery.state !== 'denied') {
            throw new Error('Expected denied after revoke, got: ' + revokedQuery.state);
        }
        if (permissions.has({ name: 'read', path: '/etc/shadow' })) {
            throw new Error('has() should return false after revoke');
        }

        // List updated rules
        const updated = permissions.list();
        if (!updated.hasRestrictions) throw new Error('hasRestrictions should now be true');
        const found = updated.denyRules.some(r => r.kind === 'FileSystem' && r.action === 'Read');
        if (!found) throw new Error('Deny rule not found in denyRules');

        JSON.stringify({ revokeSuccess: true, denyCount: updated.denyRules.length });
    "#;
    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"revokeSuccess\":true"));
}

#[test]
#[serial]
fn test_policy_creation_and_attenuation() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");
    let code = r#"
        const { createSandboxPolicy, attenuate } = require('bee:security');

        const parentPolicy = createSandboxPolicy({
            allowNet: ['api.openai.com', 'api.github.com'],
            allowRead: ['/app', '/tmp'],
            maxMemoryMb: 512,
            timeoutMs: 10000,
        });

        if (parentPolicy.allowNet.length !== 2) throw new Error('Parent policy net mismatch');

        // Attenuate policy for an untrusted sub-task: disable network and reduce timeout
        const childPolicy = attenuate(parentPolicy, {
            allowNet: [], // drop all network
            timeoutMs: 3000, // tighten timeout
            allowWrite: ['/tmp/scratch'],
        });

        if (childPolicy.allowNet.length !== 0) throw new Error('Child policy should have 0 allowed hosts');
        if (childPolicy.timeoutMs !== 3000) throw new Error('Child timeout should be 3000ms');
        if (childPolicy.maxMemoryMb !== 512) throw new Error('Child should inherit maxMemoryMb');
        if (childPolicy.allowWrite[0] !== '/tmp/scratch') throw new Error('Child should include allowWrite');

        JSON.stringify({ attenuationPassed: true });
    "#;
    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"attenuationPassed\":true"));
}
