use std::process::Command;

#[test]
fn main_processes_transactions_without_errors_works_as_expected() {
    let bin = env!("CARGO_BIN_EXE_toyments");
    let csv_path = "tests/fixtures/main_processes_transactions_without_errors_as_expected.csv";

    let output = Command::new(bin).arg(csv_path).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Status code 0
    assert!(
        output.status.success(),
        "binary failed: status={:?} stderr={stderr} stdout={stdout}",
        output.status,
    );
    // Expected report to stdout
    insta::assert_snapshot!(stdout);
    // Empty stderr
    assert!(stderr.is_empty());
}

#[test]
fn main_processes_transactions_with_errors_works_as_expected() {
    let bin = env!("CARGO_BIN_EXE_toyments");
    let csv_path = "tests/fixtures/main_processes_transactions_with_errors_as_expected.csv";

    let output = Command::new(bin).arg(csv_path).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Status code 1 due to errors
    assert_eq!(Some(1), output.status.code());
    // Expected report to stdout
    insta::assert_snapshot!(stdout);
    // Stderr populated with errors.
    // Not using snapshot because errors current representation is not yet stable enough.
    assert!(stderr.contains("failed to deserialize transaction"));
    assert!(stderr.contains("unknown variant `foo`"));
    assert!(stderr.contains("transaction already disputed"));
    assert!(stderr.contains("transaction not found"));
    assert!(stderr.contains("transaction not disputed"));
    assert!(stderr.contains("insufficient available funds"));
    assert!(stderr.contains("cannot process transaction, locked account"));
}
