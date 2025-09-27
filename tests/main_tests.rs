use std::process::Command;

#[test]
fn main_processes_transactions_without_errors_as_expected() {
    let bin = env!("CARGO_BIN_EXE_toyments");
    let csv_path = "tests/fixtures/main_processes_transactions_without_errors_as_expected.csv";

    let output = Command::new(bin).arg(csv_path).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "binary failed: status={:?} stderr={stderr} stdout={stdout}",
        output.status,
    );
    insta::assert_snapshot!(stdout);
}

#[test]
fn main_processes_transactions_with_errors_as_expected() {
    let bin = env!("CARGO_BIN_EXE_toyments");
    let csv_path = "tests/fixtures/main_processes_transactions_with_errors_as_expected.csv";

    let output = Command::new(bin).arg(csv_path).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(Some(1), output.status.code());
    insta::assert_snapshot!(stdout);
    // Not using snapshotting because I consider errors current representation not stable enough.
    // Core deserialization error for invalid type
    assert!(stderr.contains("failed to deserialize transaction"));
    assert!(stderr.contains("unknown variant `foo`"));
    assert!(stderr.contains("transaction already disputed"));
    assert!(stderr.contains("transaction not found"));
    assert!(stderr.contains("transaction not disputed"));
    assert!(stderr.contains("insufficient available funds"));
    assert!(stderr.contains("cannot process transaction, locked account"));
}
