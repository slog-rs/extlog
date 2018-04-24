//! Tests for querying the current values of stats.
//!

extern crate futures;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_extlog;
#[macro_use]
extern crate slog_extlog_derive;

use slog_extlog::slog_test::*;
use slog_extlog::stats;
use slog_extlog::stats::StatType::{Counter, Gauge};
use std::str;

const CRATE_LOG_NAME: &str = "SLOG_STATS_QUERY_TEST";

define_stats! {
    ALL_STATS = {
        test_counter(Counter, "Test counter", []),
        test_gauge(Gauge, "Test gauge", []),
        test_grouped_counter(Counter, "Test grouped counter", ["counter_group_one", "counter_group_two"]),
        test_grouped_gauge(Gauge, "Test grouped gauge", ["gauge_group_one", "gauge_group_two"])
    }
}

//LCOV_EXCL_START
#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "1", Text = "Amount sent", Level = "Info")]
#[StatTrigger(StatName = "test_counter", Action = "Incr", ValueFrom = "self.delta")]
struct CounterUpdateLog {
    delta: i32,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "2", Text = "Amount sent", Level = "Info")]
#[StatTrigger(StatName = "test_gauge", Action = "Incr", ValueFrom = "self.delta")]
struct GaugeUpdateLog {
    delta: i32,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "3", Text = "Amount sent", Level = "Info")]
#[StatTrigger(StatName = "test_grouped_counter", Action = "Incr", ValueFrom = "self.delta")]
struct GroupedCounterUpdateLog {
    delta: i32,
    #[StatGroup(StatName = "test_grouped_counter")]
    counter_group_one: String,
    #[StatGroup(StatName = "test_grouped_counter")]
    counter_group_two: u32,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "3", Text = "Amount sent", Level = "Info")]
#[StatTrigger(StatName = "test_grouped_gauge", Action = "Incr", ValueFrom = "self.delta")]
struct GroupedGaugeUpdateLog {
    delta: i32,
    #[StatGroup(StatName = "test_grouped_gauge")]
    gauge_group_one: String,
    #[StatGroup(StatName = "test_grouped_gauge")]
    gauge_group_two: u32,
}
//LCOV_EXCL_STOP

#[test]
fn request_with_no_stats() {
    let (logger, _) = create_logger_buffer(stats::EMPTY_STATS);
    let stats = logger.get_stats();

    assert_eq!(stats.len(), 0);
}

#[test]
fn request_for_single_counter() {
    static STATS: StatDefinitions = &[&test_counter];
    let (logger, _) = create_logger_buffer(STATS);
    let stats = logger.get_stats();

    check_expected_stat_snaphots(
        &stats,
        &vec![
            ExpectedStatSnapshot {
                name: "test_counter",
                description: "Test counter",
                stat_type: Counter,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        value: 0f64,
                    },
                ],
            },
        ],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[test]
fn request_for_single_gauge() {
    static STATS: StatDefinitions = &[&test_gauge];
    let (logger, _) = create_logger_buffer(STATS);
    let stats = logger.get_stats();

    check_expected_stat_snaphots(
        &stats,
        &vec![
            ExpectedStatSnapshot {
                name: "test_gauge",
                description: "Test gauge",
                stat_type: Gauge,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        value: 0f64,
                    },
                ],
            },
        ],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[test]
fn request_for_multiple_metrics() {
    static STATS: StatDefinitions = &[&test_counter, &test_gauge];
    let (logger, _) = create_logger_buffer(STATS);
    let stats = logger.get_stats();

    check_expected_stat_snaphots(
        &stats,
        &vec![
            ExpectedStatSnapshot {
                name: "test_counter",
                description: "Test counter",
                stat_type: Counter,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        value: 0f64,
                    },
                ],
            },
            ExpectedStatSnapshot {
                name: "test_gauge",
                description: "Test gauge",
                stat_type: Gauge,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        value: 0f64,
                    },
                ],
            },
        ], // LCOV_EXCL_LINE Kcov bug?
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[test]
fn request_for_updated_metrics() {
    static STATS: StatDefinitions = &[&test_counter, &test_gauge];
    let (logger, _) = create_logger_buffer(STATS);

    xlog!(logger, CounterUpdateLog { delta: 1 });
    xlog!(logger, GaugeUpdateLog { delta: 2 });

    let stats = logger.get_stats();

    check_expected_stat_snaphots(
        &stats,
        &vec![
            ExpectedStatSnapshot {
                name: "test_counter",
                description: "Test counter",
                stat_type: Counter,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        value: 1f64,
                    },
                ],
            },
            ExpectedStatSnapshot {
                name: "test_gauge",
                description: "Test gauge",
                stat_type: Gauge,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        value: 2f64,
                    },
                ],
            },
        ], // LCOV_EXCL_LINE Kcov bug?
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[test]
fn request_for_single_counter_with_groups_but_no_values() {
    static STATS: StatDefinitions = &[&test_grouped_counter];
    let (logger, _) = create_logger_buffer(STATS);
    let stats = logger.get_stats();

    check_expected_stat_snaphots(
        &stats,
        &vec![
            ExpectedStatSnapshot {
                name: "test_grouped_counter",
                description: "Test grouped counter",
                stat_type: Counter,
                values: vec![],
            },
        ],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[test]
fn request_for_single_gauge_with_groups_but_no_values() {
    static STATS: StatDefinitions = &[&test_grouped_gauge];
    let (logger, _) = create_logger_buffer(STATS);
    let stats = logger.get_stats();

    check_expected_stat_snaphots(
        &stats,
        &vec![
            ExpectedStatSnapshot {
                name: "test_grouped_gauge",
                description: "Test grouped gauge",
                stat_type: Gauge,
                values: vec![],
            },
        ],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[test]
fn request_for_single_counter_with_groups_and_one_value() {
    static STATS: StatDefinitions = &[&test_grouped_counter];
    let (logger, _) = create_logger_buffer(STATS);

    xlog!(
        logger,
        GroupedCounterUpdateLog {
            delta: 1,
            counter_group_one: "value one".to_string(),
            counter_group_two: 100,
        }
    );

    let stats = logger.get_stats();
    check_expected_stat_snaphots(
        &stats,
        &vec![
            ExpectedStatSnapshot {
                name: "test_grouped_counter",
                description: "Test grouped counter",
                stat_type: Counter,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec!["value one".to_string(), "100".to_string()],
                        value: 1f64,
                    },
                ], // LCOV_EXCL_LINE Kcov bug?
            },
        ],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[test]
fn request_for_single_gauge_with_groups_and_one_value() {
    static STATS: StatDefinitions = &[&test_grouped_gauge];
    let (logger, _) = create_logger_buffer(STATS);

    xlog!(
        logger,
        GroupedGaugeUpdateLog {
            delta: 2,
            gauge_group_one: "value two".to_string(),
            gauge_group_two: 200,
        }
    );

    let stats = logger.get_stats();
    check_expected_stat_snaphots(
        &stats,
        &vec![
            ExpectedStatSnapshot {
                name: "test_grouped_gauge",
                description: "Test grouped gauge",
                stat_type: Gauge,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec!["value two".to_string(), "200".to_string()],
                        value: 2f64,
                    },
                ],
            },
        ],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[test]
fn request_for_single_counter_with_groups_and_two_values() {
    static STATS: StatDefinitions = &[&test_grouped_counter];
    let (logger, _) = create_logger_buffer(STATS);

    xlog!(
        logger,
        GroupedCounterUpdateLog {
            delta: 1,
            counter_group_one: "value one".to_string(),
            counter_group_two: 100,
        }
    );

    xlog!(
        logger,
        GroupedCounterUpdateLog {
            delta: 2,
            counter_group_one: "value two".to_string(),
            counter_group_two: 200,
        }
    );

    let stats = logger.get_stats();
    check_expected_stat_snaphots(
        &stats,
        &vec![
            ExpectedStatSnapshot {
                name: "test_grouped_counter",
                description: "Test grouped counter",
                stat_type: Counter,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec!["value one".to_string(), "100".to_string()],
                        value: 1f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec!["value two".to_string(), "200".to_string()],
                        value: 2f64,
                    },
                ], // LCOV_EXCL_LINE Kcov bug?
            },
        ],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[test]
fn request_for_many_metrics() {
    let (logger, _) = create_logger_buffer(ALL_STATS);

    xlog!(logger, CounterUpdateLog { delta: 1 });
    xlog!(logger, GaugeUpdateLog { delta: 2 });
    xlog!(
        logger,
        GroupedCounterUpdateLog {
            delta: 3,
            counter_group_one: "value one".to_string(),
            counter_group_two: 100,
        }
    );

    xlog!(
        logger,
        GroupedCounterUpdateLog {
            delta: 4,
            counter_group_one: "value two".to_string(),
            counter_group_two: 200,
        }
    );
    xlog!(
        logger,
        GroupedGaugeUpdateLog {
            delta: 5,
            gauge_group_one: "value three".to_string(),
            gauge_group_two: 300,
        }
    );

    xlog!(
        logger,
        GroupedGaugeUpdateLog {
            delta: 6,
            gauge_group_one: "value four".to_string(),
            gauge_group_two: 400,
        }
    );

    let stats = logger.get_stats();
    check_expected_stat_snaphots(
        &stats,
        &vec![
            ExpectedStatSnapshot {
                name: "test_counter",
                description: "Test counter",
                stat_type: Counter,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        value: 1f64,
                    },
                ],
            },
            ExpectedStatSnapshot {
                name: "test_gauge",
                description: "Test gauge",
                stat_type: Gauge,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        value: 2f64,
                    },
                ],
            },
            ExpectedStatSnapshot {
                name: "test_grouped_counter",
                description: "Test grouped counter",
                stat_type: Counter,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec!["value one".to_string(), "100".to_string()],
                        value: 3f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec!["value two".to_string(), "200".to_string()],
                        value: 4f64,
                    },
                ], // LCOV_EXCL_LINE Kcov bug?
            },
            ExpectedStatSnapshot {
                name: "test_grouped_gauge",
                description: "Test grouped gauge",
                stat_type: Gauge,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec!["value three".to_string(), "300".to_string()],
                        value: 5f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec!["value four".to_string(), "400".to_string()],
                        value: 6f64,
                    },
                ], // LCOV_EXCL_LINE Kcov bug?
            },
        ], // LCOV_EXCL_LINE Kcov bug?
    ); // LCOV_EXCL_LINE Kcov bug?
}
