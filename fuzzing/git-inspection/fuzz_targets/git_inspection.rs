#![no_main]

use agentmage_capability_read_only::git::{
    GitInspectionOperation, GitInspectionRequest, parse_git_inspection,
    validate_git_inspection_request,
};
use libfuzzer_sys::fuzz_target;

const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SHA_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn request(operation: GitInspectionOperation) -> GitInspectionRequest {
    GitInspectionRequest {
        schema_version: 1,
        operation,
        revision: matches!(
            operation,
            GitInspectionOperation::Show | GitInspectionOperation::Ref
        )
        .then(|| "HEAD".to_owned()),
        object_id: (operation == GitInspectionOperation::Object).then(|| "a".repeat(40)),
        pathspecs: Vec::new(),
        max_records: 1_000,
        max_output_bytes: 4 * 1024 * 1024,
    }
}

fuzz_target!(|data: &[u8]| {
    let _ = validate_git_inspection_request(data);
    for operation in GitInspectionOperation::ALL {
        let parsed = parse_git_inspection(
            &request(operation),
            SHA_A,
            SHA_B,
            SHA_C,
            data.first().is_none_or(|byte| byte & 1 == 0),
            data,
        );
        if let Ok(result) = parsed {
            assert!(result.verify());
            assert!(result.records.len() <= 1_000);
            assert!(result.observed_bytes <= 4 * 1024 * 1024);
        }
    }
});
