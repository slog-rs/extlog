//! External logging and statistics tracking API for [`slog`].
//!
//! This crate adds support for two features to the slog ecosystem.
//!
//! # Generating "external" logs
//! External logs are logs that form an API and so must not be modified or removed
//! without agreeing as a spec change.  New logs can always safely be added.
//!
//! The real advantage of external logs is that the log itself becomes a type, therefore
//! guaranteeing that required fields must always be provided and giving compile-time checking of
//! log generation.
//!
//! When using this crate, slog loggers can be used in two ways.
//!
//!  * External logs can be defined using this crate, and then logged using
//!    a [`StatisticsLogger`].  They can also be used as triggers for statistics generation.
//!  * For internal, low-level logging, the usual slog macros (`info!`, `debug!`, `trace!` etc)
//! can be used.
//!
//! Any object can be made into an external log by implementing [`ExtLoggable`].  In nearly all
//! cases this trait should be automatically derived using the
//! [`slog-extlog-derive`] crate.
//!
//! ## Log objects
//! To use this crate to define external logs:
//!
//!   - Add `slog-extlog` and `dlog-extlog-derive` to your `Cargo.toml`.
//!
//! To make a new external log using this crate:
//!
//!   - Define a structure with appropriate fields to act as the log.
//!   - Define a constant string named `CRATE_LOG_NAME` which uniquely identifies this crate in
//!     log identifiers.  This must uniquely identify your crate.
//!   - `use slog_extlog_derive::ExtLoggable` and derive the `ExtLoggable` trait for this object.
//!      - If your structure is overly complex or unusual, manually implement [`ExtLoggable`].
//!   - Early in your program or libary, obtain or create a [`Logger`] of the correct format and
//!     wrap it in a ['StatisticsLogger`].
//!
//! Unless you want to support statistics tracking, then the easiest way to obtain an
//! appropriate logger is to create a [`DefaultLogger`](./type.DefaultLogger.html).
//!
//! You can then call the [`slog_extlog::xlog!()`] macro, passing in the [`StatisticsLogger`] and
//! an instance of your structure, and it will be logged according to the Logger's associated Drain
//! as usual.
//! Structure parameters will be added as key-value pairs, but with the bonus that you get
//! type checking.
//!
//! You can continue to make developer logs simply using `slog` as normal:
//!
//!   - Add `slog` to your `Cargo.toml`.
//!   - Use the usual [`slog`] macros, e.g., `slog::debug!`.
//!
//! ## Parameters
//! Parameters to external logs must implement [`slog::Value`].
//!
//! For types you own, you can also derive `slog::Value` using `#[derive SlogValue]` from the
//! [`slog-extlog-derive`] crate.
//!
//! For types you do not own, you can define a wrapper type that implements `Value` using
//! [`impl_value_wrapper`](macro.impl_value_wrapper.html).
//!
//! # Statistics tracking
//!
//! [`ExtLoggable`] objects can automatically trigger changes to statistics tracked by the
//! associated [`StatsLogger`].
//!
//! To make this work, the following approach is required.
//!
//!   - Create a static set of statistic definitions using the [`define_stats`] macro.
//!   - Add `StatTrigger` attributes to each external log that explains which statistics
//!   the log should update.
//!
//! The automatic derivation code then takes care of updating the statistics as and when required.
//!
//! # Example
//! An example of a simple program that defines and produces some basic logs.
//!
//! ```
//! use serde::Serialize;
//! use slog_extlog::{DefaultLogger, stats::StatsLoggerBuilder, define_stats, xlog};
//! use slog_extlog_derive::{ExtLoggable, SlogValue};
//!
//! use slog::{Drain, debug, info, o};
//! use std::sync::Mutex;
//!
//! #[derive(Clone, Serialize, ExtLoggable)]
//! #[LogDetails(Id="101", Text="Foo Request received", Level="Info")]
//! struct FooReqRcvd;
//!
//! #[derive(Clone, Serialize, ExtLoggable)]
//! #[LogDetails(Id="103", Text="Foo response sent", Level="Info")]
//! struct FooRspSent(FooRspCode);
//!
//! #[derive(Clone, Serialize, SlogValue)]
//! enum FooRspCode {
//!     Success,
//!     InvalidUser,
//! }
//!
//! #[derive(Clone, Serialize, SlogValue)]
//! enum FooMethod {
//!     GET,
//!     PATCH,
//!     POST,
//!     PUT,
//! }
//!
//! #[derive(Clone, Serialize, SlogValue)]
//! struct FooContext {
//!     id: String,
//!     method: FooMethod,
//! }
//!
//! const CRATE_LOG_NAME: &'static str = "FOO";
//!
//! #[tokio::main]
//! async fn main() {
//!     // Use a basic logger with some context.
//!     let logger = slog::Logger::root(
//!         Mutex::new(slog_json::Json::default(std::io::stdout())).map(slog::Fuse),
//!         o!());
//!     let logger = logger.new(o!("cxt" => FooContext {
//!         id: "123456789".to_string(),
//!         method: FooMethod::POST,
//!     }));
//!     let foo_logger: DefaultLogger = StatsLoggerBuilder::default().fuse(logger);
//!
//!     // Now make some logs...
//!     xlog!(foo_logger, FooReqRcvd);
//!     debug!(foo_logger, "Some debug info");
//!     xlog!(foo_logger, FooRspSent(FooRspCode::Success));
//!     let count = 1;
//!     info!(foo_logger, "New counter value"; "count" => count);
//! }
//! ```
//!
//! [`define_stats`]: ./macro.define_stats.html
//! [`Logger`]: ../slog/struct.Logger.html
//! [`ExtLoggable`]: trait.ExtLoggable.html
//! [`slog`]:  ../slog/index.html
//! [`StatisticsLogger`]: stats/struct.StatisticsLogger.html
//! [`slog-extlog-derive`]: ../slog_extlog_derive/index.html
//! [`slog::Value`]: ../slog/trait.Value.html
//! [`xlog!()`]: macro.xlog.html

// Copyright 2017 Metaswitch Networks

// Statistics handling
pub mod stats;

// Utilities for users to call in tests.
pub mod slog_test;

/// A trait that defines requirements to be automatically derivable.
///
/// Any generic parameters in `ExtLoggable` objects must have this as a trait bound.
pub trait SlogValueDerivable: std::fmt::Debug + Clone + serde::Serialize + Send + 'static {}

impl<T> SlogValueDerivable for T where T: std::fmt::Debug + Clone + serde::Serialize + Send + 'static
{}

/// The default logger type.
pub type DefaultLogger = stats::StatisticsLogger<stats::DefaultStatisticsLogFormatter>;

/// An object that can be logged.
///
/// Usually custom-derived using the [`slog-extlog-derive`](../slog_extlog_derive/index.html)
/// crate.
pub trait ExtLoggable: slog::Value {
    /// Log this object with the provided `Logger`.
    ///
    /// Do not call directly - use [`xlog!`](macro.xlog.html) instead.
    fn ext_log<T>(&self, logger: &stats::StatisticsLogger<T>)
    where
        T: stats::StatisticsLogFormatter + Send + Sync + 'static;
}

/// Log an external log through an `slog::Logger`.
///
/// Syntactic sugar for the `ExtLoggable` trait for consistency with the standard slog macros.
#[macro_export]
macro_rules! xlog {
    ($l:expr, $($args:tt)*) => {
        $crate::ExtLoggable::ext_log(&$($args)*, &$l)
    };
}

/// Generate a [`slog::Value`](../slog/trait.Value.html) trait implementation for a type we don't
/// own but which implements `std::fmt::Display` or `std::fmt::Debug`.
///
/// This allows using types from third-party crates as values in external logs. The macro defines
/// a new wrapper type to be used in external logs, which does implement `slog::Value`.
///
/// For example, if you want to use the (fictional) `foo` crate and log errors from it, then write:
///
/// ```ignore
/// slog_extlog::impl_value_wrapper!(FooError, foo::Error);
/// ```
/// Then anywhere you want to log a `foo::Error`, you can instead use `FooError(foo::Error)` - the
/// logged value will use `foo:Error`'s impl of `Display`.
///
/// For a type which implements `Debug` but not `Display`, then you can use a `?` character:
///
/// ```ignore
/// slog_extlog::impl_value_wrapper!(FooInternal, ?foo::InternalType);
/// ```
///
/// Note: this macro can be deprecated once `default impl` is available and
/// [this issue](https://github.com/slog-rs/slog/issues/120) is fixed.
///
#[macro_export]
macro_rules! impl_value_wrapper {
    ($new:ident, ?($inner:tt)*) => {
        pub struct $new(pub $($inner)*);
        impl slog::Value for $new {
            fn serialize(&self, record: &slog::Record, key: Key, serializer: &mut slog::Serializer) -> Result {
                use slog::KV;
                slog::b!(key => ?&self.0).serialize(record, serializer)
            }
        }
    };
    ($new:ident, ($inner:tt)*) => {
        pub struct $new(pub $($inner)*);
        impl slog::Value for $new {
            fn serialize(&self, record: &slog::Record, key: Key, serializer: &mut slog::Serializer) -> Result {
                use slog::KV;
                slog::b!(key => %&self.0).serialize(record, serializer)
            }
        }
    };
}

// Re-export the derive macros to ensure that version compatibility is preserved
pub use slog_extlog_derive::{ExtLoggable, SlogValue};
