//! Tests for querying the current values of stats.

use serde::Serialize;

use slog_extlog::{define_stats, xlog};

#[cfg(feature = "derive")]
use slog_extlog::ExtLoggable;
#[cfg(not(feature = "derive"))]
use slog_extlog_derive::ExtLoggable;

use slog_extlog::slog_test::*;
use slog_extlog::stats::StatType::{BucketCounter, Counter, Gauge};
use std::str;

const CRATE_LOG_NAME: &str = "SLOG_STATS_QUERY_TEST";

define_stats! {
    ALL_STATS = {
        test_counter(Counter, "Test counter", []),
        test_gauge(Gauge, "Test gauge", []),
        test_grouped_counter(Counter, "Test grouped counter", ["counter_group_one", "counter_group_two"]),
        test_grouped_gauge(Gauge, "Test grouped gauge", ["gauge_group_one", "gauge_group_two"]),
        test_bucket_counter_freq(BucketCounter, "Test bucket counter", [], (Freq, "bucket", [1, 2, 3])),
        test_bucket_counter_cumul_freq(
            BucketCounter,
            "Test cumulative bucket counter",
            [],
            (CumulFreq, "bucket", [10, 20, 30])
        ),
        test_group_bucket_counter(
            BucketCounter,
            "Test cumulative bucket counter with groups",
            ["group1", "group2"],
            (CumulFreq, "bucket", [-8, 0])
        )
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
#[StatTrigger(
    StatName = "test_grouped_counter",
    Action = "Incr",
    ValueFrom = "self.delta"
)]
struct GroupedCounterUpdateLog {
    delta: i32,
    #[StatGroup(StatName = "test_grouped_counter")]
    counter_group_one: String,
    #[StatGroup(StatName = "test_grouped_counter")]
    counter_group_two: u32,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "4", Text = "Amount sent", Level = "Info")]
#[StatTrigger(
    StatName = "test_grouped_gauge",
    Action = "Incr",
    ValueFrom = "self.delta"
)]
struct GroupedGaugeUpdateLog {
    delta: i32,
    #[StatGroup(StatName = "test_grouped_gauge")]
    gauge_group_one: String,
    #[StatGroup(StatName = "test_grouped_gauge")]
    gauge_group_two: u32,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "5", Text = "test bucket counter stat log", Level = "Info")]
#[StatTrigger(StatName = "test_bucket_counter_freq", Action = "Incr", Value = "1")]
struct BucketCounterLog {
    #[BucketBy(StatName = "test_bucket_counter_freq")]
    bucket_value: f32,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(
    Id = "6",
    Text = "test grouped bucket counter stat log",
    Level = "Info"
)]
#[StatTrigger(
    StatName = "test_group_bucket_counter",
    Action = "Incr",
    ValueFrom = "self.delta"
)]
struct GroupBucketCounterLog {
    delta: i32,
    #[StatGroup(StatName = "test_group_bucket_counter")]
    group1: String,
    #[StatGroup(StatName = "test_group_bucket_counter")]
    group2: String,
    #[BucketBy(StatName = "test_group_bucket_counter")]
    bucket_value: f32,
}
//LCOV_EXCL_STOP

#[tokio::test]
async fn request_with_no_stats() {
    let (logger, _) = create_logger_buffer(&[]);
    let stats = logger.get_stats();

    assert_eq!(stats.len(), 0);
}

#[tokio::test]
async fn request_for_single_counter() {
    static STATS: StatDefinitions = &[&test_counter];
    let (logger, _) = create_logger_buffer(STATS);
    let stats = logger.get_stats();

    check_expected_stat_snapshots(
        &stats,
        &[ExpectedStatSnapshot {
            name: "test_counter",
            description: "Test counter",
            stat_type: Counter,
            values: vec![ExpectedStatSnapshotValue {
                group_values: vec![],
                bucket_limit: None,
                value: 0f64,
            }],
            buckets: None,
        }],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_single_gauge() {
    static STATS: StatDefinitions = &[&test_gauge];
    let (logger, _) = create_logger_buffer(STATS);
    let stats = logger.get_stats();

    check_expected_stat_snapshots(
        &stats,
        &[ExpectedStatSnapshot {
            name: "test_gauge",
            description: "Test gauge",
            stat_type: Gauge,
            values: vec![ExpectedStatSnapshotValue {
                group_values: vec![],
                bucket_limit: None,
                value: 0f64,
            }],
            buckets: None,
        }],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_multiple_metrics() {
    static STATS: StatDefinitions = &[&test_counter, &test_gauge];
    let (logger, _) = create_logger_buffer(STATS);
    let stats = logger.get_stats();

    check_expected_stat_snapshots(
        &stats,
        &[
            ExpectedStatSnapshot {
                name: "test_counter",
                description: "Test counter",
                stat_type: Counter,
                values: vec![ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: None,
                    value: 0f64,
                }],
                buckets: None,
            },
            ExpectedStatSnapshot {
                name: "test_gauge",
                description: "Test gauge",
                stat_type: Gauge,
                values: vec![ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: None,
                    value: 0f64,
                }],
                buckets: None,
            },
        ], // LCOV_EXCL_LINE Kcov bug?
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_updated_metrics() {
    static STATS: StatDefinitions = &[&test_counter, &test_gauge];
    let (logger, _) = create_logger_buffer(STATS);

    xlog!(logger, CounterUpdateLog { delta: 1 });
    xlog!(logger, GaugeUpdateLog { delta: 2 });

    let stats = logger.get_stats();

    check_expected_stat_snapshots(
        &stats,
        &[
            ExpectedStatSnapshot {
                name: "test_counter",
                description: "Test counter",
                stat_type: Counter,
                values: vec![ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: None,
                    value: 1f64,
                }],
                buckets: None,
            },
            ExpectedStatSnapshot {
                name: "test_gauge",
                description: "Test gauge",
                stat_type: Gauge,
                values: vec![ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: None,
                    value: 2f64,
                }],
                buckets: None,
            },
        ], // LCOV_EXCL_LINE Kcov bug?
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_single_counter_with_groups_but_no_values() {
    static STATS: StatDefinitions = &[&test_grouped_counter];
    let (logger, _) = create_logger_buffer(STATS);
    let stats = logger.get_stats();

    check_expected_stat_snapshots(
        &stats,
        &[ExpectedStatSnapshot {
            name: "test_grouped_counter",
            description: "Test grouped counter",
            stat_type: Counter,
            values: vec![],
            buckets: None,
        }],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_single_gauge_with_groups_but_no_values() {
    static STATS: StatDefinitions = &[&test_grouped_gauge];
    let (logger, _) = create_logger_buffer(STATS);
    let stats = logger.get_stats();

    check_expected_stat_snapshots(
        &stats,
        &[ExpectedStatSnapshot {
            name: "test_grouped_gauge",
            description: "Test grouped gauge",
            stat_type: Gauge,
            values: vec![],
            buckets: None,
        }],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_single_counter_with_groups_and_one_value() {
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
    check_expected_stat_snapshots(
        &stats,
        &[ExpectedStatSnapshot {
            name: "test_grouped_counter",
            description: "Test grouped counter",
            stat_type: Counter,
            values: vec![ExpectedStatSnapshotValue {
                group_values: vec!["value one".to_string(), "100".to_string()],
                bucket_limit: None,
                value: 1f64,
            }],
            buckets: None,
        }],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_single_gauge_with_groups_and_one_value() {
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
    check_expected_stat_snapshots(
        &stats,
        &[ExpectedStatSnapshot {
            name: "test_grouped_gauge",
            description: "Test grouped gauge",
            stat_type: Gauge,
            values: vec![ExpectedStatSnapshotValue {
                group_values: vec!["value two".to_string(), "200".to_string()],
                bucket_limit: None,
                value: 2f64,
            }],
            buckets: None,
        }],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_single_counter_with_groups_and_two_values() {
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
    check_expected_stat_snapshots(
        &stats,
        &[ExpectedStatSnapshot {
            name: "test_grouped_counter",
            description: "Test grouped counter",
            stat_type: Counter,
            values: vec![
                ExpectedStatSnapshotValue {
                    group_values: vec!["value one".to_string(), "100".to_string()],
                    bucket_limit: None,
                    value: 1f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec!["value two".to_string(), "200".to_string()],
                    bucket_limit: None,
                    value: 2f64,
                },
            ], // LCOV_EXCL_LINE Kcov bug?
            buckets: None,
        }],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_bucket_counter_freq() {
    static STATS: StatDefinitions = &[&test_bucket_counter_freq];
    let (logger, _) = create_logger_buffer(STATS);
    let stats = logger.get_stats();

    check_expected_stat_snapshots(
        &stats,
        &[ExpectedStatSnapshot {
            name: "test_bucket_counter_freq",
            description: "Test bucket counter",
            stat_type: BucketCounter,
            values: vec![
                ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: Some(BucketLimit::Num(1)),
                    value: 0f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: Some(BucketLimit::Num(2)),
                    value: 0f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: Some(BucketLimit::Num(3)),
                    value: 0f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: Some(BucketLimit::Unbounded),
                    value: 0f64,
                },
            ],
            buckets: Some(Buckets::new(BucketMethod::Freq, "bucket", &[1, 2, 3])),
        }],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_bucket_counter_freq_one_value() {
    static STATS: StatDefinitions = &[&test_bucket_counter_freq];
    let (logger, _) = create_logger_buffer(STATS);

    xlog!(logger, BucketCounterLog { bucket_value: 1.5 });

    let stats = logger.get_stats();

    check_expected_stat_snapshots(
        &stats,
        &[ExpectedStatSnapshot {
            name: "test_bucket_counter_freq",
            description: "Test bucket counter",
            stat_type: BucketCounter,
            values: vec![
                ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: Some(BucketLimit::Num(1)),
                    value: 0f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: Some(BucketLimit::Num(2)),
                    value: 1f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: Some(BucketLimit::Num(3)),
                    value: 0f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: Some(BucketLimit::Unbounded),
                    value: 0f64,
                },
            ],
            buckets: Some(Buckets::new(BucketMethod::Freq, "bucket", &[1, 2, 3])),
        }],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_bucket_counter_cumul_freq() {
    static STATS: StatDefinitions = &[&test_bucket_counter_cumul_freq];
    let (logger, _) = create_logger_buffer(STATS);
    let stats = logger.get_stats();

    check_expected_stat_snapshots(
        &stats,
        &[ExpectedStatSnapshot {
            name: "test_bucket_counter_cumul_freq",
            description: "Test cumulative bucket counter",
            stat_type: BucketCounter,
            values: vec![
                ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: Some(BucketLimit::Num(10)),
                    value: 0f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: Some(BucketLimit::Num(20)),
                    value: 0f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: Some(BucketLimit::Num(30)),
                    value: 0f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: Some(BucketLimit::Unbounded),
                    value: 0f64,
                },
            ],
            buckets: Some(Buckets::new(
                BucketMethod::CumulFreq,
                "bucket",
                &[10, 20, 30],
            )),
        }],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_bucket_counter_with_groups_and_two_values() {
    static STATS: StatDefinitions = &[&test_group_bucket_counter];
    let (logger, _) = create_logger_buffer(STATS);

    xlog!(
        logger,
        GroupBucketCounterLog {
            delta: 3,
            group1: "one".to_string(),
            group2: "two".to_string(),
            bucket_value: 7.4
        } // LCOV_EXCL_LINE Kcov bug?
    );
    xlog!(
        logger,
        GroupBucketCounterLog {
            delta: 4,
            group1: "three".to_string(),
            group2: "four".to_string(),
            bucket_value: -20f32
        } // LCOV_EXCL_LINE Kcov bug?
    );

    let stats = logger.get_stats();

    check_expected_stat_snapshots(
        &stats,
        &[ExpectedStatSnapshot {
            name: "test_group_bucket_counter",
            description: "Test cumulative bucket counter with groups",
            stat_type: BucketCounter,
            values: vec![
                ExpectedStatSnapshotValue {
                    group_values: vec!["one".to_string(), "two".to_string()],
                    bucket_limit: Some(BucketLimit::Num(-8)),
                    value: 0f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec!["one".to_string(), "two".to_string()],
                    bucket_limit: Some(BucketLimit::Num(0)),
                    value: 0f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec!["one".to_string(), "two".to_string()],
                    bucket_limit: Some(BucketLimit::Unbounded),
                    value: 3f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec!["three".to_string(), "four".to_string()],
                    bucket_limit: Some(BucketLimit::Num(-8)),
                    value: 4f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec!["three".to_string(), "four".to_string()],
                    bucket_limit: Some(BucketLimit::Num(0)),
                    value: 4f64,
                },
                ExpectedStatSnapshotValue {
                    group_values: vec!["three".to_string(), "four".to_string()],
                    bucket_limit: Some(BucketLimit::Unbounded),
                    value: 4f64,
                },
            ],
            buckets: Some(Buckets::new(BucketMethod::CumulFreq, "bucket", &[-8, 0])),
        }],
    ); // LCOV_EXCL_LINE Kcov bug?
}

#[tokio::test]
async fn request_for_many_metrics() {
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
    check_expected_stat_snapshots(
        &stats,
        &[
            ExpectedStatSnapshot {
                name: "test_counter",
                description: "Test counter",
                stat_type: Counter,
                values: vec![ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: None,
                    value: 1f64,
                }],
                buckets: None,
            },
            ExpectedStatSnapshot {
                name: "test_gauge",
                description: "Test gauge",
                stat_type: Gauge,
                values: vec![ExpectedStatSnapshotValue {
                    group_values: vec![],
                    bucket_limit: None,
                    value: 2f64,
                }],
                buckets: None,
            },
            ExpectedStatSnapshot {
                name: "test_grouped_counter",
                description: "Test grouped counter",
                stat_type: Counter,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec!["value one".to_string(), "100".to_string()],
                        bucket_limit: None,
                        value: 3f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec!["value two".to_string(), "200".to_string()],
                        bucket_limit: None,
                        value: 4f64,
                    },
                ], // LCOV_EXCL_LINE Kcov bug?
                buckets: None,
            },
            ExpectedStatSnapshot {
                name: "test_grouped_gauge",
                description: "Test grouped gauge",
                stat_type: Gauge,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec!["value three".to_string(), "300".to_string()],
                        bucket_limit: None,
                        value: 5f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec!["value four".to_string(), "400".to_string()],
                        bucket_limit: None,
                        value: 6f64,
                    },
                ], // LCOV_EXCL_LINE Kcov bug?
                buckets: None,
            },
            ExpectedStatSnapshot {
                name: "test_bucket_counter_freq",
                description: "Test bucket counter",
                stat_type: BucketCounter,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        bucket_limit: Some(BucketLimit::Num(1)),
                        value: 0f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        bucket_limit: Some(BucketLimit::Num(2)),
                        value: 0f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        bucket_limit: Some(BucketLimit::Num(3)),
                        value: 0f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        bucket_limit: Some(BucketLimit::Unbounded),
                        value: 0f64,
                    },
                ],
                buckets: Some(Buckets::new(BucketMethod::Freq, "bucket", &[1, 2, 3])),
            },
            ExpectedStatSnapshot {
                name: "test_bucket_counter_cumul_freq",
                description: "Test cumulative bucket counter",
                stat_type: BucketCounter,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        bucket_limit: Some(BucketLimit::Num(10)),
                        value: 0f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        bucket_limit: Some(BucketLimit::Num(20)),
                        value: 0f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        bucket_limit: Some(BucketLimit::Num(30)),
                        value: 0f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        bucket_limit: Some(BucketLimit::Unbounded),
                        value: 0f64,
                    },
                ],
                buckets: Some(Buckets::new(
                    BucketMethod::CumulFreq,
                    "bucket",
                    &[10, 20, 30],
                )),
            },
            ExpectedStatSnapshot {
                name: "test_bucket_counter_cumul_freq",
                description: "Test cumulative bucket counter",
                stat_type: BucketCounter,
                values: vec![
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        bucket_limit: Some(BucketLimit::Num(10)),
                        value: 0f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        bucket_limit: Some(BucketLimit::Num(20)),
                        value: 0f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        bucket_limit: Some(BucketLimit::Num(30)),
                        value: 0f64,
                    },
                    ExpectedStatSnapshotValue {
                        group_values: vec![],
                        bucket_limit: Some(BucketLimit::Unbounded),
                        value: 0f64,
                    },
                ],
                buckets: Some(Buckets::new(
                    BucketMethod::CumulFreq,
                    "bucket",
                    &[10, 20, 30],
                )),
            },
        ], // LCOV_EXCL_LINE Kcov bug?
    ); // LCOV_EXCL_LINE Kcov bug?
}
