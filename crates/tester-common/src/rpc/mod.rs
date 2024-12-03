pub mod equality;
mod macros;
mod report;

use serde_json::Value;

/// Equality rpc test error
enum TestError {
    Diff { rpc1: Value, rpc2: Value },
    Rpc1Err(String),
    Rpc2Err(String),
}

type ReportResults = Vec<(String, Vec<(MethodName, Result<(), TestError>)>)>;
type MethodName = String;
