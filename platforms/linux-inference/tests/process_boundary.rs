#![forbid(unsafe_code)]

use std::process::{Command, Stdio};

use agentmage_platform_linux_inference::BOUNDARY_DESCRIPTION;

#[test]
fn packaged_process_self_check_is_exact_and_inactive() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmage-native-inference"))
        .arg("--self-check")
        .stdin(Stdio::null())
        .output()
        .expect("adapter executes");
    assert!(output.status.success());
    assert_eq!(output.stdout, BOUNDARY_DESCRIPTION);
    assert!(output.stderr.is_empty());
}

#[test]
fn packaged_process_refuses_inference_and_ambient_arguments() {
    for arguments in [
        vec![],
        vec!["--serve"],
        vec!["--model", "/tmp/model.gguf"],
        vec!["--self-check", "extra"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_agentmage-native-inference"))
            .args(arguments)
            .stdin(Stdio::null())
            .output()
            .expect("adapter executes");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(
            output.stderr,
            b"agentmage.native-inference.operation-unavailable-no-admitted-profile\n"
        );
    }
}
