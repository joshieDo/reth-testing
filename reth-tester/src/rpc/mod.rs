pub(crate) mod equality;
pub(crate) mod ext;
mod utils;

/// Equality rpc test error
enum TestError {
    Diff { local: String, remote: String },
    LocalErr(String),
    RemoteErr(String),
}

type ReportResults = Vec<(String, Vec<(MethodName, Result<(), TestError>)>)>;
type MethodName = String;