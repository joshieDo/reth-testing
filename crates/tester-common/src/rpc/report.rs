use super::{ReportResults, TestError};
use assert_json_diff::assert_json_include;
use serde_json::Value;

/// Prints test results to console presenting a coloured diff.
///
/// Returns false if it has found any failure.
pub(crate) fn report(results_by_block: ReportResults) -> eyre::Result<()> {
    let mut passed = true;
    println!("\n--- RPC Method Test Results ---");

    for (title, results) in results_by_block {
        let mut passed_title = true;

        for (name, result) in results {
            match result {
                Ok(_) => continue,
                Err(TestError::Diff { rpc1, rpc2 }) => {
                    if let Some(diffs) = find_diffs(rpc1, rpc2) {
                        if passed_title {
                            passed_title = false;
                            println!("\n{title} ❌");
                        }
                        println!("    {name}: ❌ Failure ");
                        println!("{diffs}");
                    }
                }
                Err(TestError::Rpc1Err(err)) | Err(TestError::Rpc2Err(err)) => {
                    passed_title = false;
                    println!("\n{title} ❌\n{err}");
                }
            }
        }

        if passed_title {
            println!("{title} ✅");
        }
        passed &= passed_title;
    }

    println!("--------------------------------\n");
    if passed {
        Ok(())
    } else {
        Err(eyre::eyre!("Failed."))
    }
}

fn find_diffs(rpc1: Value, rpc2: Value) -> Option<String> {
    let default_panic_hook = std::panic::take_hook();

    // Suppress the panic stderr output
    std::panic::set_hook(Box::new(|_| {}));

    let diff = std::panic::catch_unwind(|| {
        assert_json_include!(actual: rpc1, expected: rpc2);
    });

    // Restore the default hook
    std::panic::set_hook(default_panic_hook);

    if let Err(err) = diff {
        let err_msg = err
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .unwrap_or_else(|| err.downcast_ref::<String>().cloned().expect("should"))
            .replace("actual", "rpc1");
        return Some(err_msg)
    }
    None
}
