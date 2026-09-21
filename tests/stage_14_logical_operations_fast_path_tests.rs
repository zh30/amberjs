// Stage 14: 逻辑运算符快路径优化测试
// Tests for logical operations fast path optimization (&&, ||, !, ??, ?.)

#[cfg(test)]
mod logical_operations_fast_path_tests {
    use amberjs::Runtime;

    #[test]
    fn test_logical_not_true() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("!true").unwrap();
        assert_eq!(result, "false");
    }

    #[test]
    fn test_logical_not_false() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("!false").unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_logical_not_null() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("!null").unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_logical_not_undefined() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("!undefined").unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_logical_not_zero() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("!0").unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_logical_not_one() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("!1").unwrap();
        assert_eq!(result, "false");
    }

    #[test]
    fn test_logical_not_empty_string() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("!\"\"").unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_logical_not_non_empty_string() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("!\"hello\"").unwrap();
        assert_eq!(result, "false");
    }

    #[test]
    fn test_logical_and_both_true() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("true && true").unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_logical_and_one_false() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("true && false").unwrap();
        assert_eq!(result, "false");
    }

    #[test]
    fn test_logical_and_both_false() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("false && false").unwrap();
        assert_eq!(result, "false");
    }

    #[test]
    fn test_logical_or_both_true() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("true || true").unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_logical_or_one_true() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("true || false").unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_logical_or_both_false() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("false || false").unwrap();
        assert_eq!(result, "false");
    }

    #[test]
    fn test_nullish_coalescing_null() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("null ?? \"default\"").unwrap();
        assert_eq!(result, "default");
    }

    #[test]
    fn test_nullish_coalescing_undefined() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("undefined ?? \"default\"").unwrap();
        assert_eq!(result, "default");
    }

    #[test]
    fn test_nullish_coalescing_not_nullish() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("\"value\" ?? \"default\"").unwrap();
        assert_eq!(result, "value");
    }

    #[test]
    fn test_optional_chaining_null() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("null?.prop").unwrap();
        assert_eq!(result, "undefined");
    }

    #[test]
    fn test_optional_chaining_undefined() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("undefined?.prop").unwrap();
        assert_eq!(result, "undefined");
    }

    #[test]
    fn test_optional_chaining_existing_property() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("({a: 1})?.a").unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_optional_chaining_non_existing_property() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("({a: 1})?.b").unwrap();
        assert_eq!(result, "undefined");
    }

    #[test]
    fn test_complex_logical_expression() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("true && false || true").unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_nested_logical_not() {
        let runtime = Runtime::new(8 * 1024 * 1024, 64 * 1024 * 1024, false, false);
        let result = runtime.execute_code("!!true").unwrap();
        assert_eq!(result, "true");
    }
}
