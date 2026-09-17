// URLSearchParams API tests - v0.3.353
// Tests for Web standard URLSearchParams constructor and methods

#[cfg(test)]
mod url_search_params_tests {
    use amberjs::runtime_minimal::MinimalRuntime;
    use serial_test::serial;

    // v0.3.353: Basic constructor tests
    #[test]
    #[serial]
    fn test_url_search_params_constructor_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            typeof URLSearchParams;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "function",
            "Expected URLSearchParams to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial]
    fn test_url_search_params_empty_constructor() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams();
            params.toString();
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(output, "", "Expected empty query string, got: {}", output);
    }

    #[test]
    #[serial]
    fn test_url_search_params_from_string() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar&baz=qux');
            params.toString();
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // Should contain both parameters
        assert!(
            output.contains("foo=bar") && output.contains("baz=qux"),
            "Expected query string with foo=bar and baz=qux, got: {}",
            output
        );
    }

    #[test]
    #[serial]
    fn test_url_search_params_from_object() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams({ foo: 'bar', baz: 'qux' });
            params.toString();
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // Should contain both parameters (order may vary)
        assert!(
            output.contains("foo=bar") && output.contains("baz=qux"),
            "Expected query string with foo=bar and baz=qux, got: {}",
            output
        );
    }

    #[test]
    #[serial]
    fn test_url_search_params_from_sequence_pairs_and_copy_constructor() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const fromSequence = new URLSearchParams([
                ['foo', 'bar'],
                ['foo', 'baz'],
                ['qux', 'quux']
            ]);
            const copied = new URLSearchParams(fromSequence);
            fromSequence.append('foo', 'late');
            `${copied.getAll('foo').join('|')}:${copied.get('qux')}:${fromSequence.getAll('foo').join('|')}`;
        "#,
        );

        assert!(
            result.is_ok(),
            "Expected URLSearchParams sequence/copy constructor script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "bar|baz:quux:bar|baz|late",
            "Expected sequence init and copy constructor to preserve pairs independently, got: {output}"
        );
    }

    #[test]
    #[serial]
    fn test_url_search_params_decodes_plus_and_serializes_space_as_plus() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const parsed = new URLSearchParams('q=bee+runtime&plus=a%2Bb');
            const built = new URLSearchParams({ q: 'bee runtime', plus: 'a+b' });
            `${parsed.get('q')}:${parsed.get('plus')}:${built.toString()}`;
        "#,
        );

        assert!(
            result.is_ok(),
            "Expected URLSearchParams plus/space script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "bee runtime:a+b:q=bee+runtime&plus=a%2Bb",
            "Expected + to decode as space and spaces to serialize as +, got: {output}"
        );
    }

    #[test]
    #[serial]
    fn test_url_search_params_from_iterable_map_pairs() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams(new Map([
                ['topic', 'runtime'],
                ['space', 'bee js']
            ]));
            `${params.get('topic')}:${params.toString()}`;
        "#,
        );

        assert!(
            result.is_ok(),
            "Expected URLSearchParams Map iterable constructor script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "runtime:topic=runtime&space=bee+js",
            "Expected Map iterable pairs to initialize URLSearchParams in order, got: {output}"
        );
    }

    #[test]
    #[serial]
    fn test_url_search_params_rejects_invalid_sequence_pairs() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const cases = [
                () => new URLSearchParams([['name']]),
                () => new URLSearchParams([['name', 'bee', 'extra']]),
                () => new URLSearchParams(['not-a-pair'])
            ];
            cases.map((run) => {
                try {
                    run();
                    return 'accepted';
                } catch (error) {
                    return error instanceof TypeError ? 'type-error' : String(error);
                }
            }).join('|');
        "#,
        );

        assert!(
            result.is_ok(),
            "Expected invalid URLSearchParams pair script to catch errors, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "type-error|type-error|type-error",
            "Expected malformed sequence pairs to throw TypeError, got: {output}"
        );
    }

    // v0.3.353: append() method tests
    #[test]
    #[serial]
    fn test_url_search_params_append() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams();
            params.append('foo', 'bar');
            params.append('foo', 'baz');
            params.toString();
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // Should contain foo=bar and foo=baz (multiple values)
        assert!(
            output.contains("foo=bar") && output.contains("foo=baz"),
            "Expected query string with multiple foo values, got: {}",
            output
        );
    }

    // v0.3.353: delete() method tests
    #[test]
    #[serial]
    fn test_url_search_params_delete() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar&baz=qux');
            params.delete('foo');
            params.toString();
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // Should only contain baz=qux
        assert!(
            !output.contains("foo=bar") && output.contains("baz=qux"),
            "Expected query string without foo, got: {}",
            output
        );
    }

    // v0.3.353: get() method tests
    #[test]
    #[serial]
    fn test_url_search_params_get() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar&foo=baz&qux=quux');
            params.get('foo');
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // get() returns first value
        assert_eq!(output, "bar", "Expected first value 'bar', got: {}", output);
    }

    #[test]
    #[serial]
    fn test_url_search_params_get_nonexistent() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar');
            params.get('nonexistent');
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "null",
            "Expected null for nonexistent key, got: {}",
            output
        );
    }

    // v0.3.353: getAll() method tests
    #[test]
    #[serial]
    fn test_url_search_params_get_all() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar&foo=baz&qux=quux');
            params.getAll('foo').toString();
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // getAll() returns array of all values
        assert!(
            output.contains("bar") && output.contains("baz"),
            "Expected array with bar and baz, got: {}",
            output
        );
    }

    // v0.3.353: has() method tests
    #[test]
    #[serial]
    fn test_url_search_params_has() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar');
            params.has('foo').toString();
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "true",
            "Expected true for existing key, got: {}",
            output
        );
    }

    #[test]
    #[serial]
    fn test_url_search_params_has_nonexistent() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar');
            params.has('nonexistent').toString();
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "false",
            "Expected false for nonexistent key, got: {}",
            output
        );
    }

    // v0.3.353: set() method tests
    #[test]
    #[serial]
    fn test_url_search_params_set() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar&foo=baz');
            params.set('foo', 'qux');
            params.toString();
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // set() replaces all values with single value
        assert!(
            output.contains("foo=qux")
                && !output.contains("foo=bar")
                && !output.contains("foo=baz"),
            "Expected query string with single foo=qux, got: {}",
            output
        );
    }

    #[test]
    #[serial]
    fn test_url_search_params_are_live_with_url_search_and_href() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const url = new URL('https://example.com/path?a=1#hash');
            const params = url.searchParams;
            params.set('a', '2');
            params.append('b', '3');
            const afterParamsMutation =
              url.search + '|' + url.href + '|' + params.toString();
            url.search = '?c=4';
            const afterSearchMutation =
              url.search + '|' + url.href + '|' + params.get('a') + '|' + params.get('c');
            afterParamsMutation + '||' + afterSearchMutation;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output,
            "?a=2&b=3|https://example.com/path?a=2&b=3#hash|a=2&b=3||?c=4|https://example.com/path?c=4#hash|null|4",
            "Expected URL.searchParams to stay live with URL.search and href, got: {}",
            output
        );
    }

    // v0.3.353: toString() method tests
    #[test]
    #[serial]
    fn test_url_search_params_to_string() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar&baz=qux');
            typeof params.toString;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "function",
            "Expected toString to be a function, got: {}",
            output
        );
    }

    // v0.3.353: forEach() method tests
    #[test]
    #[serial]
    fn test_url_search_params_for_each() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar&baz=qux');
            let keys = [];
            params.forEach((value, key) => keys.push(key));
            keys.sort().toString();
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // Should iterate over both keys
        assert!(
            output.contains("baz") && output.contains("foo"),
            "Expected keys to contain baz and foo, got: {}",
            output
        );
    }

    #[test]
    #[serial]
    fn test_url_search_params_for_each_uses_this_arg_and_owner() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar&foo=baz');
            const seen = [];
            params.forEach(function(value, key, owner) {
                seen.push(`${this.label}:${key}=${value}:${owner === params}`);
            }, { label: 'ctx' });
            seen.join('|');
        "#,
        );

        assert!(
            result.is_ok(),
            "Expected URLSearchParams forEach thisArg script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "ctx:foo=bar:true|ctx:foo=baz:true",
            "Expected URLSearchParams.forEach to honor thisArg and owner, got: {output}"
        );
    }

    // v0.3.353: entries() iterator tests
    #[test]
    #[serial]
    fn test_url_search_params_entries() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar');
            const entries = params.entries();
            const first = entries.next().value;
            first ? first[0] + '=' + first[1] : '';
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "foo=bar",
            "Expected first entry to be foo=bar, got: {}",
            output
        );
    }

    #[test]
    #[serial]
    fn test_url_search_params_iterators_are_iterable_and_direct_iteration_uses_entries() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar&foo=baz&qux=quux');
            const entryIterator = params.entries();
            const entries = Array.from(entryIterator).map(([key, value]) => `${key}=${value}`).join('|');
            const direct = Array.from(params).map(([key, value]) => `${key}=${value}`).join('|');
            const looped = [];
            for (const [key, value] of params) {
                looped.push(`${key}=${value}`);
            }
            const keys = Array.from(params.keys()).join('|');
            const values = Array.from(params.values()).join('|');
            `${typeof entryIterator[Symbol.iterator]}:${entries}:${direct}:${looped.join('|')}:${keys}:${values}`;
        "#,
        );

        assert!(
            result.is_ok(),
            "Expected URLSearchParams iterable script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output,
            "function:foo=bar|foo=baz|qux=quux:foo=bar|foo=baz|qux=quux:foo=bar|foo=baz|qux=quux:foo|foo|qux:bar|baz|quux",
            "Expected URLSearchParams iterators and direct iteration to expose entries in order, got: {output}"
        );
    }

    // v0.3.353: keys() iterator tests
    #[test]
    #[serial]
    fn test_url_search_params_keys() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar&baz=qux');
            const keys = params.keys();
            const first = keys.next().value;
            first || '';
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // First key should be either 'foo' or 'baz'
        assert!(
            output == "foo" || output == "baz",
            "Expected first key to be foo or baz, got: {}",
            output
        );
    }

    // v0.3.353: values() iterator tests
    #[test]
    #[serial]
    fn test_url_search_params_values() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('foo=bar&baz=qux');
            const values = params.values();
            const first = values.next().value;
            first || '';
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // First value should be either 'bar' or 'qux'
        assert!(
            output == "bar" || output == "qux",
            "Expected first value to be bar or qux, got: {}",
            output
        );
    }

    // v0.3.353: URL encoding tests
    #[test]
    #[serial]
    fn test_url_search_params_encodes_special_chars() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams({ foo: 'bar baz' });
            params.toString();
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // Should URL-encode spaces as %20 or +
        assert!(
            output.contains("foo=") && output.contains("%20") || output.contains("foo=bar+baz"),
            "Expected encoded query string with space, got: {}",
            output
        );
    }

    // v0.3.353: Sort() method tests
    #[test]
    #[serial]
    fn test_url_search_params_sort() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const params = new URLSearchParams('z=1&a=2&m=3');
            params.sort();
            params.toString();
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // Should be sorted: a=2&m=3&z=1
        assert_eq!(
            output, "a=2&m=3&z=1",
            "Expected sorted query string, got: {}",
            output
        );
    }
}
