//!
//! Copyright 2017 Metaswitch Networks
//!
//! Helper functions for use when testing slog logs.
//!
//! This module is a grab-bag of tools that have been found useful when testing slog logs.
//! A typical test will create a new `iobuffer::IoBuffer`, pass it to `new_test_logger` to
//! construct a logger, and pass that logger to the system under test. It will then exercise
//! the system. Finally, it will use `logs_in_range` to extract the logs it is interested in
//! in a canonical order, and test that they are as expected using standard Rust tools.
//!
//! # Example
//! ```
//! use slog::debug;
//! use slog_extlog::slog_test;
//!
//! // Setup code
//! let mut data = iobuffer::IoBuffer::new();
//! let logger = slog_test::new_test_logger(data.clone());
//!
//! // Application code
//! debug!(logger, "Something happened to it";
//!        "subject" => "something",
//!        "verb"    => "happened",
//!        "object"  => "it");
//!
//! // Test code - parse all logs and check their contents.
//! let logs = slog_test::read_json_values(&mut data);
//! slog_test::assert_json_matches(&logs[0], &serde_json::json!({ "subject": "something", "object": "it" }));
//! assert!(logs[0]["msg"].as_str().unwrap().contains("to it"));
//!
//! // More application code
//! debug!(logger, "Another log"; "log_id" => "ABC123");
//! debug!(logger, "Imposter"; "log_id" => "XYZ123");
//!
//! // Alternate test code - parse selected logs and check their contents.
//! let abc_logs = slog_test::logs_in_range("ABC", "ABD", &mut data);
//! assert_eq!(abc_logs.len(), 1);
//! assert_eq!(abc_logs[0]["msg"].as_str().unwrap(), "Another log".to_string());
//! ```
//!
//! # Statistics testing
//!
//! For verifying statistics, the `create_logger_buffer` and `check_expected_stats` methods
//! are useful for creating a `slog_extlog::StatisticsLogger` and then verifying that statistics
//! are generated as expected.
#![allow(clippy::float_cmp)]
pub use super::stats::*;

use slog::o;
#[allow(unused_imports)] // we need this trait for lines()
use std::io::BufRead;
use std::sync::Mutex;

/// Buffer containing log data.
pub type Buffer = iobuffer::IoBuffer;

/// Create a new test logger suitable for use with `read_json_values`.
pub fn new_test_logger(data: Buffer) -> slog::Logger {
    slog::Logger::root(
        slog::Fuse::new(Mutex::new(slog_json::Json::default(data))),
        o!(),
    )
}

/// Read all the newline-delimited JSON objects from the given stream,
/// panicking if there is an IO error or a JSON parse error.
/// No attempt is made to avoid reading partial lines from the stream.
pub fn read_json_values(data: &mut Buffer) -> Vec<serde_json::Value> {
    let iter = data.lines().map(move |line| {
        serde_json::from_slice::<serde_json::Value>(&line).expect("JSON parse error")
    });
    iter.collect()
}

/// Test whether the given log lies in the given range: between
/// `min_id` (inclusive) and `max_id` (exclusive).
pub fn log_in_range(min_id: &str, max_id: &str, log: &serde_json::Value) -> bool {
    match log["log_id"].as_str() {
        Some(log_id) => log_id >= min_id && log_id < max_id,
        None => false,
    }
}

/// Collect all logs of the indicated type (see `log_in_range`) and sort them in
/// ascending order of log_id.
pub fn logs_in_range(min_id: &str, max_id: &str, data: &mut Buffer) -> Vec<serde_json::Value> {
    let mut v = read_json_values(data)
        .into_iter()
        .filter(|log| log_in_range(min_id, max_id, log))
        .collect::<Vec<_>>(); // LCOV_EXCL_LINE kcov bug?
    v.sort_by_key(|log| log["log_id"].as_str().map(ToString::to_string));
    v
}

/// Assert that every item contained in `expected` also appears in `actual`.
/// Additional values in `actual` are ignored.
pub fn assert_json_matches(actual: &serde_json::Value, expected: &serde_json::Value) {
    fn check(
        actual: &serde_json::Value,
        expected: &serde_json::Value,
        left: &serde_json::Value,
        right: &serde_json::Value,
        path: &str,
    ) {
        if left.is_object() && right.is_object() {
            for (key, value) in right.as_object().unwrap().iter() {
                let path = format!("{}.{}", path, key);
                check(actual, expected, &left[key], value, &path);
            }
        } else if left.is_array() && right.is_array() {
            for (index, value) in right.as_array().unwrap().iter().enumerate() {
                let path = format!("{}.{}", path, index);
                check(actual, expected, &left[index], value, &path);
            }
        } else {
            assert!(
                left == right,
                "Mismatch at {}:\nexpected:\n{}\nbut found:\n{}",
                path,
                expected,
                actual
            );
        }
    }
    check(actual, expected, actual, expected, "");
}

/// A default logging interval for tests, short so UTs run faster.
pub static TEST_LOG_INTERVAL: u64 = 5;

/// Common setup function.
///
/// Creates a logger using the provided statistics and buffer so we can easily
/// view the generated logs.
pub fn create_logger_buffer(
    stats: StatDefinitions,
) -> (StatisticsLogger<DefaultStatisticsLogFormatter>, Buffer) {
    let data = iobuffer::IoBuffer::new();
    let logger = new_test_logger(data.clone());

    let logger = StatsLoggerBuilder::<DefaultStatisticsLogFormatter>::default()
        .with_log_interval(TEST_LOG_INTERVAL)
        .with_stats(vec![stats])
        .fuse(logger);
    (logger, data)
}

/// An expected statistic helper method.
pub struct ExpectedStat {
    /// Stat name E.g. "test_bucket_counter_grouped_freq"
    pub stat_name: &'static str,
    /// Stat tag E.g. Some("name=name,error=3,bucket=-5")
    pub tag: Option<&'static str>,
    /// Value of this stat E.g. 0f64
    pub value: f64,
    /// Type of metric for this stat E.g. "bucket_counter"
    pub metric_type: &'static str,
}

/// Asserts that a set of logs (retrieved using `logs_in_range)` is exactly equal to an
/// expected set of stats.
///
/// Particularly useful for grouped stats.
pub fn check_expected_stats(logs: &[serde_json::Value], mut expected_stats: Vec<ExpectedStat>) {
    for log in logs {
        let mut matched = None;
        for (id, exp) in expected_stats.iter().enumerate() {
            if log["name"] == exp.stat_name
                && (exp.tag.is_none() || log["tags"] == exp.tag.unwrap())
            {
                assert_eq!(logs[0]["metric_type"], exp.metric_type);
                assert_eq!(log["value"].as_f64(), Some(exp.value));
                matched = Some(id);
                break;
            }
        }
        assert!(matched.is_some());
        expected_stats.remove(matched.unwrap());
    }

    assert_eq!(expected_stats.len(), 0);
}

// LCOV_EXCL_START Don't test derives
/// ExpectedStatSnapshot helper.
/// E.g.
/// ExpectedStatSnapshot {
///     name: "test_group_bucket_counter",
///     description: "Test cumulative bucket counter with groups",
///     stat_type: BucketCounter,
///     values: vec![
///         ExpectedStatSnapshotValue {
///             group_values: vec!["one".to_string(), "two".to_string()],
///             bucket_limit: Some(BucketLimit::Num(-8)),
///             value: 0f64,
///         },
///         ExpectedStatSnapshotValue {
///             group_values: vec!["one".to_string(), "two".to_string()],
///             bucket_limit: Some(BucketLimit::Num(0)),
///             value: 0f64,
///         },
///         ExpectedStatSnapshotValue {
///             group_values: vec!["one".to_string(), "two".to_string()],
///             bucket_limit: Some(BucketLimit::Unbounded),
///             value: 3f64,
///         },
///         ExpectedStatSnapshotValue {
///             group_values: vec!["three".to_string(), "four".to_string()],
///             bucket_limit: Some(BucketLimit::Num(-8)),
///             value: 4f64,
///         },
///         ExpectedStatSnapshotValue {
///             group_values: vec!["three".to_string(), "four".to_string()],
///             bucket_limit: Some(BucketLimit::Num(0)),
///             value: 4f64,
///         },
///         ExpectedStatSnapshotValue {
///             group_values: vec!["three".to_string(), "four".to_string()],
///             bucket_limit: Some(BucketLimit::Unbounded),
///             value: 4f64,
///         },
///     ],
///     buckets: Some(Buckets::new(BucketMethod::CumulFreq, "bucket", &[-8, 0])),
/// }
#[derive(Debug)]
pub struct ExpectedStatSnapshot {
    /// Name of the stat
    pub name: &'static str,
    /// Description of the stat
    pub description: &'static str,
    /// Type of the stat
    pub stat_type: StatType,
    /// A vec of the SnapShot values expected for this snapshot.
    pub values: Vec<ExpectedStatSnapshotValue>,
    /// The buckets for the stat.
    pub buckets: Option<Buckets>,
}

/// ExpectedStatSnapshotValue helper.
#[derive(Debug)]
pub struct ExpectedStatSnapshotValue {
    /// The group values (tags) for which this value for the Snapshot os grouped by,
    pub group_values: Vec<String>,
    /// The value of this snapshot.
    pub value: f64,
    /// The upper limit of the bucket which is represented by this snapshot.
    pub bucket_limit: Option<BucketLimit>,
}
// LCOV_EXCL_STOP

/// Check that a set of stat snapshots are as expected.
pub fn check_expected_stat_snapshots(
    stats: &[StatSnapshot],
    expected_stats: &[ExpectedStatSnapshot],
) {
    for stat in expected_stats {
        let found_stat = stats.iter().find(|s| s.definition.name() == stat.name);

        assert!(found_stat.is_some(), "Failed to find stat {}", stat.name);
        let found_stat = found_stat.unwrap();

        assert_eq!(found_stat.definition.stype(), stat.stat_type);
        assert_eq!(found_stat.definition.description(), stat.description);
        assert_eq!(found_stat.definition.buckets(), stat.buckets);

        match found_stat.values {
            StatSnapshotValues::Counter(ref vals) | StatSnapshotValues::Gauge(ref vals) => {
                for value in &stat.values {
                    let found_value = vals
                        .iter()
                        .find(|val| val.group_values == value.group_values);

                    assert!(
                        found_value.is_some(),
                        "Failed to find value with groups {:?} and bucket_limit {:?} for stat {}",
                        value.group_values, // LCOV_EXCL_LINE
                        value.bucket_limit, // LCOV_EXCL_LINE
                        stat.name           // LCOV_EXCL_LINE
                    );
                    let found_value = found_value.unwrap();
                    assert_eq!(found_value.group_values, value.group_values);
                    assert_eq!(found_value.value, value.value);
                }
            }

            StatSnapshotValues::BucketCounter(ref buckets, ref vals) => {
                assert_eq!(Some(buckets), stat.buckets.as_ref());
                for value in &stat.values {
                    let found_value = vals.iter().find(|(val, bucket)| {
                        val.group_values == value.group_values
                            && Some(bucket) == value.bucket_limit.as_ref()
                    });

                    assert!(
                        found_value.is_some(),
                        "Failed to find value with groups {:?} and bucket_limit {:?} for stat {}",
                        value.group_values, // LCOV_EXCL_LINE
                        value.bucket_limit, // LCOV_EXCL_LINE
                        stat.name           // LCOV_EXCL_LINE
                    );
                    let (found_value, _) = found_value.unwrap();
                    assert_eq!(found_value.group_values, value.group_values);
                    assert_eq!(found_value.value, value.value);
                }
            }
        }
    }

    assert_eq!(stats.len(), expected_stats.len());
}

#[cfg(test)]
mod tests {
    use super::*;
    use slog::debug;
    use std::sync::mpsc;
    use std::thread;

    /// Ensure that partial writes don't cause JSON parsing errors.
    #[test]
    fn test_partial_write() {
        // Set up the logger.
        let mut data = iobuffer::IoBuffer::new();
        let logger = new_test_logger(data.clone());

        let (started_send, started_recv) = mpsc::channel();
        let (done_send, done_recv) = mpsc::channel();

        // In a separate thread, repeatedly write JSON values until
        // told to stop. There are lots of fields, and serde `write()`s each
        // value separately, so there's plenty of opportunity for reading an
        // incomplete record.
        let _ = thread::spawn(move || {
            started_send.send(()).unwrap();
            while done_recv.try_recv().is_err() {
                debug!(logger, "Some data";
                       "alfa" => "alpha",
                       "bravo" => "beta",
                       "charlie" => "gamma",
                       "delta" => "delta",
                       "echo" => "epsilon");
            }
        });

        // Wait until the thread has started.
        started_recv.recv().unwrap();

        // Now try to read some values. This should not fail with a parse
        // error.
        let _ = read_json_values(&mut data);

        // Tell the daemon thread to stop so as not to leak CPU/memory.
        done_send.send(()).unwrap();
    }
}
