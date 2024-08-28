use super::{ReportResults, TestError};
use console::Style;
use similar::{ChangeTag, TextDiff};

/// Prints test results to console presenting a coloured diff.
pub(crate) fn report(results_by_block: ReportResults) {
    println!("\n--- RPC Method Test Results ---");

    for (title, results) in results_by_block {
        let failures: Vec<_> = results
            .into_iter()
            .filter_map(|(name, result)| result.err().map(|err| (name, err)))
            .collect();

        if failures.is_empty() {
            println!("{title} ✅");
        } else {
            println!("\n{title} ❌");
            for (name, err) in failures {
                println!("    {name}: ❌ Failure ");
                match err {
                    TestError::Diff { local, remote } => {
                        let diff = TextDiff::from_lines(&local, &remote);
                        for op in diff.ops() {
                            for change in diff.iter_changes(op).peekable() {
                                let (sign, style) = match change.tag() {
                                    ChangeTag::Delete => ("-", Style::new().red()),
                                    ChangeTag::Insert => ("+", Style::new().green()),
                                    ChangeTag::Equal => (" ", Style::new()),
                                };
                                print!("{}{}", style.apply_to(sign).bold(), style.apply_to(change));
                            }
                        }
                    }
                    TestError::LocalErr(err) => println!("## Local node error: {err}"),
                    TestError::RemoteErr(err) => println!("## Remote node error: {err}"),
                }
            }
        }
    }

    println!("--------------------------------\n");
}
