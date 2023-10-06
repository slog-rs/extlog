# Changelog

All notable changes to this project will be documented in this file.

This file's format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

### Changed

### Added

### Fixed

## [8.1.0]

### Added
- Allow providing the `Id` field of `LogDetails` as a literal integer as well as a quoted integer
- Allow providing the `Value` field of `StatTrigger` as a literal integer as well as a quoted integer
- `slog_extlog` now exports `erased-serde` so users of `slog-extlog-derive` don't need to import that crate into their namespace

### Fixed
- Migrate to `syn v2.0`

## [8.0.0]

### Changed
- Change `StatsTracker`, `StatsLoggerBuilder` and `StatisticsLogger` to not be bound by `<T>`
  - Breaking change: `StatsLoggerBuilder` loses `with_log_interval()` and gains `fuse_with_log_interval<T>`,
    where `<T>` is a `StatisticsLogFormatter`. This method is only accessible with
    the `interval_logging` feature.
  - Breaking change: `StatsTracker::log_all()` now requires a `<T: StatisticsLogFormatter>` bound
    when called.

## [7.0.0]

### Changed

- The statistics interval logging function is behind a feature flag (`interval_logging`) this allows users of the crate who are simply using the derive macros or `xlog!` to not have to pull in a whole `tokio` runtime.
- `StatisticsLogger` creation API has changed:
    - Removed `StatsConfig` and `StatsConfigBuilder`
    - Added `StatsLoggerBuilder` with `fuse(logger)` to create the `StatisticsLogger`
    - Creating a `StatisticsLogger` must be done within a `tokio v1.0` runtime if the interval logging is enabled
- `slog_extlog` gains the `derive` feature which causes it to re-export the `#[derive(ExtLoggable)]` and `#[derive(SlogValue)]` macros meaning there's no need to depend explicitly on `slog_extlog_derive` any more

## [6.0.1]

### Fixed

- Correct dependency between derive and main crate

## [6.0.0]

### Changed

- Update to 1.0 versions of `syn` and `quote`
- Update to `tokio` 0.2 and `futures` 0.3

## [5.3.0]

### Changed

- `slog_test` test methods: don't panic if log is partially written in another thread, but leave partial data for next read.
  Technically this is a breaking change if you use `slog_test`, since
  methods `new_test_logger`, `read_json_values` and `logs_in_range` now accept
  only an `IoBuffer`. However in practice callers almost certainly passed an
  `IoBuffer` already.
- (Internal) Add comment to statics created through `define_stats` macro to prevent downstream clippy failures. Also add a `.cargo/config` turning on some common warnings (and fix up code, mainly comments).

## [5.2.1]

### Fixed

- Ensure that derive log level handling still works with `slog` 2.5.

## [5.2.0]

### Changed

- Update to 2018 edition.
- Update to latest `slog` - stop pinning to old version.

### Added

## [5.1.0]

### Added

- Add the `FixedGroups` attribute to `StatTrigger`, allowing for a compile-time constant value on
a metric.

## [5.0.0]

### Added

- Add the `BucketCounter` stat type for tracking stats grouped into numerical ranges.

## [4.1.0]

### Added

- Add a `DefaultLogger` for users who don't need statistics.
- Move repository homepage.

## [4.0.0]

### Changed

 - Some small but breaking changes to stats creation API.
   - No longer require an instance of the formatter type.
   - Take a list of `StatsDefinitions` so users can track stats from multiple crates in one place.

### Fixed
- Proper `Clone` support for `StatisticsLogger`

## [3.2.0] - 2018-04-25

### Added
- Add support for retrieving the current values of all stats.

## [3.1.0] - 2018-04-20

- Initial open source release.
- Versions 3.0.0 and earlier were proprietary.
