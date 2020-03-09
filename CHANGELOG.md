# Changelog

All notable changes to this project will be documented in this file.

This file's format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]
### Changed

## [5.2.2]
### Changed
- Add comment to statics created through `define_stats` macro to prevent downstream clippy failures. Also add a `.cargo/config` turning on some common warnings (and fix up code, mainly comments).

## [5.2.1]
### Changed
- Ensure that derive log level handling still works with `slog` 2.5.

## [5.2.0]
### Changed
- Update to 2018 edition.
- Update to latest `slog` - stop pinning to old version.

### Added

## [5.1.0]
### Changed
### Added
- Add the `FixedGroups` attribute to `StatTrigger`, allowing for a compile-time constant value on
a metric.

## [5.0.0]
### Changed
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
Initial open source release.
Versions 3.0.0 and earlier were proprietary.
