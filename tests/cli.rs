use std::process::Command;

#[test]
fn runs_ok() {
    let status = Command::new(env!("CARGO_BIN_EXE_shellver"))
        .status()
        .unwrap();
    assert!(status.success());
}
