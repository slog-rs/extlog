//! Main tests for `ExtLoggable` crate.
//!
//! Copyright 2017 Metaswitch Networks
//!

use serde::Serialize;
use slog::o;
use slog_extlog::xlog;
use slog_extlog_derive::{ExtLoggable, SlogValue};

use slog_extlog::slog_test;
use slog_extlog::{stats::StatsLoggerBuilder, DefaultLogger};
use std::str;

const CRATE_LOG_NAME: &str = "SLOGTST";

// Helper to create a logger and matching Ring buffer to store them.
fn create_logger(testname: &'static str) -> (DefaultLogger, iobuffer::IoBuffer) {
    let data = iobuffer::IoBuffer::new();
    let logger = slog_test::new_test_logger(data.clone()).new(o!("testname" => testname));
    let logger = StatsLoggerBuilder::default().fuse(logger);
    (logger, data)
}

#[tokio::test]
async fn test_basic_log() {
    // Create a basic log, generate it, and ensure the correct fields come through.
    //
    // The "Id" parameter becomes "log_id" (appended to the `CRATE_NAME`), the "Text" becomes
    // "msg" and the Level becomes "level".
    #[derive(Debug, Clone, Serialize, ExtLoggable)]
    #[LogDetails(Id = "123", Text = "This is a basic log", Level = "Warning")]
    struct BasicLog;

    let (logger, mut data) = create_logger("basic_log");
    xlog!(logger, BasicLog);
    let logs = slog_test::read_json_values(&mut data);
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0]["log_id"], "SLOGTST-123");
    assert_eq!(logs[0]["msg"], "This is a basic log");
    assert_eq!(logs[0]["level"], "WARN");
}

#[tokio::test]
async fn test_derived_structs() {
    // LCOV_EXCL_START not interesting to track automatic derive coverage
    #[derive(Debug, Clone, Serialize, SlogValue)]
    struct FooData {
        id: u32,
        user: String,
        count: u64,
    }

    #[derive(Debug, Clone, Serialize, SlogValue)]
    enum FooRspType {
        Ok,
        Err(u32),
    }

    #[derive(Debug, Clone, Serialize, ExtLoggable)]
    #[LogDetails(
        Id = "456",
        Text = "Received a foo response from server",
        Level = "Info"
    )]
    struct FooRspRcvd(FooRspType, &'static str);
    // LCOV_EXCL_STOP

    let (logger, mut data) = create_logger("derived_structs");
    let foo_logger = logger.new(o!("data" => FooData {
        id: 10,
        user: "Bob".to_string(),
        count: 2,
    }));
    let foo_logger: DefaultLogger = StatsLoggerBuilder::default().fuse(foo_logger);

    xlog!(foo_logger, FooRspRcvd(FooRspType::Ok, "Success"));
    let logs = slog_test::read_json_values(&mut data);
    assert_eq!(logs.len(), 1);
    let j = &logs[0];
    // From the CRATE_NAME and LogDetails::Id
    assert_eq!(j["log_id"], "SLOGTST-456");
    // From the LogDetails::Text
    assert_eq!(j["msg"], "Received a foo response from server");
    // From the LogDetails::Level
    assert_eq!(j["level"], "INFO");
    // From the logger's associated parameter
    assert_eq!(j["data"]["id"], 10);
    assert_eq!(j["data"]["user"], "Bob");

    // The inner tuple-struct fields of the ExtLoggable object.
    assert_eq!(j["details"][0], "Ok");
    assert_eq!(j["details"][1], "Success");

    xlog!(foo_logger, FooRspRcvd(FooRspType::Err(404), "Not found"));
    let logs = slog_test::read_json_values(&mut data);
    let j2 = &logs[0];
    assert_eq!(logs.len(), 1);
    assert_eq!(j2["log_id"], "SLOGTST-456");
    assert_eq!(j2["details"][0]["Err"], 404);
    assert_eq!(j2["details"][1], "Not found");
}

#[tokio::test]
async fn test_fixed_field() {
    // LCOV_EXCL_START not interesting to track automatic derive coverage
    #[derive(Debug, Clone, Serialize, ExtLoggable)]
    #[LogDetails(Id = "789", Text = "Fixed field log", Level = "Info")]
    #[FixedFields(Foo = "Bar", Answer = "42")]
    #[FixedFields(Hello = "World")]
    struct FixedFieldLog {
        field: String,
    }
    // LCOV_EXCL_STOP

    let my_log = FixedFieldLog {
        field: "This is a variable field".to_string(),
    };

    let (logger, mut data) = create_logger("fixed_field");
    xlog!(logger, my_log);

    let logs = slog_test::read_json_values(&mut data);
    assert_eq!(logs.len(), 1);
    let j = &logs[0];

    // Same as for the other tests, check that ExtLoggable has done what it's supposed to
    assert_eq!(j["log_id"], "SLOGTST-789");
    assert_eq!(j["msg"], "Fixed field log");
    assert_eq!(j["level"], "INFO");
    assert_eq!(j["details"]["field"], "This is a variable field");

    // Check that the additional FixedFields have been logged
    assert_eq!(j["Foo"], "Bar");
    assert_eq!(j["Answer"], "42");
    assert_eq!(j["Hello"], "World");
}

#[tokio::test]
async fn test_generics() {
    // Currently, fields with lifetimes cannot be used as slog::Value, because we need a way to
    // convert to a static lifetime.  Can try by making FooData have a lifetime parameter for the
    // "desc" field.

    // LCOV_EXCL_START not interesting to track automatic derive coverage
    #[derive(Debug, Clone, Serialize, SlogValue)]
    struct FooData {
        id: u32,
        desc: &'static str,
    }

    #[derive(Debug, Clone, Serialize, SlogValue)]
    struct Wrapper<V: slog_extlog::SlogValueDerivable>(V);

    #[derive(Debug, Clone, Serialize, ExtLoggable)]
    #[LogDetails(Id = "4242", Text = "A foo log", Level = "Info")]
    struct FooLog<V: slog_extlog::SlogValueDerivable> {
        data: FooData,
        inner: Wrapper<V>,
    }
    // LCOV_EXCL_STOP

    let (logger, mut data) = create_logger("derived_structs");
    let foo_data = FooData {
        id: 42,
        desc: "FoobarBaz",
    };
    let inner = Wrapper(666);
    xlog!(
        logger,
        FooLog {
            data: foo_data,
            inner,
        }
    );
    let logs = slog_test::read_json_values(&mut data);
    assert_eq!(logs.len(), 1);
    let j = &logs[0];

    assert_eq!(j["log_id"], "SLOGTST-4242");
    assert_eq!(j["msg"], "A foo log");
    assert_eq!(j["level"], "INFO");
    assert_eq!(j["details"]["data"]["id"], 42);
    assert_eq!(j["details"]["data"]["desc"], "FoobarBaz");
    assert_eq!(j["details"]["inner"], 666);
}
