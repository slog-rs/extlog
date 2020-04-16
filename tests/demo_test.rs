//! Demonstration tests for the slog test utilities.
//!
//! Copyright 2017 Metaswitch Networks
//!
//! The main test is copied verbatim from src/lib.rs because `cargo kcov` does not currently run
//! doc-tests. This should be removed once that is fixed.
//! See https://github.com/kennytm/cargo-kcov/issues/15

use serde_json::json;

use slog::debug;
use slog_extlog::slog_test;

#[test]
fn test_main() {
    // Setup code
    let mut data = iobuffer::IoBuffer::new();
    let logger = slog_test::new_test_logger(data.clone());

    // Application code
    debug!(logger, "Something happened to it";
       "subject" => "something",
       "verb"    => "happened",
       "object"  => "it");

    // Test code - parse all logs and check their contents.
    let logs = slog_test::read_json_values(&mut data);
    slog_test::assert_json_matches(&logs[0], &json!({ "subject": "something", "object": "it" }));
    assert!(logs[0]["msg"].as_str().unwrap().contains("to it"));

    // More application code
    debug!(logger, "Internal log with no log_id");
    debug!(logger, "Another log"; "log_id" => "ABC123");
    debug!(logger, "Imposter"; "log_id" => "XYZ123");

    // Alternate test code - parse selected logs and check their contents.
    let abc_logs = slog_test::logs_in_range("ABC", "ABD", &mut data);
    assert!(abc_logs.len() == 1);
    assert!(abc_logs[0]["msg"].as_str().unwrap() == "Another log");
}

#[cfg(test)]
fn assert_not_json_matches(actual: &serde_json::Value, expected: &serde_json::Value) {
    use std::panic;
    let result = panic::catch_unwind(|| slog_test::assert_json_matches(actual, expected));
    assert!(
        result.is_err(),
        "Unexpected match:\nexpected:\n{}\nbut found:\n{}",
        expected,
        actual
    );
}

#[test]
fn test_assert_json_matches() {
    let v_0 = &json!({});
    let v_a = &json!({ "a": "alpha"});
    let v_b = &json!({ "b": "42" });
    let v_ab = &json!({ "a": "alpha", "b": "42" });
    let v_ab_alt = &json!({ "a": "aleph", "b": "42" });
    let v_b_i = &json!({ "b": 42 });
    let v_b_n = &json!({ "b": null });
    let v_b_o = &json!({ "b": { "b": "42" }});
    let v_b_a = &json!({ "b": [ "b", "42"]});
    let v_nested_1 = &json!({ "a": { "aa": 42, "ab": 52 }, "b": { "bb": { "bbb": 62 }}});
    let v_nested_2 = &json!(
        { "a": { "aa": 42, "ab": 52 },
          "b": { "bb": { "bbb": 62 }, "bc": 67 },
          "c": 72 });
    let v_array_1 = &json!([99, 97, 95, [94, 93, 92, 91, []], 90]);
    let v_array_2 = &json!([99, 97, 95, [94, 93, 92, 91, ["boo"], ["ign"]], 90, "more"]);

    // simple
    slog_test::assert_json_matches(v_ab, v_ab);

    // various matches and mismatches
    slog_test::assert_json_matches(v_b, v_b);
    slog_test::assert_json_matches(v_b_i, v_b_i);
    slog_test::assert_json_matches(v_b_n, v_b_n);
    slog_test::assert_json_matches(v_b_o, v_b_o);
    slog_test::assert_json_matches(v_b_a, v_b_a);
    assert_not_json_matches(v_b_i, v_b);
    assert_not_json_matches(v_b_n, v_b);
    assert_not_json_matches(v_b_o, v_b);
    assert_not_json_matches(v_b_a, v_b);
    assert_not_json_matches(v_ab, v_ab_alt);

    // ignore additional values
    slog_test::assert_json_matches(v_ab, v_0);
    slog_test::assert_json_matches(v_ab, v_a);
    slog_test::assert_json_matches(v_ab, v_b);

    // ...but not the other way around
    assert_not_json_matches(v_0, v_ab);
    assert_not_json_matches(v_a, v_ab);
    assert_not_json_matches(v_b, v_ab);

    // nesting objects
    slog_test::assert_json_matches(v_nested_2, v_nested_1);
    assert_not_json_matches(v_nested_1, v_nested_2);

    // nesting arrays
    slog_test::assert_json_matches(v_array_2, v_array_1);
    assert_not_json_matches(v_array_1, v_array_2);
}
