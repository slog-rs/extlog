//! Extlog tests for Stats tracker.
//!

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_extlog;
#[macro_use]
extern crate slog_extlog_derive;

use slog_extlog::stats::*;
use slog_extlog::slog_test::*;
use std::{panic, thread, time};

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
        test_latest_foo_error_count(Counter, "Latest foo error byte count", [])
    }
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "1", Text = "Amount sent", Level = "Info")]
#[StatTrigger(StatName = "test_counter", Action = "Incr", Value = "1")]
#[StatTrigger(StatName = "test_second_counter", Action = "Incr", Value = "1")]
#[StatTrigger(StatName = "test_gauge", Condition = "self.bytes < 200", Action = "Incr",
              Value = "1")]
#[StatTrigger(StatName = "test_second_gauge", Condition = "self.bytes > self.unbytes as u32",
              Action = "Incr", ValueFrom = "self.bytes - (self.unbytes as u32)")]
//LCOV_EXCL_START
struct ExternalLog {
    bytes: u32,
    unbytes: i8,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "2", Text = "Some floating point number", Level = "Error")]
#[StatTrigger(StatName = "test_gauge", Condition = "self.floating > 1.0", Action = "Decr",
              Value = "1")]
struct SecondExternalLog {
    floating: f32,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "3", Text = "A string of text", Level = "Warning")]
#[StatTrigger(StatName = "test_foo_count", Condition = "self.name == \"foo\"", Action = "Incr",
              Value = "1")]
#[StatTrigger(StatName = "test_grouped_counter", Action = "Incr", Value = "1")]
struct ThirdExternalLog {
    #[StatGroup(StatName = "test_grouped_counter")]
    name: String,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "4", Text = "Some more irrelevant text", Level = "Info")]
#[StatTrigger(StatName = "test_double_grouped", Action = "Incr", Value = "1")]
#[StatTrigger(StatName = "test_latest_foo_error_count", Condition = "self.error != 0",
              Action = "SetVal", ValueFrom = "self.foo_count")]
struct FourthExternalLog {
    #[StatGroup(StatName = "test_double_grouped")]
    name: String,
    foo_count: u32,
    #[StatGroup(StatName = "test_double_grouped")]
    error: u8,
}
//LCOV_EXCL_STOP

// Shortcut for a standard external log of the first struct with given values.
fn log_external_stat(
    logger: &StatisticsLogger<DefaultStatisticsLogFormatter>,
    bytes: u32,
    unbytes: i8,
) {
    xlog!(logger, ExternalLog { bytes, unbytes });
}

// Shortcut for a standard external log of the fifourthrst struct with given values.
fn log_external_grouped(
    logger: &StatisticsLogger<DefaultStatisticsLogFormatter>,
    name: String,
    error: u8,
) {
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
fn get_stat_logs(
    stat_name: &'static str,
    mut data: &mut iobuffer::IoBuffer,
) -> Vec<serde_json::Value> {
    logs_in_range("STATS-1", "STATS-2", &mut data)
        .iter()
        .cloned()
        .filter(|l| l["name"] == stat_name)
        .collect()
}

// Does the work of checking that we get the expected values in our logs.
fn check_log_fields(
    stat_name: &'static str,
    data: &mut iobuffer::IoBuffer,
    metric_type: &str,
    value: f64,
) {
    thread::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1));

    let logs = get_stat_logs(stat_name, data);
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0]["metric_type"], metric_type);
    assert_eq!(logs[0]["value"].as_f64(), Some(value));
}

#[test]
fn external_logging_works() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    log_external_stat(&logger, 234, 1);

    // Wait for the stats logs.
    check_log_fields("test_counter", &mut data, "counter", f64::from(1));

    let logs = logs_in_range("STATS-1", "STATS-2", &mut data);
    assert_eq!(logs.len(), 0);
}

#[test]
fn test_extloggable_gauges_with_equality_condition() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);

    log_external_stat(&logger, 5, 76);
    log_external_stat(&logger, 123, 3);
    xlog!(logger, SecondExternalLog { floating: 0.5 });
    xlog!(logger, SecondExternalLog { floating: 1.34 });

    // Wait for the stats logs.
    check_log_fields("test_gauge", &mut data, "gauge", f64::from(1));
}

#[test]
fn test_extloggable_field_gauges() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);

    log_external_stat(&logger, 15, 6);
    // Make some logs with garbage or missing parameters to check they are ignored.
    info!(logger, "Test log"; "log_id" => "SLOG_STATS_TRACKER_TEST-1", "bytes" => "foobar");
    info!(logger, "Test log"; "log_id" => "SLOG_STATS_TRACKER_TEST-1", "bytes" => 23.456);
    info!(logger, "Test log"; "log_id" => "SLOG_STATS_TRACKER_TEST-1", "madeup" => 10);

    // Wait for the stats logs.
    check_log_fields("test_second_gauge", &mut data, "gauge", f64::from(9));

    log_external_stat(&logger, 2, 0);

    check_log_fields("test_second_gauge", &mut data, "gauge", f64::from(11));
}

#[test]
fn test_extloggable_strings() {
    // For code coverage, test that the `with_params` method works OK.
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    let logger = logger.with_params(o!("global_name" => "foobar"));

    xlog!(
        logger,
        ThirdExternalLog {
            name: "foo".to_string(),
        }
    );

    check_log_fields("test_foo_count", &mut data, "counter", f64::from(1));
}

#[test]
fn test_extloggable_setto() {
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
    );
}

#[test]
fn basic_extloggable_grouped_by_string() {
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
    thread::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1));
    let logs = get_stat_logs("test_grouped_counter", &mut data);
    assert_eq!(logs.len(), 2);

    check_expected_stats(
        &logs,
        vec![
            ExpectedStat {
                stat_name: "test_grouped_counter",
                tag: Some("name=bar"),
                value: 4f64,
            },
            ExpectedStat {
                stat_name: "test_grouped_counter",
                tag: Some("name=foo"),
                value: 2f64,
            },
        ],
    );
}

#[test]
fn basic_extloggable_grouped_by_mixed() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    log_external_grouped(&logger, "bar".to_string(), 0);
    log_external_grouped(&logger, "foo".to_string(), 2);
    log_external_grouped(&logger, "bar".to_string(), 0);
    log_external_grouped(&logger, "foo".to_string(), 0);
    log_external_grouped(&logger, "bar".to_string(), 2);
    log_external_grouped(&logger, "bar".to_string(), 1);

    // Wait for the stats logs.
    thread::sleep(time::Duration::from_secs(TEST_LOG_INTERVAL + 1));
    let logs = get_stat_logs("test_double_grouped", &mut data);
    assert_eq!(logs.len(), 5);

    check_expected_stats(
        &logs,
        vec![
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=bar,error=0"),
                value: 2f64,
            },
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=foo,error=0"),
                value: 1f64,
            },
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=bar,error=1"),
                value: 1f64,
            },
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=foo,error=2"),
                value: 1f64,
            },
            ExpectedStat {
                stat_name: "test_double_grouped",
                tag: Some("name=bar,error=2"),
                value: 1f64,
            },
        ],
    );
}

#[test]
fn test_set_slog_logger() {
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
    check_log_fields("test_counter", &mut data, "counter", f64::from(2));
}

// Verify that we can pass loggers across an unwind boundary.
#[test]
fn unwind_safety_works() {
    let (logger, mut data) = create_logger_buffer(SLOG_TEST_STATS);
    let res = panic::catch_unwind(|| {
        log_external_stat(&logger, 234, 1);
    });
    assert!(res.is_ok());
    // Wait for the stats logs.
    check_log_fields("test_counter", &mut data, "counter", f64::from(1));

    let logs = logs_in_range("STATS-1", "STATS-2", &mut data);
    assert_eq!(logs.len(), 0);
}

#[test]
fn multiple_stats_defns() {
    #[derive(ExtLoggable, Clone, Serialize)]
    #[LogDetails(Id = "100", Text = "Soemthing special happened", Level = "Info")]
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

    let logger = StatisticsLogger::new(
        logger,
        StatsConfigBuilder::<DefaultStatisticsLogFormatter>::new()
            .with_log_interval(TEST_LOG_INTERVAL)
            .with_stats(vec![SLOG_TEST_STATS, SLOG_EXTRA_STATS])
            .fuse(), // LCOV_EXCL_LINE Kcov bug?
    ); // LCOV_EXCL_LINE Kcov bug?

    xlog!(logger, SpecialLog);
    log_external_stat(&logger, 246, 7);
    check_log_fields("test_counter", &mut data, "counter", f64::from(1));
    check_log_fields("test_special_counter", &mut data, "counter", f64::from(1));
}
