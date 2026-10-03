// The route grant owner over a real encrypted operational store in a private
// temporary directory; no process, transport, network or model.
use super::*;
use crate::runtime_start_tests::JobLedgerStore;

/// 2026-01-02 at midnight.
const NOW: u64 = 1_767_312_000_000;
const WORKSPACE: &str = "coding-development-0123456789abcdef01234567";

fn file(grant_id: &str, route_id: &str) -> RouteGrantFile {
    RouteGrantFile {
        schema_version: 1,
        grant_id: grant_id.to_owned(),
        route_id: route_id.to_owned(),
        candidate_sha256: "d".repeat(64),
        provider_id: "provider-remote".to_owned(),
        data_classes: vec![
            RunRouteDataClass::ToolOutputs,
            RunRouteDataClass::Conversation,
            RunRouteDataClass::WorkspaceExcerpts,
        ],
        max_requests: 3,
        max_input_tokens: 100_000,
        fallback_allowed: false,
        valid_for_hours: 24,
    }
}

fn grant(grant_id: &str, route_id: &str) -> RouteGrant {
    RouteGrant::from_file(&file(grant_id, route_id), NOW).unwrap()
}

fn grant_request(grant_id: &str, route_id: &str) -> RouteGrantRequest {
    RouteGrantRequest::Grant {
        workspace_id: WORKSPACE.to_owned(),
        grant: grant(grant_id, route_id),
    }
}

/// Answers one request over a freshly opened store.
fn answer(store: &JobLedgerStore, request: &RouteGrantRequest, now: u64) -> RouteGrantAnswer {
    let mut held = None;
    let answer = answer_route_grant(
        request,
        &mut || {
            let runtime = store
                .try_runtime()
                .ok_or(RouteGrantRefusal::StoreUnavailable)?;
            let states = runtime.owner_states();
            held = Some(runtime);
            Ok(states)
        },
        Some(now),
    );
    drop(held);
    answer
}

fn refused(answer: RouteGrantAnswer) -> Option<RouteGrantRefusal> {
    match answer {
        RouteGrantAnswer::Refused { refusal } => Some(refusal),
        _ => None,
    }
}

fn listed(store: &JobLedgerStore, workspace_id: Option<&str>, now: u64) -> Vec<RouteGrantView> {
    match answer(
        store,
        &RouteGrantRequest::List {
            workspace_id: workspace_id.map(str::to_owned),
        },
        now,
    ) {
        RouteGrantAnswer::Listed { grants, .. } => grants,
        other => panic!("listed: {other:?}"),
    }
}

#[test]
fn a_request_file_becomes_a_sealed_grant_only_within_its_rules() {
    let sealed = grant("grant-remote", "remote-route");
    // The data classes are kept once each and in order; the digest is the
    // kernel's, and the expiry is dated from the client's clock.
    assert_eq!(
        sealed.data_classes,
        [
            RunRouteDataClass::Conversation,
            RunRouteDataClass::WorkspaceExcerpts,
            RunRouteDataClass::ToolOutputs
        ]
    );
    assert_eq!(sealed.expires_at_epoch_ms, NOW + 24 * HOUR_MS);
    assert_eq!(
        sealed.computed_sha256().as_deref(),
        Some(sealed.grant_sha256.as_str())
    );
    assert_eq!(
        hybrid_route_grant_digest(&sealed.kernel()).unwrap(),
        sealed.grant_sha256
    );
    let changed: [fn(&mut RouteGrantFile); 12] = [
        |file| file.schema_version = 2,
        |file| file.valid_for_hours = 0,
        |file| file.valid_for_hours = MAX_GRANT_VALIDITY_HOURS + 1,
        |file| file.grant_id = "grant remote".to_owned(),
        |file| file.route_id = "-route".to_owned(),
        |file| file.provider_id = "p".repeat(129),
        |file| file.candidate_sha256 = "D".repeat(64),
        |file| file.data_classes.clear(),
        |file| file.data_classes.push(RunRouteDataClass::Conversation),
        |file| file.max_requests = 0,
        |file| file.max_requests = MAX_GRANT_REQUESTS + 1,
        |file| file.max_input_tokens = MAX_GRANT_INPUT_TOKENS + 1,
    ];
    for change in changed {
        let mut request = file("grant-remote", "remote-route");
        change(&mut request);
        assert_eq!(RouteGrant::from_file(&request, NOW), None, "{request:?}");
    }
    // A request file is read exactly: an unknown member or another shape, or
    // more than its bound, refuses before any host is launched.
    let bytes = serde_json::to_vec(&file("grant-remote", "remote-route")).unwrap();
    assert_eq!(
        grant_request_of(WORKSPACE, &bytes, NOW),
        Ok(grant_request("grant-remote", "remote-route"))
    );
    let mut extra = serde_json::to_value(file("grant-remote", "remote-route")).unwrap();
    extra["endpoint"] = serde_json::json!("provider-endpoint");
    let mut missing = serde_json::to_value(file("grant-remote", "remote-route")).unwrap();
    missing.as_object_mut().unwrap().remove("valid_for_hours");
    for refused in [
        serde_json::to_vec(&extra).unwrap(),
        serde_json::to_vec(&missing).unwrap(),
        b"[]".to_vec(),
        [
            bytes.clone(),
            vec![b' '; MAX_ROUTE_GRANT_FILE_BYTES as usize],
        ]
        .concat(),
    ] {
        assert_eq!(
            grant_request_of(WORKSPACE, &refused, NOW),
            Err(RouteGrantRefusal::InvalidInput)
        );
    }
}

#[test]
fn a_grant_is_kept_listed_revoked_and_survives_reopening() {
    let store = JobLedgerStore::new("route-grant-lifecycle");
    assert!(listed(&store, None, NOW).is_empty());
    let request = grant_request("grant-remote", "remote-route");
    let granted = answer(&store, &request, NOW);
    let RouteGrantAnswer::Granted {
        grant: view,
        receipt,
    } = &granted
    else {
        panic!("granted: {granted:?}");
    };
    assert!(route_grant_answer_acknowledges(&request, &granted));
    assert_eq!(receipt.catalog_revision, 1);
    assert_eq!(
        Some(receipt.decision_sha256.clone()),
        route_grant_decision_sha256(&request)
    );
    assert_eq!(view.state, RouteGrantState::Live);
    // Another workspace sees nothing of it; a reopened store keeps it.
    assert!(listed(&store, Some("workspace-other"), NOW).is_empty());
    assert_eq!(
        listed(&store, Some(WORKSPACE), NOW),
        std::slice::from_ref(view)
    );
    let revoke = RouteGrantRequest::Revoke {
        workspace_id: WORKSPACE.to_owned(),
        grant_id: "grant-remote".to_owned(),
    };
    let revoked = answer(&store, &revoke, NOW + 1);
    assert!(route_grant_answer_acknowledges(&revoke, &revoked));
    let RouteGrantAnswer::Revoked {
        grant: view,
        receipt,
    } = &revoked
    else {
        panic!("revoked: {revoked:?}");
    };
    assert_eq!(receipt.catalog_revision, 2);
    assert_eq!(view.state, RouteGrantState::Revoked);
    assert_eq!(view.revoked_at_epoch_ms, Some(NOW + 1));
    // Revoking is permanent, and the revoked grant stays listed.
    assert_eq!(
        refused(answer(&store, &revoke, NOW + 2)),
        Some(RouteGrantRefusal::AlreadyRevoked)
    );
    assert_eq!(listed(&store, None, NOW + 2), std::slice::from_ref(view));
    // The same route may be granted again once no live grant names it.
    assert!(matches!(
        answer(
            &store,
            &grant_request("grant-again", "remote-route"),
            NOW + 3
        ),
        RouteGrantAnswer::Granted { .. }
    ));
    let runtime = store.try_runtime().unwrap();
    assert_eq!(
        live_route_grants(&runtime.owner_states(), WORKSPACE, NOW + 4)
            .unwrap()
            .iter()
            .map(|granted| granted.grant.grant_id.as_str())
            .collect::<Vec<_>>(),
        ["grant-again"]
    );
}

#[test]
fn a_grant_outside_its_rules_is_refused_and_changes_nothing() {
    let store = JobLedgerStore::new("route-grant-refusals");
    assert!(matches!(
        answer(&store, &grant_request("grant-remote", "remote-route"), NOW),
        RouteGrantAnswer::Granted { .. }
    ));
    let mut tampered = grant("grant-tampered", "route-tampered");
    tampered.max_requests += 1;
    let mut too_late = grant("grant-late", "route-late");
    too_late.expires_at_epoch_ms = NOW + MAX_EXPIRY_AFTER_MS + 1;
    too_late.grant_sha256 = too_late.computed_sha256().unwrap();
    let cases = [
        (
            RouteGrantRequest::Grant {
                workspace_id: WORKSPACE.to_owned(),
                grant: tampered,
            },
            RouteGrantRefusal::InvalidInput,
        ),
        (
            RouteGrantRequest::Grant {
                workspace_id: "workspace with spaces".to_owned(),
                grant: grant("grant-other", "route-other"),
            },
            RouteGrantRefusal::InvalidInput,
        ),
        (
            RouteGrantRequest::Grant {
                workspace_id: WORKSPACE.to_owned(),
                grant: too_late,
            },
            RouteGrantRefusal::ExpiryOutOfRange,
        ),
        (
            grant_request("grant-remote", "route-other"),
            RouteGrantRefusal::Duplicate,
        ),
        (
            grant_request("grant-second", "remote-route"),
            RouteGrantRefusal::RouteAlreadyGranted,
        ),
        (
            RouteGrantRequest::Revoke {
                workspace_id: WORKSPACE.to_owned(),
                grant_id: "grant-unknown".to_owned(),
            },
            RouteGrantRefusal::NotFound,
        ),
        (
            RouteGrantRequest::Revoke {
                workspace_id: "workspace-other".to_owned(),
                grant_id: "grant-remote".to_owned(),
            },
            RouteGrantRefusal::NotFound,
        ),
    ];
    for (request, expected) in cases {
        assert_eq!(
            refused(answer(&store, &request, NOW)),
            Some(expected),
            "{request:?}"
        );
    }
    // A grant dated by a client whose clock is a day behind has already expired.
    assert_eq!(
        refused(answer(
            &store,
            &grant_request("grant-stale", "route-stale"),
            NOW + 24 * HOUR_MS
        )),
        Some(RouteGrantRefusal::ExpiryOutOfRange)
    );
    // A clock earlier than the catalog's last grant is refused, so an expired
    // grant cannot appear live again.
    assert_eq!(
        refused(answer(
            &store,
            &RouteGrantRequest::List { workspace_id: None },
            NOW - 1
        )),
        Some(RouteGrantRefusal::ClockUnavailable)
    );
    assert_eq!(
        answer_route_grant(
            &RouteGrantRequest::List { workspace_id: None },
            &mut || Err(RouteGrantRefusal::StoreUnavailable),
            None
        ),
        RouteGrantAnswer::Refused {
            refusal: RouteGrantRefusal::ClockUnavailable
        }
    );
    let views = listed(&store, None, NOW);
    assert_eq!(views.len(), 1);
    let runtime = store.try_runtime().unwrap();
    assert_eq!(
        runtime
            .owner_states()
            .load(OwnerStateName::RouteGrantCatalog)
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn a_workspace_holds_at_most_its_live_grants_and_the_catalog_its_workspaces() {
    let store = JobLedgerStore::new("route-grant-bounds");
    for index in 0..MAX_LIVE_GRANTS_PER_SCOPE {
        assert!(matches!(
            answer(
                &store,
                &grant_request(&format!("grant-{index}"), &format!("route-{index}")),
                NOW
            ),
            RouteGrantAnswer::Granted { .. }
        ));
    }
    assert_eq!(
        refused(answer(
            &store,
            &grant_request("grant-over", "route-over"),
            NOW
        )),
        Some(RouteGrantRefusal::ResourceLimit)
    );
    // A revoked grant no longer counts as live.
    assert!(matches!(
        answer(
            &store,
            &RouteGrantRequest::Revoke {
                workspace_id: WORKSPACE.to_owned(),
                grant_id: "grant-0".to_owned(),
            },
            NOW
        ),
        RouteGrantAnswer::Revoked { .. }
    ));
    assert!(matches!(
        answer(&store, &grant_request("grant-over", "route-over"), NOW),
        RouteGrantAnswer::Granted { .. }
    ));
    let other = JobLedgerStore::new("route-grant-scopes");
    for index in 0..MAX_ROUTE_GRANT_SCOPES {
        let request = RouteGrantRequest::Grant {
            workspace_id: format!("workspace-{index}"),
            grant: grant("grant-remote", "remote-route"),
        };
        assert!(matches!(
            answer(&other, &request, NOW),
            RouteGrantAnswer::Granted { .. }
        ));
    }
    let request = RouteGrantRequest::Grant {
        workspace_id: "workspace-over".to_owned(),
        grant: grant("grant-remote", "remote-route"),
    };
    assert_eq!(
        refused(answer(&other, &request, NOW)),
        Some(RouteGrantRefusal::ResourceLimit)
    );
}

#[test]
fn live_grants_carry_their_counters_and_counting_stops_at_each_bound() {
    let store = JobLedgerStore::new("route-grant-counters");
    for (grant_id, route_id) in [("grant-a", "route-a"), ("grant-b", "route-b")] {
        assert!(matches!(
            answer(&store, &grant_request(grant_id, route_id), NOW),
            RouteGrantAnswer::Granted { .. }
        ));
    }
    assert!(matches!(
        answer(
            &store,
            &RouteGrantRequest::Revoke {
                workspace_id: WORKSPACE.to_owned(),
                grant_id: "grant-b".to_owned(),
            },
            NOW
        ),
        RouteGrantAnswer::Revoked { .. }
    ));
    let runtime = store.try_runtime().unwrap();
    let states = runtime.owner_states();
    let digest = grant("grant-a", "route-a").grant_sha256;
    let revoked = grant("grant-b", "route-b").grant_sha256;
    // Only the live grant is offered, with what was counted against it.
    let live = live_route_grants(&states, WORKSPACE, NOW + 1).unwrap();
    assert_eq!(live.len(), 1);
    assert_eq!((live[0].used_requests, live[0].used_input_tokens), (0, 0));
    assert_eq!(
        count_route_grant_request(&states, WORKSPACE, &digest, 40_000, NOW + 1),
        Ok((1, 40_000))
    );
    assert_eq!(
        count_route_grant_request(&states, WORKSPACE, &digest, 60_001, NOW + 1),
        Err(RouteGrantRefusal::NotAdmitted)
    );
    assert_eq!(
        count_route_grant_request(&states, WORKSPACE, &digest, 60_000, NOW + 1),
        Ok((2, 100_000))
    );
    let live = live_route_grants(&states, WORKSPACE, NOW + 1).unwrap();
    assert_eq!(
        (live[0].used_requests, live[0].used_input_tokens),
        (2, 100_000)
    );
    for (grant_sha256, now, expected) in [
        (&digest, NOW + 1, RouteGrantRefusal::NotAdmitted),
        (&revoked, NOW + 1, RouteGrantRefusal::NotAdmitted),
        (&"e".repeat(64), NOW + 1, RouteGrantRefusal::NotFound),
        (&digest, NOW - 1, RouteGrantRefusal::ClockUnavailable),
    ] {
        assert_eq!(
            count_route_grant_request(&states, WORKSPACE, grant_sha256, 1, now),
            Err(expected)
        );
    }
    // Expired at its expiry: neither offered nor counted.
    let expiry = grant("grant-a", "route-a").expires_at_epoch_ms;
    assert!(
        live_route_grants(&states, WORKSPACE, expiry)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        count_route_grant_request(&states, WORKSPACE, &digest, 1, expiry),
        Err(RouteGrantRefusal::NotAdmitted)
    );
    drop((states, runtime));
    let views = listed(&store, Some(WORKSPACE), expiry);
    assert_eq!(
        views
            .iter()
            .map(|view| (view.grant.grant_id.as_str(), view.state))
            .collect::<Vec<_>>(),
        [
            ("grant-a", RouteGrantState::Expired),
            ("grant-b", RouteGrantState::Revoked)
        ]
    );
    let views = listed(&store, Some(WORKSPACE), NOW + 2);
    assert_eq!(views[0].state, RouteGrantState::Exhausted);
    assert_eq!(views[0].used_input_tokens, 100_000);
}

#[test]
fn a_changed_catalog_is_refused_and_an_answer_must_name_what_was_sent() {
    let store = JobLedgerStore::new("route-grant-integrity");
    let request = grant_request("grant-remote", "remote-route");
    let granted = answer(&store, &request, NOW);
    // An answer for another workspace, another grant, a changed grant or a
    // different decision is not this request's.
    let RouteGrantAnswer::Granted {
        grant: view,
        receipt,
    } = granted.clone()
    else {
        panic!("granted");
    };
    for changed in [
        RouteGrantAnswer::Granted {
            grant: RouteGrantView {
                workspace_id: "workspace-other".to_owned(),
                ..view.clone()
            },
            receipt: receipt.clone(),
        },
        RouteGrantAnswer::Granted {
            grant: RouteGrantView {
                grant: grant("grant-other", "remote-route"),
                ..view.clone()
            },
            receipt: receipt.clone(),
        },
        RouteGrantAnswer::Granted {
            grant: RouteGrantView {
                used_requests: 1,
                ..view.clone()
            },
            receipt: receipt.clone(),
        },
        RouteGrantAnswer::Granted {
            grant: view.clone(),
            receipt: RouteGrantReceipt {
                decision_sha256: "f".repeat(64),
                ..receipt.clone()
            },
        },
        RouteGrantAnswer::Listed {
            grants: vec![view.clone()],
            catalog_revision: 1,
        },
    ] {
        assert!(
            !route_grant_answer_acknowledges(&request, &changed),
            "{changed:?}"
        );
    }
    assert!(!route_grant_answer_acknowledges(
        &RouteGrantRequest::List {
            workspace_id: Some("workspace-other".to_owned())
        },
        &RouteGrantAnswer::Listed {
            grants: vec![view],
            catalog_revision: 1,
        }
    ));
    // A stored catalog changed by a key holder fails its checks.
    let runtime = store.try_runtime().unwrap();
    let states = runtime.owner_states();
    let stored = states.load(OwnerStateName::RouteGrantCatalog).unwrap();
    let text = String::from_utf8(stored.state).unwrap();
    for changed in [
        text.replace("\"used_requests\":0", "\"used_requests\":9"),
        text.replace("\"max_requests\":3", "\"max_requests\":4"),
        text.replace("{\"schema_version\":1", "{\"schema_version\":2"),
        format!("{text} "),
    ] {
        let other = JobLedgerStore::new("route-grant-integrity-changed");
        let changed_runtime = other.try_runtime().unwrap();
        changed_runtime
            .owner_states()
            .commit(OwnerStateName::RouteGrantCatalog, 0, changed.as_bytes())
            .unwrap();
        assert_eq!(
            live_route_grants(&changed_runtime.owner_states(), WORKSPACE, NOW),
            Err(RouteGrantRefusal::StoreIntegrity),
            "{changed}"
        );
    }
    drop((states, runtime));
}

#[test]
fn a_grant_is_shown_before_it_is_sent_and_every_answer_renders_in_both_formats() {
    let sealed = grant("grant-remote", "remote-route");
    let preview = render_route_grant_preview(WORKSPACE, &sealed);
    assert!(preview.starts_with(
        "route grant grant-remote would let remote route remote-route of provider provider-remote"
    ));
    assert!(preview.contains(&format!(
        "receive conversation, workspace_excerpts, tool_outputs from runs in workspace {WORKSPACE}"
    )));
    assert!(
        preview
            .contains("- at most 3 requests and 100000 input tokens, until 2026-01-03T00:00:00Z\n")
    );
    assert!(preview.contains("it may not replace a failed route"));
    assert!(preview.ends_with(
        "This development host offers no remote route yet: no run sends anything off this machine until a host offers this route.\n"
    ));
    let store = JobLedgerStore::new("route-grant-render");
    let granted = answer(&store, &grant_request("grant-remote", "remote-route"), NOW);
    let human = render_route_grant_answer(&granted, false);
    assert!(human.starts_with("route grant kept at catalog revision 1\n"));
    assert!(human.contains(&format!(
        "- {WORKSPACE} grant-remote live: route remote-route of provider provider-remote; data conversation, workspace_excerpts, tool_outputs; 0 of 3 requests and 0 of 100000 input tokens used; granted 2026-01-02T00:00:00Z; expires 2026-01-03T00:00:00Z; digest "
    )));
    let json: serde_json::Value =
        serde_json::from_str(render_route_grant_answer(&granted, true).trim_end()).unwrap();
    assert_eq!(json["type"], "route_grant");
    assert_eq!(json["result"], "granted");
    assert_eq!(json["grant"]["state"], "live");
    assert_eq!(
        render_route_grant_refusal(RouteGrantRefusal::RouteAlreadyGranted, false),
        "route grant refused: route-grant.route-already-granted\n"
    );
    let json: serde_json::Value = serde_json::from_str(
        render_route_grant_refusal(RouteGrantRefusal::NotFound, true).trim_end(),
    )
    .unwrap();
    assert_eq!(json["refusal"], "route-grant.not-found");
    // A host's malformed digest is shown, never sliced past its end.
    let RouteGrantAnswer::Granted { mut grant, receipt } = granted else {
        panic!("granted");
    };
    grant.grant.grant_sha256 = "short".to_owned();
    assert!(
        render_route_grant_answer(&RouteGrantAnswer::Granted { grant, receipt }, false)
            .contains("digest short\n")
    );
    assert_eq!(
        route_grant_refusal_exit(RouteGrantRefusal::RouteAlreadyGranted),
        crate::headless::ClientExitCode::PolicyDenied
    );
}
