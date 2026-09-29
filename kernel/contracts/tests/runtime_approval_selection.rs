//! Compatibility of the Decision 0114 hunk-selection extension of approval responses.

use agentmage_kernel_contracts::{
    ApprovalId, CONTRACT_SCHEMA_VERSION, GrantId, RuntimeApprovalDisposition,
    RuntimeApprovalResponse, RuntimeHunkSelection, RuntimeRunId, from_json, to_canonical_json,
};

fn response(disposition: RuntimeApprovalDisposition) -> RuntimeApprovalResponse {
    RuntimeApprovalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: RuntimeRunId::from_raw("run-selection"),
        approval_id: ApprovalId::from_raw("approval-selection"),
        disposition,
        challenge_sha256: "c".repeat(64),
        grant_id: (disposition == RuntimeApprovalDisposition::Allow)
            .then(|| GrantId::from_raw("grant-selection")),
        selection: None,
    }
}

#[test]
fn allow_and_deny_responses_keep_their_prior_encoding() {
    // Committed literals of the encoding before the extension existed.
    let allow = format!(
        r#"{{"schema_version":{CONTRACT_SCHEMA_VERSION},"run_id":"run-selection","approval_id":"approval-selection","disposition":"allow","challenge_sha256":"{}","grant_id":"grant-selection"}}"#,
        "c".repeat(64)
    );
    let deny = format!(
        r#"{{"schema_version":{CONTRACT_SCHEMA_VERSION},"run_id":"run-selection","approval_id":"approval-selection","disposition":"deny","challenge_sha256":"{}","grant_id":null}}"#,
        "c".repeat(64)
    );
    for (literal, disposition) in [
        (allow, RuntimeApprovalDisposition::Allow),
        (deny, RuntimeApprovalDisposition::Deny),
    ] {
        let decoded: RuntimeApprovalResponse = from_json(literal.as_bytes()).unwrap();
        assert_eq!(decoded, response(disposition));
        assert_eq!(to_canonical_json(&decoded).unwrap(), literal.as_bytes());
    }
}

#[test]
fn a_narrowing_response_carries_its_exact_selection() {
    let mut narrow = response(RuntimeApprovalDisposition::Narrow);
    narrow.selection = Some(RuntimeHunkSelection {
        preimage_sha256: "a".repeat(64),
        proposal_sha256: "b".repeat(64),
        accepted_hunk_ids: vec!["d".repeat(64)],
    });
    let encoded = to_canonical_json(&narrow).unwrap();
    let text = String::from_utf8(encoded.clone()).unwrap();
    assert!(text.contains(r#""disposition":"narrow""#), "{text}");
    assert!(text.ends_with(&format!(
        r#""grant_id":null,"selection":{{"preimage_sha256":"{}","proposal_sha256":"{}","accepted_hunk_ids":["{}"]}}}}"#,
        "a".repeat(64),
        "b".repeat(64),
        "d".repeat(64)
    )));
    assert_eq!(
        from_json::<RuntimeApprovalResponse>(&encoded).unwrap(),
        narrow
    );
}

#[test]
fn unknown_selection_fields_and_dispositions_are_refused() {
    let base = format!(
        r#"{{"schema_version":{CONTRACT_SCHEMA_VERSION},"run_id":"r","approval_id":"a","disposition":"narrow","challenge_sha256":"{}","grant_id":null,"selection":{{"preimage_sha256":"{}","proposal_sha256":"{}","accepted_hunk_ids":[]"#,
        "c".repeat(64),
        "a".repeat(64),
        "b".repeat(64)
    );
    // A selection field this contract does not define is refused, as an older
    // peer refuses the whole `selection` field and the `narrow` value.
    let extra = format!(r#"{base},"all":true}}}}"#);
    let error = from_json::<RuntimeApprovalResponse>(extra.as_bytes()).unwrap_err();
    assert_eq!(error.code, "contract.field.unknown");
    let unknown = format!(r#"{base}}}}}"#).replace(r#""narrow""#, r#""narrow_all""#);
    let error = from_json::<RuntimeApprovalResponse>(unknown.as_bytes()).unwrap_err();
    assert_eq!(error.code, "contract.value.unsupported");
    let missing = format!(r#"{base}}}}}"#).replace(r#","accepted_hunk_ids":[]"#, "");
    let error = from_json::<RuntimeApprovalResponse>(missing.as_bytes()).unwrap_err();
    assert_eq!(error.code, "contract.field.missing");
}
