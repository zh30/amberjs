//! Lightweight assertion and testing helpers (`amber:std/assert`).

pub const ASSERT_JS_CODE: &str = r#"
(function() {
    function formatValue(v) {
        try {
            return JSON.stringify(v);
        } catch {
            return String(v);
        }
    }

    function deepEqual(a, b) {
        if (a === b) return true;
        if (typeof a !== typeof b) return false;
        if (a === null || b === null || typeof a !== 'object') return false;

        if (Array.isArray(a) !== Array.isArray(b)) return false;

        if (Array.isArray(a)) {
            if (a.length !== b.length) return false;
            for (let i = 0; i < a.length; i++) {
                if (!deepEqual(a[i], b[i])) return false;
            }
            return true;
        }

        const keysA = Object.keys(a);
        const keysB = Object.keys(b);
        if (keysA.length !== keysB.length) return false;

        for (const k of keysA) {
            if (!Object.prototype.hasOwnProperty.call(b, k)) return false;
            if (!deepEqual(a[k], b[k])) return false;
        }
        return true;
    }

    function assert(condition, message = 'Assertion failed') {
        if (!condition) {
            throw new Error(message);
        }
    }

    function assertEquals(actual, expected, message = null) {
        if (!deepEqual(actual, expected)) {
            const msg = message || `Expected ${formatValue(expected)}, but got ${formatValue(actual)}`;
            throw new Error(msg);
        }
    }

    function assertNotEquals(actual, expected, message = null) {
        if (deepEqual(actual, expected)) {
            const msg = message || `Expected values not to equal: ${formatValue(actual)}`;
            throw new Error(msg);
        }
    }

    function assertThrows(fn, expectedError = null, message = null) {
        let threw = false;
        let thrownError = null;
        try {
            fn();
        } catch (e) {
            threw = true;
            thrownError = e;
        }

        if (!threw) {
            throw new Error(message || 'Expected function to throw, but it succeeded');
        }

        if (expectedError) {
            if (typeof expectedError === 'string') {
                assert(String(thrownError).includes(expectedError), `Error message does not contain: "${expectedError}"`);
            } else if (expectedError instanceof RegExp) {
                assert(expectedError.test(String(thrownError)), `Error message does not match regex: ${expectedError}`);
            }
        }
    }

    globalThis.__amber_assert = {
        assert,
        assertEquals,
        assertNotEquals,
        assertThrows
    };
})();
"#;
