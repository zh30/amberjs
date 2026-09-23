// Stage 93 Phase 3.4 - Enhanced Assertion Matchers Tests
// Comprehensive test suite for new assertion matchers
//
// New matchers added:
// - toBeTruthy / toBeFalsy: Type-aware truthy/falsy checks
// - toThrow: Error throwing validation
// - toHaveProperty: Object property existence and value checks
// - toBeInstanceOf: Type instance checking
// - toMatchObject: Partial object matching
// - toStrictEqual: Strict equality checking
// - toBeGreaterThanOrEqual / toBeLessThanOrEqual: Boundary comparisons

use amberjs::testing::assertions::*;

/// Test toBeTruthy matcher - basic values
#[test]
fn test_to_be_truthy_basic() {
    let matcher = ExtendedMatcher::to_be_truthy();
    assert!(matcher.matches(&serde_json::json!(true)));
    assert!(matcher.matches(&serde_json::json!(1)));
    assert!(matcher.matches(&serde_json::json!("hello")));
    assert!(matcher.matches(&serde_json::json!("false")));
    assert!(matcher.matches(&serde_json::json!("0")));
    assert!(matcher.matches(&serde_json::json!([1, 2, 3])));
    assert!(matcher.matches(&serde_json::json!([])));
    assert!(matcher.matches(&serde_json::json!({"key": "value"})));
    assert!(matcher.matches(&serde_json::json!({})));
}

/// Test toBeTruthy matcher - falsy values should fail
#[test]
fn test_to_be_truthy_falsy_fail() {
    let matcher = ExtendedMatcher::to_be_truthy();
    assert!(!matcher.matches(&serde_json::json!(false)));
    assert!(!matcher.matches(&serde_json::json!(0)));
    assert!(!matcher.matches(&serde_json::json!("")));
    assert!(!matcher.matches(&serde_json::json!(null)));
}

/// Test toBeFalsy matcher
#[test]
fn test_to_be_falsy() {
    let matcher = ExtendedMatcher::to_be_falsy();
    assert!(matcher.matches(&serde_json::json!(false)));
    assert!(matcher.matches(&serde_json::json!(0)));
    assert!(matcher.matches(&serde_json::json!("")));
    assert!(matcher.matches(&serde_json::json!(null)));
}

/// Test toBeFalsy matcher - truthy values should fail
#[test]
fn test_to_be_falsy_truthy_fail() {
    let matcher = ExtendedMatcher::to_be_falsy();
    assert!(!matcher.matches(&serde_json::json!(true)));
    assert!(!matcher.matches(&serde_json::json!(1)));
    assert!(!matcher.matches(&serde_json::json!("hello")));
    assert!(!matcher.matches(&serde_json::json!("false")));
    assert!(!matcher.matches(&serde_json::json!("0")));
    assert!(!matcher.matches(&serde_json::json!([])));
    assert!(!matcher.matches(&serde_json::json!({})));
}

#[test]
fn to_be_truthy_treats_empty_array_and_object_as_truthy() {
    let to_be_truthy = ExtendedMatcher::to_be_truthy();
    let to_be_falsy = ExtendedMatcher::to_be_falsy();
    let truthy = ExtendedMatcher::new_truthy();
    let falsy = ExtendedMatcher::new_falsy();

    for value in [serde_json::json!([]), serde_json::json!({})] {
        assert!(
            to_be_truthy.matches(&value),
            "toBeTruthy should treat {value:?} as truthy"
        );
        assert!(
            truthy.matches(&value),
            "Truthy matcher should treat {value:?} as truthy"
        );
        assert!(
            !to_be_falsy.matches(&value),
            "toBeFalsy should not treat {value:?} as falsy"
        );
        assert!(
            !falsy.matches(&value),
            "Falsy matcher should not treat {value:?} as falsy"
        );
    }
}

/// Test toThrow matcher with error message
#[test]
fn test_to_throw_with_message() {
    let matcher = ExtendedMatcher::to_throw(Some("Error message".to_string()));
    // Test with a JSON object that represents a thrown error
    let error_json = serde_json::json!({"thrown": true, "message": "Error message occurred"});
    assert!(matcher.matches(&error_json));
}

/// Test toThrow matcher without message (any throw)
#[test]
fn test_to_throw_any() {
    let matcher = ExtendedMatcher::to_throw(None);
    let error_json = serde_json::json!({"thrown": true, "message": "any error"});
    assert!(matcher.matches(&error_json));
}

/// Test toHaveProperty matcher - existing property
#[test]
fn test_to_have_property_exists() {
    let matcher = ExtendedMatcher::to_have_property("name".to_string(), None);
    let obj = serde_json::json!({"name": "John", "age": 30});
    assert!(matcher.matches(&obj));
}

/// Test toHaveProperty matcher - property with specific value
#[test]
fn test_to_have_property_with_value() {
    let matcher =
        ExtendedMatcher::to_have_property("name".to_string(), Some(serde_json::json!("John")));
    let obj = serde_json::json!({"name": "John", "age": 30});
    assert!(matcher.matches(&obj));
}

/// Test toHaveProperty matcher - property with wrong value
#[test]
fn test_to_have_property_wrong_value() {
    let matcher =
        ExtendedMatcher::to_have_property("name".to_string(), Some(serde_json::json!("Jane")));
    let obj = serde_json::json!({"name": "John", "age": 30});
    assert!(!matcher.matches(&obj));
}

/// Test toHaveProperty matcher - non-existing property
#[test]
fn test_to_have_property_not_exists() {
    let matcher = ExtendedMatcher::to_have_property("email".to_string(), None);
    let obj = serde_json::json!({"name": "John", "age": 30});
    assert!(!matcher.matches(&obj));
}

/// Test toHaveProperty matcher - nested property
#[test]
fn test_to_have_property_nested() {
    let matcher = ExtendedMatcher::to_have_property("address.city".to_string(), None);
    let obj = serde_json::json!({
        "name": "John",
        "address": {"city": "NYC", "country": "USA"}
    });
    assert!(matcher.matches(&obj));
}

/// Test toBeInstanceOf matcher with String
#[test]
fn test_to_be_instance_of_string() {
    let matcher = ExtendedMatcher::to_be_instance_of("String".to_string());
    assert!(matcher.matches(&serde_json::json!("hello")));
    assert!(!matcher.matches(&serde_json::json!(42)));
}

/// Test toBeInstanceOf with Number
#[test]
fn test_to_be_instance_of_number() {
    let matcher = ExtendedMatcher::to_be_instance_of("Number".to_string());
    assert!(matcher.matches(&serde_json::json!(42)));
    assert!(!matcher.matches(&serde_json::json!("hello")));
}

/// Test toBeInstanceOf with Array
#[test]
fn test_to_be_instance_of_array() {
    let matcher = ExtendedMatcher::to_be_instance_of("Array".to_string());
    assert!(matcher.matches(&serde_json::json!([1, 2, 3])));
    assert!(!matcher.matches(&serde_json::json!("hello")));
}

/// Test toMatchObject matcher - partial match
#[test]
fn test_to_match_object_partial() {
    let pattern = serde_json::json!({"name": "John", "age": 30});
    let value = serde_json::json!({"name": "John", "age": 30, "email": "john@example.com"});
    let matcher = ExtendedMatcher::to_match_object(pattern);
    assert!(matcher.matches(&value));
}

/// Test toMatchObject matcher - mismatch
#[test]
fn test_to_match_object_mismatch() {
    let pattern = serde_json::json!({"name": "John"});
    let value = serde_json::json!({"name": "Jane", "age": 30});
    let matcher = ExtendedMatcher::to_match_object(pattern);
    assert!(!matcher.matches(&value));
}

/// Test toMatchObject matcher - nested objects
#[test]
fn test_to_match_object_nested() {
    let pattern = serde_json::json!({"address": {"city": "NYC"}});
    let value = serde_json::json!({
        "name": "John",
        "address": {"city": "NYC", "country": "USA"}
    });
    let matcher = ExtendedMatcher::to_match_object(pattern);
    assert!(matcher.matches(&value));
}

/// Test toStrictEqual matcher
#[test]
fn test_to_strict_equal() {
    let matcher = ExtendedMatcher::to_strict_equal(serde_json::json!(42));
    assert!(matcher.matches(&serde_json::json!(42)));
}

/// Test toStrictEqual matcher - type mismatch
#[test]
fn test_to_strict_equal_type_mismatch() {
    let matcher = ExtendedMatcher::to_strict_equal(serde_json::json!("42"));
    assert!(!matcher.matches(&serde_json::json!(42)));
}

/// Test toStrictEqual matcher - array strictness
#[test]
fn test_to_strict_equal_array() {
    let matcher = ExtendedMatcher::to_strict_equal(serde_json::json!([1, 2, 3]));
    assert!(matcher.matches(&serde_json::json!([1, 2, 3])));
    // Different from toEqual - null doesn't match empty array
    assert!(!matcher.matches(&serde_json::json!(null)));
}

/// Test toBeGreaterThanOrEqual matcher
#[test]
fn test_to_be_greater_than_or_equal() {
    let matcher = ExtendedMatcher::to_be_greater_than_or_equal(serde_json::json!(10));
    assert!(matcher.matches(&serde_json::json!(15)));
    assert!(matcher.matches(&serde_json::json!(10)));
    assert!(!matcher.matches(&serde_json::json!(5)));
}

/// Test toBeLessThanOrEqual matcher
#[test]
fn test_to_be_less_than_or_equal() {
    let matcher = ExtendedMatcher::to_be_less_than_or_equal(serde_json::json!(10));
    assert!(matcher.matches(&serde_json::json!(5)));
    assert!(matcher.matches(&serde_json::json!(10)));
    assert!(!matcher.matches(&serde_json::json!(15)));
}

/// Test GreaterThan matcher - use Value type for consistency
#[test]
fn test_greater_than() {
    let matcher = ExtendedMatcher::GreaterThan(serde_json::json!(10));
    assert!(matcher.matches(&serde_json::json!(15)));
    assert!(!matcher.matches(&serde_json::json!(10)));
    assert!(!matcher.matches(&serde_json::json!(5)));
}

/// Test LessThan matcher - use Value type for consistency
#[test]
fn test_less_than() {
    let matcher = ExtendedMatcher::LessThan(serde_json::json!(10));
    assert!(matcher.matches(&serde_json::json!(5)));
    assert!(!matcher.matches(&serde_json::json!(10)));
    assert!(!matcher.matches(&serde_json::json!(15)));
}

/// Test message generation for new matchers
#[test]
fn test_matcher_messages() {
    let truthy_msg = ExtendedMatcher::to_be_truthy().message(&serde_json::json!(false));
    assert!(truthy_msg.contains("truthy"));

    let falsy_msg = ExtendedMatcher::to_be_falsy().message(&serde_json::json!(true));
    assert!(falsy_msg.contains("falsy"));

    let throw_msg =
        ExtendedMatcher::to_throw(Some("error".to_string())).message(&serde_json::json!({}));
    assert!(throw_msg.contains("throw"));

    let property_msg =
        ExtendedMatcher::to_have_property("key".to_string(), None).message(&serde_json::json!({}));
    assert!(property_msg.contains("property"));

    let instance_msg =
        ExtendedMatcher::to_be_instance_of("String".to_string()).message(&serde_json::json!(42));
    assert!(instance_msg.contains("instance"));

    let match_obj_msg =
        ExtendedMatcher::to_match_object(serde_json::json!({})).message(&serde_json::json!({}));
    assert!(match_obj_msg.contains("match"));

    let strict_msg =
        ExtendedMatcher::to_strict_equal(serde_json::json!(1)).message(&serde_json::json!(2));
    assert!(strict_msg.contains("strict"));
}

/// Test matcher chaining through AssertionContext
#[test]
fn test_assertion_context_chain() {
    let context = AssertionContext::new(42);
    // to_equal is already available
    assert!(context.to_equal(&42));
}
