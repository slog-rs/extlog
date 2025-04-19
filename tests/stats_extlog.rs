//! Extlog tests for Stats tracker.
//!
#![cfg(feature = "interval_logging")]

use slog::{info, o};
use slog_extlog::{define_stats, xlog};
use slog_extlog_derive::ExtLoggable;

use serde::Serialize;
use slog_extlog::slog_test::*;
use std::{panic, time};

const CRATE_LOG_NAME: &str = "SLOG_STATS_TRACKER_TEST";

define_stats! {
    SLOG_TEST_STATS = {
        // Some simple counters
        test_counter(Counter, "Test counter", []),
        test_second_counter(Counter, "Second test counter", []),
        test_gauge(Gauge, "Test gauge", []),
        test_second_gauge(Gauge, "Second test gauge", []),
        test_foo_count(Counter, "Counter of foos", []),
        test_grouped_counter(Counter, "Test counter grouped by name", ["name"]),
        test_double_grouped(Counter, "Test counter grouped by type and error",
                               ["name", "error"]),
        test_latest_foo_error_count(Counter, "Latest foo error byte count", []),
        test_bucket_counter_freq(BucketCounter, "Test bucket counter", [], (Freq, "bucket", [1,2,3,4])),
        test_bucket_counter_cumul_freq(BucketCounter, "Test cumulative bucket counter", [], (CumulFreq, "bucket", [1,2,3,4])),
        test_bucket_counter_grouped_freq(BucketCounter, "Test bucket counter grouped by name and error", ["name", "error"], (Freq, "bucket", [-5, 5])),
        test_bucket_counter_grouped_cumul_freq(BucketCounter, "Test cumulative bucket counter grouped by name and error", ["name", "error"], (CumulFreq, "bucket", [-5, 5]))
    }
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "1", Text = "Amount sent", Level = "Info")]
#[StatTrigger(StatName = "test_counter", Action = "Incr", Value = "1")]
#[StatTrigger(StatName = "test_second_counter", Action = "Incr", Value = "1")]
#[StatTrigger(
    StatName = "test_gauge",
    Condition = "self.bytes < 200",
    Action = "Incr",
    Value = "1"
)]
#[StatTrigger(
    StatName = "test_second_gauge",
    Condition = "self.bytes > self.unbytes as u32",
    Action = "Incr",
    ValueFrom = "self.bytes - (self.unbytes as u32)"
)]
//LCOV_EXCL_START
struct ExternalLog {
    bytes: u32,
    unbytes: i8,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "2", Text = "Some floating point number", Level = "Error")]
#[StatTrigger(
    StatName = "test_gauge",
    Condition = "self.floating > 1.0",
    Action = "Decr",
    Value = "1"
)]
struct SecondExternalLog {
    floating: f32,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "3", Text = "A string of text", Level = "Warning")]
#[StatTrigger(
    StatName = "test_foo_count",
    Condition = "self.name == \"foo\"",
    Action = "Incr",
    Value = "1"
)]
#[StatTrigger(StatName = "test_grouped_counter", Action = "Incr", Value = "1")]
struct ThirdExternalLog {
    #[StatGroup(StatName = "test_grouped_counter")]
    name: String,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "4", Text = "Some more irrelevant text", Level = "Info")]
#[StatTrigger(StatName = "test_double_grouped", Action = "Incr", Value = "1")]
#[StatTrigger(
    StatName = "test_latest_foo_error_count",
    Condition = "self.error != 0",
    Action = "SetVal",
    ValueFrom = "self.foo_count"
)]
struct FourthExternalLog {
    #[StatGroup(StatName = "test_double_grouped")]
    name: String,
    foo_count: u32,
    #[StatGroup(StatName = "test_double_grouped")]
    error: u8,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "5", Text = "Some floating point number", Level = "Error")]
#[StatTrigger(StatName = "test_bucket_counter_freq", Action = "Incr", Value = "1")]
#[StatTrigger(
    StatName = "test_bucket_counter_cumul_freq",
    Action = "Incr",
    Value = "1"
)]
struct FifthExternalLog {
    #[BucketBy(StatName = "test_bucket_counter_freq")]
    #[BucketBy(StatName = "test_bucket_counter_cumul_freq")]
    floating: f32,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(
    Id = "6",
    Text = "Some floating point number with name and error",
    Level = "Error"
)]
#[StatTrigger(
    StatName = "test_bucket_counter_grouped_freq",
    Action = "Incr",
    Value = "1"
)]
#[StatTrigger(
    StatName = "test_bucket_counter_grouped_cumul_freq",
    Action = "Incr",
    Value = "1"
)]
struct SixthExternalLog {
    #[StatGroup(StatName = "test_bucket_counter_grouped_freq")]
    #[StatGroup(StatName = "test_bucket_counter_grouped_cumul_freq")]
    name: String,
    #[StatGroup(StatName = "test_bucket_counter_grouped_freq")]
    #[StatGroup(StatName = "test_bucket_counter_grouped_cumul_freq")]
    error: u8,
    #[BucketBy(StatName = "test_bucket_counter_grouped_freq")]
    #[BucketBy(StatName = "test_bucket_counter_grouped_cumul_freq")]
    floating: f32,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "3", Text = "A string of text", Level = "Warning")]
#[StatTrigger(
    StatName = "test_double_grouped",
    Action = "Incr",
    Value = "3",
    FixedGroups = "name=foo"
)]
#[StatTrigger(
    StatName = "test_double_grouped",
    Action = "Incr",
    Value = "1",
    FixedGroups = "name=bar"
)]
struct FixedExternalLog {
    #[StatGroup(StatName = "test_double_grouped")]
    error: u8,
}

// LCOV_EXCL_STOP

// Shortcut for a standard external log of the first struct with given values.
fn log_external_stat(logger: &StatisticsLogger, bytes: u32, unbytes: i8) {
    xlog!(logger, ExternalLog { bytes, unbytes });
}

// Shortcut for a standard external log of the fourth struct with given values.
fn log_external_grouped(logger: &StatisticsLogger, name: String, error: u8) {
    xlog!(
        logger,
        FourthExternalLog {
            name,
            error,
            foo_count: 42,
        }
    );
}

// Retrieves logs for a given statistic.
fn get_stat_logs(stat_name: &str, data: &mut Buffer) -> Vec<serde_json::Value> {
    logs_in_range("STATS-1", "STATS-2", data)
        .iter()
        .filter(|l| l["name"] == stat_name)
        .cloned()
        .collect()
}

// Does the work of checking that we get the expected values in our logs.
async fn check_log_fields(stat_name: &str, data: &mut Buffer, metric_type: &str, value: f64) {
    tokio::time::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1)).await;

    let logs = get_stat_logs(stat_name, data);
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0]["metric_type"], metric_type);
    assert_eq!(logs[0]["value"].as_f64(), Some(value));
}

#[tokio::test]
async fn external_logging_works() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    log_external_stat(&logger, 234, 1);

    // Wait for the stats logs.
    check_log_fields("test_counter", &mut data, "counter", f64::from(1)).await;

    let logs = logs_in_range("STATS-1", "STATS-2", &mut data);
    assert_eq!(logs.len(), 0);
}

#[tokio::test]
async fn test_extloggable_gauges_with_equality_condition() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);

    log_external_stat(&logger, 5, 76);
    log_external_stat(&logger, 123, 3);
    xlog!(logger, SecondExternalLog { floating: 0.5 });
    xlog!(logger, SecondExternalLog { floating: 1.34 });

    // Wait for the stats logs.
    check_log_fields("test_gauge", &mut data, "gauge", f64::from(1)).await;
}

#[tokio::test]
async fn test_extloggable_field_gauges() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);

    log_external_stat(&logger, 15, 6);
    // Make some logs with garbage or missing parameters to check they are ignored.
    info!(logger, "Test log"; "log_id" => "SLOG_STATS_TRACKER_TEST-1", "bytes" => "foobar");
    info!(logger, "Test log"; "log_id" => "SLOG_STATS_TRACKER_TEST-1", "bytes" => 23.456);
    info!(logger, "Test log"; "log_id" => "SLOG_STATS_TRACKER_TEST-1", "madeup" => 10);

    // Wait for the stats logs.
    check_log_fields("test_second_gauge", &mut data, "gauge", f64::from(9)).await;

    log_external_stat(&logger, 2, 0);

    check_log_fields("test_second_gauge", &mut data, "gauge", f64::from(11)).await;
}

#[tokio::test]
async fn test_extloggable_strings() {
    // For code coverage, test that the `with_params` method works OK.
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    let logger = logger.with_params(o!("global_name" => "foobar"));

    xlog!(
        logger,
        ThirdExternalLog {
            name: "foo".to_string(),
        }
    );

    check_log_fields("test_foo_count", &mut data, "counter", f64::from(1)).await;
}

#[tokio::test]
async fn test_extloggable_set_to() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);

    xlog!(
        logger,
        FourthExternalLog {
            name: "foo".to_string(),
            error: 1,
            foo_count: 42,
        }
    );

    xlog!(
        logger,
        FourthExternalLog {
            name: "foo".to_string(),
            error: 0,
            foo_count: 12,
        }
    );

    check_log_fields(
        "test_latest_foo_error_count",
        &mut data,
        "counter",
        f64::from(42),
    )
    .await;
}

#[tokio::test]
async fn basic_extloggable_grouped_by_string() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);

    xlog!(
        logger,
        ThirdExternalLog {
            name: "bar".to_string(),
        }
    );
    xlog!(
        logger,
        ThirdExternalLog {
            name: "foo".to_string(),
        }
    );
    xlog!(
        logger,
        ThirdExternalLog {
            name: "bar".to_string(),
        }
    );
    xlog!(
        logger,
        ThirdExternalLog {
            name: "bar".to_string(),
        }
    );
    xlog!(
        logger,
        ThirdExternalLog {
            name: "bar".to_string(),
        }
    );
    xlog!(
        logger,
        ThirdExternalLog {
            name: "foo".to_string(),
        }
    );

    // Wait for the stats logs.
    tokio::time::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1)).await;
    let logs = get_stat_logs("test_grouped_counter", &mut data);
    assert_eq!(logs.len(), 2);

    check_expected_stats(
        &logs,
        vec![
            ExpectedStat {
                stat_name: "test_grouped_counter",
                tag: Some("name=bar"),
                value: 4f64,
                metric_type: "counter",
            },
            ExpectedStat {
                stat_name: "test_grouped_counter",
                tag: Some("name=foo"),
                value: 2f64,
                metric_type: "counter",
            },
        ],
    );
}

#[tokio::test]
async fn basic_extloggable_fixed_group() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);

    xlog!(logger, FixedExternalLog { error: 23 });
    xlog!(logger, FixedExternalLog { error: 23 });
    xlog!(logger, FixedExternalLog { error: 42 });

    // Wait for the stats logs.
    tokio::time::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1)).await;
    let logs = get_stat_logs("test_double_grouped", &mut data);
    assert_eq!(logs.len(), 4);

    check_expected_stats(
        &logs,
        vec![
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=foo,error=23"),
                value: 6f64,
                metric_type: "counter",
            },
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=foo,error=42"),
                value: 3f64,
                metric_type: "counter",
            },
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=bar,error=23"),
                value: 2f64,
                metric_type: "counter",
            },
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=bar,error=42"),
                value: 1f64,
                metric_type: "counter",
            },
        ],
    );
}

#[tokio::test]
async fn basic_extloggable_grouped_by_mixed() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    log_external_grouped(&logger, "bar".to_string(), 0);
    log_external_grouped(&logger, "foo".to_string(), 2);
    log_external_grouped(&logger, "bar".to_string(), 0);
    log_external_grouped(&logger, "foo".to_string(), 0);
    log_external_grouped(&logger, "bar".to_string(), 2);
    log_external_grouped(&logger, "bar".to_string(), 1);

    // Wait for the stats logs.
    tokio::time::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1)).await;
    let logs = get_stat_logs("test_double_grouped", &mut data);
    assert_eq!(logs.len(), 5);

    check_expected_stats(
        &logs,
        vec![
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=bar,error=0"),
                value: 2f64,
                metric_type: "counter",
            },
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=foo,error=0"),
                value: 1f64,
                metric_type: "counter",
            },
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=bar,error=1"),
                value: 1f64,
                metric_type: "counter",
            },
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=foo,error=2"),
                value: 1f64,
                metric_type: "counter",
            },
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=bar,error=2"),
                value: 1f64,
                metric_type: "counter",
            },
        ],
    );
}

#[tokio::test]
async fn test_extloggable_bucket_counter_freq() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    xlog!(logger, FifthExternalLog { floating: 2.5 });

    // Wait for the stats logs.
    tokio::time::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1)).await;
    let logs = get_stat_logs("test_bucket_counter_freq", &mut data);
    assert_eq!(logs.len(), 5);

    check_expected_stats(
        &logs,
        vec![
            ExpectedStat {
                stat_name: "test_bucket_counter_freq",
                tag: Some("bucket=1"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_freq",
                tag: Some("bucket=2"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_freq",
                tag: Some("bucket=3"),
                value: 1f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_freq",
                tag: Some("bucket=4"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_freq",
                tag: Some("bucket=Unbounded"),
                value: 0f64,
                metric_type: "bucket counter",
            },
        ],
    );
}

#[tokio::test]
async fn test_extloggable_bucket_counter_freq_high_value() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    xlog!(logger, FifthExternalLog { floating: 10_f32 });

    // Wait for the stats logs.
    tokio::time::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1)).await;
    let logs = get_stat_logs("test_bucket_counter_freq", &mut data);
    assert_eq!(logs.len(), 5);

    check_expected_stats(
        &logs,
        vec![
            ExpectedStat {
                stat_name: "test_bucket_counter_freq",
                tag: Some("bucket=1"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_freq",
                tag: Some("bucket=2"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_freq",
                tag: Some("bucket=3"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_freq",
                tag: Some("bucket=4"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_freq",
                tag: Some("bucket=Unbounded"),
                value: 1f64,
                metric_type: "bucket counter",
            },
        ],
    );
}

#[tokio::test]
async fn test_extloggable_bucket_counter_cumul_freq() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    xlog!(logger, FifthExternalLog { floating: 2.5 });

    // Wait for the stats logs.
    tokio::time::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1)).await;
    let logs = get_stat_logs("test_bucket_counter_cumul_freq", &mut data);
    assert_eq!(logs.len(), 5);

    check_expected_stats(
        &logs,
        vec![
            ExpectedStat {
                stat_name: "test_bucket_counter_cumul_freq",
                tag: Some("bucket=1"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_cumul_freq",
                tag: Some("bucket=2"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_cumul_freq",
                tag: Some("bucket=3"),
                value: 1f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_cumul_freq",
                tag: Some("bucket=4"),
                value: 1f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_cumul_freq",
                tag: Some("bucket=Unbounded"),
                value: 1f64,
                metric_type: "bucket counter",
            },
        ],
    );
}

#[tokio::test]
async fn test_extloggable_bucket_counter_cumul_freq_high_value() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    xlog!(logger, FifthExternalLog { floating: 8_f32 });

    // Wait for the stats logs.
    tokio::time::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1)).await;
    let logs = get_stat_logs("test_bucket_counter_cumul_freq", &mut data);
    assert_eq!(logs.len(), 5);

    check_expected_stats(
        &logs,
        vec![
            ExpectedStat {
                stat_name: "test_bucket_counter_cumul_freq",
                tag: Some("bucket=1"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_cumul_freq",
                tag: Some("bucket=2"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_cumul_freq",
                tag: Some("bucket=3"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_cumul_freq",
                tag: Some("bucket=4"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_cumul_freq",
                tag: Some("bucket=Unbounded"),
                value: 1f64,
                metric_type: "bucket counter",
            },
        ],
    );
}

#[tokio::test]
async fn test_extloggable_buckets_and_repeated_tags() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    xlog!(
        logger,
        SixthExternalLog {
            name: "name".to_string(),
            error: 3,
            floating: -1f32
        }
    );
    xlog!(
        logger,
        SixthExternalLog {
            name: "name".to_string(),
            error: 3,
            floating: 7f32
        }
    );

    // Wait for the stats logs.
    tokio::time::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1)).await;
    let logs = get_stat_logs("test_bucket_counter_grouped_freq", &mut data);
    assert_eq!(logs.len(), 3);

    check_expected_stats(
        &logs,
        vec![
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_freq",
                tag: Some("name=name,error=3,bucket=-5"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_freq",
                tag: Some("name=name,error=3,bucket=5"),
                value: 1f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_freq",
                tag: Some("name=name,error=3,bucket=Unbounded"),
                value: 1f64,
                metric_type: "bucket counter",
            },
        ],
    );
}

#[tokio::test]
async fn test_extloggable_bucket_counter_grouped_freq() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    xlog!(
        logger,
        SixthExternalLog {
            name: "first".to_string(),
            error: 1,
            floating: -7_f32
        }
    );
    xlog!(
        logger,
        SixthExternalLog {
            name: "second".to_string(),
            error: 2,
            floating: 3.7634_f32
        }
    );

    // Wait for the stats logs.
    tokio::time::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1)).await;
    let logs = get_stat_logs("test_bucket_counter_grouped_freq", &mut data);
    assert_eq!(logs.len(), 6);

    check_expected_stats(
        &logs,
        vec![
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_freq",
                tag: Some("name=first,error=1,bucket=-5"),
                value: 1f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_freq",
                tag: Some("name=first,error=1,bucket=5"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_freq",
                tag: Some("name=first,error=1,bucket=Unbounded"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_freq",
                tag: Some("name=second,error=2,bucket=-5"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_freq",
                tag: Some("name=second,error=2,bucket=5"),
                value: 1f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_freq",
                tag: Some("name=second,error=2,bucket=Unbounded"),
                value: 0f64,
                metric_type: "bucket counter",
            },
        ],
    );
}

#[tokio::test]
async fn test_extloggable_bucket_counter_grouped_cumul_freq() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    xlog!(
        logger,
        SixthExternalLog {
            name: "first".to_string(),
            error: 1,
            floating: -7_f32
        }
    );
    xlog!(
        logger,
        SixthExternalLog {
            name: "second".to_string(),
            error: 2,
            floating: 3.7634_f32
        }
    );

    // Wait for the stats logs.
    tokio::time::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1)).await;
    let logs = get_stat_logs("test_bucket_counter_grouped_cumul_freq", &mut data);
    assert_eq!(logs.len(), 6);

    check_expected_stats(
        &logs,
        vec![
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_cumul_freq",
                tag: Some("name=first,error=1,bucket=-5"),
                value: 1f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_cumul_freq",
                tag: Some("name=first,error=1,bucket=5"),
                value: 1f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_cumul_freq",
                tag: Some("name=first,error=1,bucket=Unbounded"),
                value: 1f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_cumul_freq",
                tag: Some("name=second,error=2,bucket=-5"),
                value: 0f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_cumul_freq",
                tag: Some("name=second,error=2,bucket=5"),
                value: 1f64,
                metric_type: "bucket counter",
            },
            ExpectedStat {
                stat_name: "test_bucket_counter_grouped_cumul_freq",
                tag: Some("name=second,error=2,bucket=Unbounded"),
                value: 1f64,
                metric_type: "bucket counter",
            },
        ],
    );
}

#[tokio::test]
async fn test_set_slog_logger() {
    let (mut logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);

    // check logging is working
    log_external_stat(&logger, 779, 7);
    let logs = logs_in_range(
        "SLOG_STATS_TRACKER_TEST-1",
        "SLOG_STATS_TRACKER_TEST-2",
        &mut data,
    );
    assert_eq!(logs.len(), 1);

    // modify logger to throw away all logs
    logger.set_slog_logger(slog::Logger::root(slog::Fuse::new(slog::Discard), o!()));

    // check the logger is no longer writing logs
    log_external_stat(&logger, 780, 8);
    let logs = logs_in_range(
        "SLOG_STATS_TRACKER_TEST-1",
        "SLOG_STATS_TRACKER_TEST-2",
        &mut data,
    );
    assert_eq!(logs.len(), 0);

    // check that stats were updated anyway
    check_log_fields("test_counter", &mut data, "counter", f64::from(2)).await;
}

// Verify that we can pass loggers across an unwind boundary.
#[tokio::test]
async fn unwind_safety_works() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    let res = panic::catch_unwind(|| {
        log_external_stat(&logger, 234, 1);
    });
    assert!(res.is_ok());
    // Wait for the stats logs.
    check_log_fields("test_counter", &mut data, "counter", f64::from(1)).await;

    let logs = logs_in_range("STATS-1", "STATS-2", &mut data);
    assert_eq!(logs.len(), 0);
}

#[tokio::test]
async fn multiple_stats_defns() {
    #[derive(ExtLoggable, Clone, Serialize)]
    #[LogDetails(Id = "100", Text = "Something special happened", Level = "Info")]
    #[StatTrigger(StatName = "test_special_counter", Action = "Incr", Value = "1")]
    struct SpecialLog;

    define_stats! {
        SLOG_EXTRA_STATS = {
            // Some simple counters
            test_special_counter(Counter, "An extra-special Test counter", [])
        }
    }

    let mut data = iobuffer::IoBuffer::new();
    let logger = new_test_logger(data.clone());

    let logger = StatsLoggerBuilder::default()
        .with_stats(vec![SLOG_TEST_STATS, SLOG_EXTRA_STATS])
        .fuse_with_log_interval::<DefaultStatisticsLogFormatter>(TEST_LOG_INTERVAL, logger);

    xlog!(logger, SpecialLog);
    log_external_stat(&logger, 246, 7);
    check_log_fields("test_counter", &mut data, "counter", f64::from(1)).await;
    check_log_fields("test_special_counter", &mut data, "counter", f64::from(1)).await;
}
