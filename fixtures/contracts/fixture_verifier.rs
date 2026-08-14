use std::{fs, path::Path};

use agentmage_kernel_contracts::{
    Action, ApprovalRequest, BoundaryFailure, CancellationSignal, CapabilityGrant, ContractError,
    EvidenceReference, Plan, Prompt, Receipt, Task, ToolCall, ToolDefinition, ToolResult,
    VersionedContract, WorkPacket, from_json, to_canonical_json,
};

fn verify<T>(root: &Path, name: &str)
where
    T: VersionedContract,
{
    let path = root.join("v2/valid").join(format!("{name}.json"));
    let bytes = fs::read(path).expect("golden fixture must be readable");
    let value = from_json::<T>(&bytes).expect("golden fixture must parse");
    assert_eq!(
        to_canonical_json(&value).expect("golden fixture must serialize"),
        bytes
    );
}

#[test]
fn every_valid_and_compatibility_fixture_matches_the_public_contract() {
    let root = std::env::var_os("AGENTMAGE_CONTRACT_FIXTURE_ROOT")
        .map(std::path::PathBuf::from)
        .expect("fixture root must be explicit");
    verify::<Action>(&root, "action");
    verify::<ApprovalRequest>(&root, "approval_request");
    verify::<BoundaryFailure>(&root, "boundary_failure");
    verify::<CancellationSignal>(&root, "cancellation_signal");
    verify::<CapabilityGrant>(&root, "capability_grant");
    verify::<ContractError>(&root, "contract_error");
    verify::<EvidenceReference>(&root, "evidence_reference");
    verify::<Plan>(&root, "plan");
    verify::<Prompt>(&root, "prompt");
    verify::<Receipt>(&root, "receipt");
    verify::<Task>(&root, "task");
    verify::<ToolCall>(&root, "tool_call");
    verify::<ToolDefinition>(&root, "tool_definition");
    verify::<ToolResult>(&root, "tool_result");
    verify::<WorkPacket>(&root, "work_packet");

    for (name, expected_code) in [
        ("task.v0.unsupported.json", "contract.version.unsupported"),
        ("task.v1.unsupported.json", "contract.version.unsupported"),
        ("task.v2.missing-field.json", "contract.field.missing"),
        ("task.v2.unknown-field.json", "contract.field.unknown"),
        ("task.v2.duplicate-field.json", "contract.field.duplicate"),
        ("task.v2.malformed.json", "contract.parse.eof"),
        ("task.v2.trailing-value.json", "contract.parse.syntax"),
        ("task.v3.unsupported.json", "contract.version.unsupported"),
    ] {
        let bytes = fs::read(root.join("compatibility/v2").join(name))
            .expect("compatibility fixture must be readable");
        let error = from_json::<Task>(&bytes).expect_err("compatibility fixture must fail");
        assert_eq!(error.code, expected_code, "fixture: {name}");
    }

    let oversized = vec![b' '; agentmage_kernel_contracts::MAX_CONTRACT_JSON_BYTES + 1];
    let error = from_json::<Task>(&oversized).expect_err("oversized input must fail");
    assert_eq!(error.code, "contract.size.exceeded");
}
