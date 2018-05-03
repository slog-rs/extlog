//! Statistics generator for [`slog`].
//!
//! This crate allows for statistics - counters and gauges - to be automatically calculated and
//! reported based on logged events.  The logged events MUST implement the [`ExtLoggable`] trait.
//!
//! To support this, the [`slog-extlog-derive'] crate can be used to link logs to a specific
//! statistic.   This generates fast, compile-time checking for statistics updates at the point
//! of logging.
//!
//! Users should use the [`define_stats`] macro to list their statistics.  They can then pass the
//! list (along with stats from any sepenedncies) to a [`StatisticsLogger`] wrapping an
//! [`slog:Logger`].  The statistics trigger function on the `ExtLoggable` objects then triggers
//! statistic updates based on logged values.
//!
//! Library users should export the result of `define_stats!`, so that binary developers can
//! track the set of stats from all dependent crates in a single tracker.
//!
//! Triggers should be added to [`ExtLoggable`] objects using the [`slog-extlog-derive`] crate.
//!
//! [`ExtLoggable`]: ../slog-extlog/trait.ExtLoggable.html
//! [`define_stats`]: ./macro.define_stats.html
//! [`Logger`]: ../slog/struct.Logger.html)
//! [`slog`]: ../slog/index.html
//! [`slog-extlog-derive`]: ../slog_extlog_derive/index.html
//! [`StatisticsLogger`]: ./struct.StatisticsLogger.html

extern crate futures;
extern crate tokio_core;
extern crate tokio_timer;

use std::panic::RefUnwindSafe;
use std::sync::Arc;
use self::tokio_core::reactor::{Core, Handle};
use self::tokio_timer::Timer;
use self::futures::stream::Stream;
use self::futures::Future;
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::RwLock;
use std::time::Duration;
use std::thread;
use std::ops::Deref;
use std::marker::PhantomData;

use super::slog;

//////////////////////////////////////////////////////
// Public types - stats definitions
//////////////////////////////////////////////////////

/// A configured statistic, defined in terms of the external logs that trigger it to change.
///
/// These definitions are provided at start of day to populate the tracker.
///
/// These should NOT be constructed directly but by using the
/// [`define_stats`](./macro.define_stats.html) macro.
///
pub trait StatDefinition: fmt::Debug {
    /// The name of this metric.  This name is reported in logs as the `metric_name` field.
    fn name(&self) -> &'static str;
    /// A human readable-description of the statistic, describing its meaning.  When logged this
    /// is the log message.
    fn description(&self) -> &'static str;
    /// The type of statistic.
    fn stype(&self) -> StatType;
    /// An optional list of field names to group the statistic by.
    fn group_by(&self) -> Vec<&'static str>;
}

/// A macro to define the statistics that can be tracked by the logger.
/// Use of this macro requires [`StatDefinition`](trait.StatDefinition.html) to be in scope.
///
/// All statistics should be defined by their library using this macro.
///
/// The syntax is as follows:
///
/// ```text
///   define_stats!{
///      STATS_LIST_NAME = {
///          StatName(Type, "Description", ["tag1, "tag2", ...]),
///          Stat Name2(...),
///          ...
///      }
///   }
/// ```
///
/// The `STATS_LIST_NAME` is then created as a vector of definitions that can be passed in as the
/// `stats` field on a `StatsConfig` object.
///
/// Each definition in the list has the format above, with the fields as follows.
///
///   - `StatName` is the externally-facing metric name.
///   - `Type` is the `StatType` of this statistic, for example `Counter`.
///    Must be a valid subtype if that enum.
///   - `Description`  is a human readable description of the statistic.  This will be logged as
///   the log message,
///   - The list of `tags` define field names to group the statistic by.
///    A non-empty list indicates that this statistic should be split into buckets,
///   counting the stat separately for each different value of these fields that is seen.
///   These might be a remote hostname, say, or a tag field.
///     - If multiple tags are provided, the stat is counted separately for all distinct
///       combinations of tag values.
///     - Use of this feature should be avoided for fields that can take very many values, such as
///   a subscriber number, or for large numbers of tags - each tag name and seen value adds a
///   performance dip and a small memory overhead that is never freed.
#[macro_export]
macro_rules! define_stats {
    // Entry point - match the full list
    ($name:ident = {$($stat:ident($stype:ident, $desc:expr, [$($tags:tt),*])),*}) => {
        /// The list of statistics.
        pub static $name: $crate::stats::StatDefinitions = &[$(&$stat),*];

        mod inner_stats {
        $(
               #[derive(Debug, Clone)]
               // Prometheus metrics are snake_case, so allow non-camel-case types here.
               #[allow(non_camel_case_types)]
               pub struct $stat;
            )*
        }
        $(define_stats!{@single $stat, $stype, $desc, $($tags),*})*
    };

      // Trait impl for StatDefinition
    (@single $stat:ident, $stype:ident, $desc:expr, $($tags:tt),*) => {

        // Suppress the warning about cases - this value is never going to be seen
        #[allow(non_upper_case_globals)]
        static $stat : inner_stats::$stat = inner_stats::$stat;

        impl $crate::stats::StatDefinition for inner_stats::$stat {
            /// The name of this statistic.
            fn name(&self) -> &'static str { stringify!($stat) }
            /// A human readable-description of the statistic, describing its meaning.
            fn description(&self) -> &'static str { $desc }
            /// The type
            fn stype(&self) -> $crate::stats::StatType { $crate::stats::StatType::$stype }
            /// An optional list of field names to group the statistic by.
            fn group_by(&self) -> Vec<&'static str> { vec![$($tags),*] }
        }
    };

    ($name:ident = {$($stat:ident($stype:ident, $id:expr, $desc:expr, [$($tags:tt),*])),*}) => {
        define_stats! { $name = {$($stat($stype, $desc, [$($tags),*])),*} }
    };
}

/// A trait indicating that this statistic can be used to trigger a statistics change.
pub trait StatTrigger {
    /// The list of stats that this trigger applies to.
    fn stat_list(&self) -> &'static [&'static (StatDefinition + Sync)];
    /// The condition that must be satisfied for this stat to change
    fn condition(&self, _stat_id: &StatDefinition) -> bool {
        false
    }
    /// Get the associated tag value for this log.
    /// The value must be convertable to a string so it can be stored internally.
    fn tag_value(&self, stat_id: &StatDefinition, _tag_name: &'static str) -> String;
    /// The details of the change to make for this stat, if `condition` returned true.
    fn change(&self, _stat_id: &StatDefinition) -> Option<ChangeType> {
        None
    }
}

/// Types of changes made to a statistic.
// LCOV_EXCL_START not interesting to track automatic derive coverage
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ChangeType {
    /// Increment by a fixed amount.
    Incr(usize),
    /// Decrement by a fixed amount.
    Decr(usize),
    /// Set to a specific value.
    SetTo(isize),
}

/// Types of statistics.  Automatically determined from the `StatDefinition`.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub enum StatType {
    /// A counter - a value that only increments.
    Counter,
    /// A gauge - a value that represents a current value and can go up or down.
    Gauge,
}
// LCOV_EXCL_STOP

impl slog::Value for StatType {
    fn serialize(
        &self,
        _record: &::slog::Record,
        key: ::slog::Key,
        serializer: &mut ::slog::Serializer,
    ) -> ::slog::Result {
        match *self {
            StatType::Counter => serializer.emit_str(key, "counter"),
            StatType::Gauge => serializer.emit_str(key, "gauge"),
        }
    } // LCOV_EXCL_LINE Kcov bug?
}

/////////////////////////////////////////////////////////////////////////////////
// The statistics tracker and related fields.
/////////////////////////////////////////////////////////////////////////////////

/// An object that tracks statistics and can be asked to log them
// LCOV_EXCL_START not interesting to track automatic derive coverage
#[derive(Debug)]
pub struct StatsTracker<T: StatisticsLogFormatter> {
    // The list of statistics, mapping from stat name to value.
    stats: HashMap<&'static str, Stat>,

    // The callback to make for logging the statistic.  This is a marker type so store it
    // as phatom.
    stat_formatter: PhantomData<T>,
}
// LCOV_EXCL_STOP

impl<T> StatsTracker<T>
where
    T: StatisticsLogFormatter,
{
    /// Create a new tracker with the given formatter.
    pub fn new() -> Self {
        StatsTracker {
            stats: HashMap::new(),
            stat_formatter: PhantomData,
        } // LCOV_EXCL_LINE Kcov bug?
    }

    /// Add a new statistic to this tracker.
    pub fn add_statistic(&mut self, defn: &'static (StatDefinition + Sync + RefUnwindSafe)) {
        let stat = Stat {
            defn,
            is_grouped: !defn.group_by().is_empty(),
            group_values: RwLock::new(HashMap::new()),
            value: StatValue::new(0, 1),
        }; // LCOV_EXCL_LINE Kcov bug?

        self.stats.insert(defn.name(), stat);
    } // LCOV_EXCL_LINE Kcov bug

    /// Update the statistics for the current log.
    ///
    /// This checks for any configured stats that are triggered by this log, and
    /// updates their value appropriately.
    fn update_stats(&self, log: &StatTrigger) {
        for defn in log.stat_list() {
            if log.condition(*defn) {
                let stat = &self.stats.get(defn.name()).expect(&format!(
                    "No statistic found with name {}, did you try writing a log through
                    a logger which wasn't initialized with your stats definitions?",
                    defn.name()
                ));
                stat.value
                    .update(&log.change(*defn).expect("Bad log definition"));
                // If this is a grouped stat, then we may also need to update the grouped
                // value.
                stat.check_update_grouped(*defn, log)
            }
        }
    }

    /// Log all statistics.
    ///
    /// This function is usually just called on a timer by the logger directly.
    pub fn log_all(&self, logger: &StatisticsLogger<T>) {
        for stat in self.stats.values() {
            if stat.is_grouped {
                // Log all the grouped values.
                let outputs = stat.get_group_name_vals();

                // The `outputs` is a vector of tuples containing the (tag value, stat value).
                for (name, val) in outputs {
                    // The tags require a vector of (tag name, tag value) types, so get these.
                    let tags = stat.get_tags(&name);
                    T::log_stat(
                        &logger,
                        &StatLogData {
                            stype: stat.defn.stype(),
                            name: stat.defn.name(),
                            description: stat.defn.description(),
                            value: val,
                            tags,
                        },
                    ); // LCOV_EXCL_LINE Kcov bug?
                }
            } else {
                // No grouping - just log the total
                T::log_stat(
                    &logger,
                    &StatLogData {
                        stype: stat.defn.stype(),
                        name: stat.defn.name(),
                        description: stat.defn.description(),
                        value: stat.value.as_float(),
                        tags: vec![],
                    },
                ); // LCOV_EXCL_LINE Kcov bug?
            }
        }
    }

    /// Retrieve the current values of all stats tracked by this logger.
    pub fn get_stats(&self) -> Vec<StatSnapshot> {
        self.stats
            .values()
            .map(|stat| stat.get_snapshot())
            .collect::<Vec<_>>()
    }
}

////////////////////////////////////
// Types to help integrate the tracker with loggers.
////////////////////////////////////

/// The default period between logging all statistics.
pub const DEFAULT_LOG_INTERVAL_SECS: u64 = 300;

/// Type alias for the return of [`define_stats`](../macro.define_stats.html).
pub type StatDefinitions = &'static [&'static (StatDefinition + Sync + RefUnwindSafe)];

/// Configuration required for tracking statistics.
///
/// This configuration should be passed to a [`StatisticsLogger`](struct.StatisticsLogger.html)
/// to allow tracking metrics from logs.
///
/// Construct either with `Default::default()` for no stats at all,
/// or else use a `StatsConfigBuilder`.
// LCOV_EXCL_START not interesting to track automatic derive coverage
#[derive(Debug)]
pub struct StatsConfig<T>
where
    T: StatisticsLogFormatter,
{
    /// The period, in seconds, to log the generated metrics into the log stream.  Defaults to
    /// 300 seconds (5 minutes).  One log will be generated for each metric value.  A value of
    /// `None` indicates stats should never be logged.
    pub interval_secs: Option<u64>,
    /// The list of statistics to track.  This MUST be created using the
    /// [`define_stats`](../macro.define_stats.html) macro.
    pub stats: Vec<StatDefinitions>,
    /// The [`tokio` reactor core](../tokio_core/reactor/struct.Core.html) to run the stats logging
    /// on, if the user is using `tokio` already.
    /// If this is `None` (the default), then a new core is created for logging stats.
    pub handle: Option<Handle>,
    /// An object that handles formatting the individual statistic values into a log.
    pub stat_formatter: PhantomData<T>,
}
// LCOV_EXCL_STOP

/// A builder to allow customization of stats config.  This gives flexibility when the other
/// methods are insufficient.
///
/// Create the builder using `new()` and chain other methods as required, ending with `fuse()` to
/// return the `StatsConfig`.
///
/// # Example
/// Creating a config with a custom stats interval and the default formatter.
///
/// ```
/// # #[macro_use]
/// # extern crate slog_extlog;
/// #
/// # use slog_extlog::stats::*;
///
/// define_stats! {
///     MY_STATS = {
///         SomeStat(Counter, "A test counter", []),
///         SomeOtherStat(Counter, "Another test counter", [])
///     }
/// }
///
/// fn main() {
///     let full_stats = vec![MY_STATS];
///     let cfg = StatsConfigBuilder::<DefaultStatisticsLogFormatter>::new()
///                  .with_stats(full_stats)
///                  .with_log_interval(30)
///                  .fuse();
/// }
/// ```
pub struct StatsConfigBuilder<T: StatisticsLogFormatter> {
    cfg: StatsConfig<T>,
}

impl<T: StatisticsLogFormatter> StatsConfigBuilder<T> {
    /// Create a new config builder, using the given formatter.
    ///
    /// The formatter must be provided here as it is intrinsic to the builder.
    pub fn new() -> Self {
        StatsConfigBuilder {
            cfg: StatsConfig {
                stats: vec![],
                stat_formatter: PhantomData,
                handle: None,
                interval_secs: None,
            },
        }
    }

    /// Set the list of statistics to track.
    pub fn with_stats(mut self, defns: Vec<StatDefinitions>) -> Self {
        self.cfg.stats = defns;
        self
    }

    /// Set the logging interval.
    pub fn with_log_interval(mut self, interval: u64) -> Self {
        self.cfg.interval_secs = Some(interval);
        self
    }

    /// Set the Tokio reactor core to use for the logging of the statistics.
    // LCOV_EXCL_START No testing for this directly - simple code and a pain to
    // create tokio setups in UT.
    pub fn with_core(mut self, handle: Handle) -> Self {
        self.cfg.handle = Some(handle);
        self
    }
    // LCOV_EXCL_STOP

    /// Return the built configuration.
    pub fn fuse(self) -> StatsConfig<T> {
        self.cfg
    }
}

// A default `StatsDefinition` with no statistics in it.
// Deprecated since 4.0 - just use an empty vector.
define_stats!{ EMPTY_STATS = {} }

impl<F> Default for StatsConfig<F>
where
    F: StatisticsLogFormatter,
{
    fn default() -> Self {
        StatsConfig {
            interval_secs: Some(DEFAULT_LOG_INTERVAL_SECS),
            stats: vec![EMPTY_STATS],
            handle: None,
            stat_formatter: PhantomData,
        }
    }
}

/// Data and callback type for actually generating the log.
///
/// This allows the user to decide what format to actually log the stats in.
// LCOV_EXCL_START not interesting to track automatic derive coverage
#[derive(Debug)]
pub struct StatLogData<'a> {
    /// The description, as provided on the definition.
    pub description: &'static str,
    /// The statistic type, automatically determined from the definition.
    pub stype: StatType,
    /// The statistic name, as provided on the definition.
    pub name: &'static str,
    /// The current value.
    pub value: f64,
    /// The groups and name
    pub tags: Vec<(&'static str, &'a str)>,
}

/// Structure to use for the default implementation of `StatisticsLogFormatter`.
#[derive(Debug, Clone)]
pub struct DefaultStatisticsLogFormatter;
// LCOV_EXCL_STOP

/// The log identifier for the default formatter.
pub static DEFAULT_LOG_ID: &str = "STATS-1";

impl StatisticsLogFormatter for DefaultStatisticsLogFormatter {
    /// The formatting callback.  This default implementation just logs each field.
    fn log_stat(logger: &StatisticsLogger<Self>, stat: &StatLogData)
    where
        Self: Sized,
    {
        // A realistic implementation would use `xlog`.  However, since the derivation of
        // `ExtLoggable` depends on this crate, we can't use it here!
        //
        // So just log the value manually using the `slog` macros.
        info!(logger, "New statistic value";
               "log_id" => DEFAULT_LOG_ID,
               "name" => stat.name,
               "metric_type" => stat.stype,
               "description" => stat.description,
               "value" => stat.value,
               "tags" => stat.tags.iter().
                   map(|x| format!("{}={}", x.0, x.1)).collect::<Vec<_>>().join(","))
    }
}

/// A trait object to allow users to customise the format of stats when logged.
pub trait StatisticsLogFormatter {
    /// The formatting callback.  This should take the statistic information and log it through the
    /// provided logger in the relevant format.
    ///
    /// The `DefaultStatisticsLogFormatter` provides a basic format, or users can override the
    /// format of the generated logs by providing an object that implements this trait in the
    /// `StatsConfig`.
    fn log_stat(logger: &StatisticsLogger<Self>, stat: &StatLogData)
    where
        Self: Sized;
}

/// A logger with statistics tracking.
///
/// This should only be created through the `new` method.
#[derive(Debug)]
pub struct StatisticsLogger<T: StatisticsLogFormatter> {
    /// The logger that receives the logs.
    logger: slog::Logger,
    /// The stats tracker.
    tracker: Arc<StatsTracker<T>>,
}

// Manually impl clone because the automatically derived type requires that `T:Clone`,
// which isn't needed.
//
// See https://github.com/rust-lang/rust/issues/26925 for details.
impl<T: StatisticsLogFormatter> Clone for StatisticsLogger<T> {
    fn clone(&self) -> Self {
        StatisticsLogger {
            logger: self.logger.clone(),
            tracker: self.tracker.clone(),
        }
    }
}

impl<T: StatisticsLogFormatter> Deref for StatisticsLogger<T> {
    type Target = slog::Logger;
    fn deref(&self) -> &Self::Target {
        &self.logger
    }
}

impl<T> StatisticsLogger<T>
where
    T: StatisticsLogFormatter + Send + Sync + 'static,
{
    /// Create a child logger with stats tracking support.
    ///
    /// The `StatsConfig` must contain the definitions necessary to generate metrics from logs.
    pub fn new(logger: slog::Logger, cfg: StatsConfig<T>) -> StatisticsLogger<T> {
        let mut tracker = StatsTracker::new();
        for set in cfg.stats {
            for s in set {
                tracker.add_statistic(*s)
            }
        }

        // Wrap the tracker in an Arc so we can pass it to the Logger and to the timer.
        let tracker = Arc::new(tracker);

        // Clone the logger and tracker for using on the timer - we may not need them,
        // but the clones are cheap.
        let timer_tracker = Arc::clone(&tracker);
        let timer_logger = logger.clone();

        let timer_full_logger = StatisticsLogger {
            logger: timer_logger.clone(),
            tracker: timer_tracker,
        };

        // Kick off a timer to repeatedly log stats, if requested.
        if let Some(interval) = cfg.interval_secs {
            let timer = Timer::default()
                .interval(Duration::from_secs(interval))
                .for_each(move |_| {
                    timer_full_logger.tracker.log_all(&timer_full_logger);
                    Ok(())
                });
            match cfg.handle {
                Some(h) => {
                    // LCOV_EXCL_START
                    // This isn't covered in tests due to the pain of generating our own cores.
                    // This code is simple enough it's unlikely to be bugged and will be well
                    // exercised by many users of the library.
                    h.spawn(timer.map_err(|_| ()));
                    // LCOV_EXCL_STOP
                }
                None => {
                    thread::spawn(|| {
                        let mut core = Core::new().expect("Failed to initialize tokio core");
                        core.run(timer).unwrap()
                    }); // LCOV_EXCL_LINE Kcov bug
                }
            }
        } // LCOV_EXCL_LINE Kcov bug
        StatisticsLogger { logger, tracker }
    }

    /// Build a child logger with new parameters.
    ///
    /// This is essentially a wrapper around `slog::Logger::new()`.
    pub fn with_params<P>(&self, params: slog::OwnedKV<P>) -> Self
    where
        P: slog::SendSyncRefUnwindSafeKV + 'static,
    {
        StatisticsLogger {
            logger: self.logger.new(params),
            tracker: self.tracker.clone(),
        } // LCOV_EXCL_LINE Kcov bug
    }

    /// Update the statistics for the current log.
    pub fn update_stats(&self, log: &StatTrigger) {
        self.tracker.update_stats(log)
    }

    /// Modify the logger field without changing the stats tracker
    pub fn set_slog_logger(&mut self, logger: slog::Logger) {
        self.logger = logger;
    }

    /// Retrieve the current values of all stats tracked by this logger.
    pub fn get_stats(&self) -> Vec<StatSnapshot> {
        self.tracker.get_stats()
    }
}

/// A snapshot of the current values for a particular stat.
// LCOV_EXCL_START not interesting to track automatic derive coverage
#[derive(Debug)]
pub struct StatSnapshot {
    pub definition: &'static StatDefinition,
    pub values: Vec<StatSnapshotValue>,
}
// LCOV_EXCL_STOP

impl StatSnapshot {
    /// Create a new snapshot of a stat
    pub fn new(definition: &'static StatDefinition, values: Vec<StatSnapshotValue>) -> Self {
        StatSnapshot { definition, values }
    }
}

/// A snapshot of a current (possibly groupedl) value for a stat.
// LCOV_EXCL_START not interesting to track automatic derive coverage
#[derive(Debug)]
pub struct StatSnapshotValue {
    pub group_values: Vec<String>,
    pub value: f64,
}
// LCOV_EXCL_STOP

impl StatSnapshotValue {
    /// Create a new snapshot value.
    pub fn new(group_values: Vec<String>, value: f64) -> Self {
        StatSnapshotValue {
            group_values,
            value,
        }
    }
}

///////////////////////////
// Private types and private methods.
///////////////////////////
// LCOV_EXCL_START not interesting to track automatic derive coverage

// The internal representation of a tracked statistic.
#[derive(Debug)]
struct Stat {
    // The definition fields, as a trait object.
    defn: &'static (StatDefinition + Sync + RefUnwindSafe),
    // The value - if grouped, this is the total value across all statistics.
    value: StatValue,
    // Does this stat use groups.  Cached here for efficiency.
    is_grouped: bool,
    // The fields the stat is grouped by.  If empty, then there is no grouping.
    group_values: RwLock<HashMap<String, StatValue>>,
}
// LCOV_EXCL_STOP

impl Stat {
    // Get all the tags for this stat as a vector of (name, value) tuples.
    fn get_tags<'a, 'b>(&'a self, name: &'b str) -> Vec<(&'static str, &'b str)> {
        self.defn
            .group_by()
            .iter()
            .cloned()
            .zip(name.split(','))
            .collect::<Vec<_>>()
    }

    // Get all the grouped value names currently tracked.
    fn get_group_name_vals(&self) -> Vec<(String, f64)> {
        // Only hold the read lock long enough to get the keys and values.
        let inner_vals = self.group_values.read().expect("Poisoned lock)");
        inner_vals
            .iter()
            .map(|(k, v)| (k.clone(), v.as_float()))
            .collect()
    }

    // Check if the stat is grouped, and if so, get and update the correct
    // grouped values.
    fn check_update_grouped(&self, defn: &StatDefinition, trigger: &StatTrigger) {
        if self.is_grouped {
            let tag_values = self.defn
                .group_by()
                .iter()
                .map(|n| trigger.tag_value(defn, n))
                .collect::<Vec<String>>()
                .join(","); // LCOV_EXCL_LINE Kcov bug?

            // Use an inner block here to ensure the read lock drops out of scope.
            {
                let inner_vals = self.group_values.read().expect("Poisoned lock");
                if let Some(val) = inner_vals.get(&tag_values) {
                    val.update(&trigger.change(defn).expect("Bad log definition"));
                    return;
                }
            }

            // We didn't find a grouped value.  Get the write lock on the map so we can add it.
            let mut inner_vals = self.group_values.write().expect("Poisoned lock");
            // It's possible that while we were waiting for the write lock another thread got
            // in and created the stat entry, so check again.
            let val = inner_vals
                .entry(tag_values)
                .or_insert_with(|| StatValue::new(0, 1));

            val.update(&trigger.change(defn).expect("Bad log definition"));
        }
    }

    /// Get the current values for this stat as a MetricFamily
    fn get_snapshot(&self) -> StatSnapshot {
        let values = if self.is_grouped {
            self.get_group_name_vals()
                .iter()
                .map(|&(ref group_value_str, value)| {
                    let group_values = group_value_str
                        .split(",")
                        .map(|group| group.to_string())
                        .collect::<Vec<_>>();

                    StatSnapshotValue::new(group_values, value)
                })
                .collect()
        } else {
            vec![StatSnapshotValue::new(vec![], self.value.as_float())]
        };

        StatSnapshot::new(self.defn, values)
    }
}

// LCOV_EXCL_START not interesting to track automatic derive coverage
/// A single statistic value.
#[derive(Debug)]
struct StatValue {
    // The tracked integer value.
    num: AtomicIsize,
    // A divisor for printing the stat value only - currently this is always 1 but is here
    // to allow in future for, say, percentages.
    divisor: u64,
}
// LCOV_EXCL_STOP

impl StatValue {
    /// Create a new value.
    fn new(num: isize, divisor: u64) -> Self {
        StatValue {
            num: AtomicIsize::new(num),
            divisor,
        }
    }

    /// Update the stat and return whether it has changed.
    fn update(&self, change: &ChangeType) -> bool {
        match *change {
            ChangeType::Incr(i) => {
                self.num.fetch_add(i as isize, Ordering::Relaxed);
                true
            }
            ChangeType::Decr(d) => {
                self.num.fetch_sub(d as isize, Ordering::Relaxed);
                true
            }

            ChangeType::SetTo(v) => self.num.swap(v, Ordering::Relaxed) != v,
        }
    }

    /// Return the statistic value as a float, for use in display.
    fn as_float(&self) -> f64 {
        (self.num.load(Ordering::Relaxed) as f64) / (self.divisor as isize as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    struct DummyNonCloneFormatter;
    impl StatisticsLogFormatter for DummyNonCloneFormatter {
        fn log_stat(_logger: &StatisticsLogger<Self>, _stat: &StatLogData)
        where
            Self: Sized,
        {
        }
    }

    #[test]
    // Check that loggers can be cloned even if the formatter can't.
    fn check_clone() {
        let logger = StatisticsLogger::new(
            slog::Logger::root(slog::Discard, o!()),
            StatsConfigBuilder::<DummyNonCloneFormatter>::new().fuse(),
        );

        let _new_logger: StatisticsLogger<DummyNonCloneFormatter> = logger.clone();
    }
}
