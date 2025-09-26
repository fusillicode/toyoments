use std::process::Command;

#[test]
fn main_processes_transactions_as_expected() {
    let bin = env!("CARGO_BIN_EXE_toyments");
    let csv_path = "tests/fixtures/main_processes_transactions_as_expected.csv";

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
