// Native negative integration through the existing canonical authority owner.
// Synthetic parent clock permits only expired-worker refusal before DNS/HTTP;
// this is not TLS, provider, model or product admission.
mod dispatch_fixture {
    use super::*;
    use agentmage_kernel_contracts::*;
    use agentmage_kernel_engine::authority_transaction::AuthorityTransactionRequest;
    use agentmage_kernel_engine::grants::{DerivedOperationGrantRequest, SessionReadGrantRequest};
    use agentmage_kernel_engine::operational_store::{
        DurableAuthorityRuntime, OperationalStoreKeyError, OperationalStoreKeyProvider,
    };
    use agentmage_kernel_engine::policy::{
        PolicyDocument, PolicyEngine, PolicyEvaluationContext, ScopeRules, ToolPolicyBinding,
    };
    use agentmage_kernel_engine::propagation::CancellationObservationError;
    use agentmage_kernel_engine::public_research::{PublicSearchRequest, PublicSourceType};
    use agentmage_kernel_engine::research_budget::ResearchOperation;
    use agentmage_kernel_engine::research_journal::ResearchBudgetContext;
    use agentmage_kernel_engine::research_plan::{PreparedResearchPlan, ResearchPlanDraft};
    use agentmage_kernel_engine::runtime_artifact::{
        runtime_payload_reference, seal_runtime_artifact_manifest,
    };
    use agentmage_kernel_engine::runtime_event::{runtime_event_persistence, seal_runtime_event};
    use agentmage_kernel_engine::runtime_loop::RuntimePortFailure;
    use agentmage_kernel_engine::tooling::{Tool, ToolRegistry};

    struct Key;
    impl OperationalStoreKeyProvider for Key {
        fn with_key<T>(
            &mut self,
            callback: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(callback(&[42; 32]))
        }
    }
    struct FixedClock(u64);
    impl RuntimeClock for FixedClock {
        fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
            Ok(self.0)
        }
    }
    struct NoCancel;
    impl EffectCancellationObservation for NoCancel {
        fn observe_effect_cancellation(
            &self,
        ) -> Result<Option<CancellationSignal>, CancellationObservationError> {
            Ok(None)
        }
    }
    struct Definition(ToolDefinition);
    impl Tool for Definition {
        fn definition(&self) -> &ToolDefinition {
            &self.0
        }
    }
    fn rules<T: Ord>(one: T) -> ScopeRules<T> {
        ScopeRules {
            allowed: BTreeSet::from([one]),
            denied: BTreeSet::new(),
        }
    }
    fn hash(bytes: &[u8]) -> String {
        hex_digest(&digest_bytes(bytes))
    }
    fn retention() -> RuntimeEventRetention {
        RuntimeEventRetention {
            kind: RuntimeEventRetentionKind::Session,
            expires_at_epoch_ms: None,
        }
    }
    fn next(
        prior: &RuntimeEvent,
        kind: RuntimeEventKind,
        now: u64,
        in_operation: bool,
    ) -> RuntimeEvent {
        let mut event = prior.clone();
        event.event_id =
            RuntimeEventId::from_raw(format!("native-research-event-{}", prior.sequence + 1));
        event.sequence += 1;
        event.occurred_at_epoch_ms = now;
        event.previous_event_sha256 = prior.event_sha256.clone();
        event.causation_event_id = Some(prior.event_id.clone());
        event.payload_reference = None;
        event.turn_id = Some(RuntimeTurnId::from_raw("native-research-turn"));
        event.operation_id =
            in_operation.then(|| RuntimeOperationId::from_raw("runtime-op-not-call-id"));
        event.persistence = runtime_event_persistence(&kind);
        event.kind = kind;
        event
    }

    #[test]
    #[ignore = "canonical SQLCipher/native dual-proof refusal integration; expired worker input, no DNS/HTTP"]
    fn native_dispatch_spends_once_retains_uncertainty_and_never_replays_after_reopen() {
        for prelaunch_expired in [false, true] {
            let root_path = fixture_root("canonical-dispatch");
            let workspace_path = root_path.join("workspace");
            std::fs::create_dir(&workspace_path).unwrap();
            let workspace = crate::authorize_workspace_root(
                &workspace_path,
                WorkspaceId::from_raw("native-research-workspace"),
                WorkspaceAuthorizationId::from_raw("native-research-authorization"),
                AdapterInstanceId::from_raw("native-research-adapter"),
            )
            .unwrap();
            let private_root = crate::LinuxStrictLocalRootInspector::inspect(&root_path).unwrap();
            let mut payloads =
                crate::runtime_artifact_store::LinuxRuntimeArtifactPayloadStore::open(
                    &private_root,
                    crate::runtime_artifact_crypto::derive_artifact_payload_key(&[42; 32]).unwrap(),
                )
                .unwrap();
            let mut runtime = DurableAuthorityRuntime::open(
                &root_path.join("authority.db"),
                private_root.observation(),
                &mut Key,
                1,
            )
            .unwrap();
            let session = SessionId::from_raw("native-research-session");
            let task = TaskId::from_raw("research-probe");
            let run = RuntimeRunId::from_raw("native-research-run");
            let actor = ActorId::from_raw("native-research-actor");
            let action = ActionId::from_raw("native-research-action");
            let correlation = CorrelationId::from_raw("native-research-correlation");
            let tool_id = ToolId::from_raw("native.research.fixture");
            let operation = OperationBinding::new(GrantOperation::NetworkAccess);
            let target = GrantTarget::held_workspace_root(&workspace).unwrap();
            let scope_target = GrantTarget::workspace_scope(
                &workspace,
                WorkspaceScopePath::new(
                    workspace.workspace_id().clone(),
                    std::iter::empty::<&str>(),
                )
                .unwrap(),
            )
            .unwrap();
            let policy = PolicyEngine::new(PolicyDocument {
                schema_version: 1,
                revision: 1,
                actors: rules(actor.clone()),
                tasks: rules(task.clone()),
                actions: rules(action.clone()),
                tools: rules(ToolPolicyBinding {
                    tool_id: tool_id.clone(),
                    tool_version: "1.0.0".into(),
                }),
                operations: rules(operation),
                targets: rules(target.clone()),
                denied_argument_sha256s: BTreeSet::new(),
                denied_preimage_sha256s: BTreeSet::new(),
                network_scopes: rules("https:docs.example.com:443".to_owned()),
                credential_scopes: ScopeRules::deny_all(),
                publication_scopes: ScopeRules::deny_all(),
            })
            .unwrap();
            let context = ResearchBudgetContext {
                session_id: session.clone(),
                task_id: task.clone(),
                run_id: run.clone(),
                policy_sha256: policy.policy_sha256().into(),
            };
            let plan = PreparedResearchPlan::prepare(ResearchPlanDraft {
                schema_version: 1,
                task_id: task.as_str().into(),
                depth: ResearchDepth::Quick,
                network_mode: ResearchNetworkMode::Ask,
                limits: ResearchLimits::ceiling(ResearchDepth::Quick),
                destination_domains: BTreeSet::from(["docs.example.com".into()]),
                queries: vec![PublicSearchRequest {
                    request_id: "native-research-query".into(),
                    query: "synthetic public query".into(),
                    domains: vec!["docs.example.com".into()],
                    recency_days: 30,
                    source_types: vec![PublicSourceType::PrimaryDocumentation],
                    max_results: 1,
                    max_total_bytes: 1024,
                }],
            })
            .unwrap();
            let initial = seal_runtime_event(RuntimeEvent {
                schema_version: CONTRACT_SCHEMA_VERSION,
                event_id: RuntimeEventId::from_raw("native-research-event-0"),
                run_id: run.clone(),
                session_id: session.clone(),
                task_id: task.clone(),
                turn_id: None,
                operation_id: None,
                correlation_id: correlation.clone(),
                causation_event_id: None,
                sequence: 0,
                occurred_at_epoch_ms: 1,
                sensitivity: ContextSensitivity::Private,
                retention: retention(),
                persistence: RuntimeEventPersistenceClass::Correctness,
                policy_id: PolicyId::from_raw("native-research-policy"),
                payload_reference: None,
                kind: RuntimeEventKind::RunStarted {
                    request_sha256: "1".repeat(64),
                },
                previous_event_sha256: "0".repeat(64),
                event_sha256: "0".repeat(64),
            })
            .unwrap();
            runtime.record_runtime_event(initial.clone()).unwrap();
            let bytes = serde_json::to_vec(plan.draft()).unwrap();
            let manifest = seal_runtime_artifact_manifest(RuntimeArtifactManifest {
                schema_version: CONTRACT_SCHEMA_VERSION,
                artifact_id: RuntimeArtifactId::from_raw("native-research-plan"),
                kind: RuntimeArtifactKind::Report,
                payload_sha256: hash(&bytes),
                byte_size: bytes.len() as u64,
                media_type: "application/json".into(),
                sensitivity: ContextSensitivity::Private,
                retention: retention(),
                session_id: session.clone(),
                task_id: task.clone(),
                producer_run_id: run.clone(),
                producer_turn_id: None,
                producer_operation_id: None,
                receipt_id: None,
                policy_id: initial.policy_id.clone(),
                policy_sha256: policy.policy_sha256().into(),
                created_at_epoch_ms: 2,
                integrity: RuntimeArtifactIntegrityState::Verified,
                preview: None,
                manifest_sha256: "0".repeat(64),
            })
            .unwrap();
            let publication = runtime
                .publish_runtime_artifact(&mut payloads, manifest, &mut bytes.as_slice())
                .unwrap();
            let mut published = next(
                &initial,
                RuntimeEventKind::ArtifactCreated {
                    artifact_id: publication.reference.artifact_id.clone(),
                    manifest_sha256: publication.reference.manifest_sha256.clone(),
                },
                3,
                false,
            );
            published.turn_id = None;
            published.payload_reference =
                Some(runtime_payload_reference(&publication.manifest).unwrap());
            published = seal_runtime_event(published).unwrap();
            runtime.record_runtime_event(published.clone()).unwrap();
            runtime
                .open_research_budget(
                    &payloads,
                    &context,
                    &publication.reference,
                    plan.scope(),
                    100,
                )
                .unwrap();
            let prepared = packet_at(101);
            let reservation = runtime
                .reserve_research_request(
                    &payloads,
                    &context,
                    &prepared,
                    ResearchOperation::Visit,
                    101,
                )
                .unwrap();
            let arguments = serde_json::to_vec(prepared.packet().request()).unwrap();
            let schema = SchemaReference {
                schema_id: SchemaId::from_raw("native-research-fixture"),
                schema_version: 1,
                schema_sha256: "2".repeat(64),
            };
            let call = ToolCall {
                schema_version: CONTRACT_SCHEMA_VERSION,
                tool_call_id: ToolCallId::from_raw("research-probe-1"),
                correlation_id: correlation.clone(),
                action_id: action.clone(),
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".into(),
                arguments: ContractPayload {
                    schema: schema.clone(),
                    media_type: "application/json".into(),
                    sha256: hash(&arguments),
                    bytes: arguments,
                },
            };
            let definition = ToolDefinition {
                schema_version: CONTRACT_SCHEMA_VERSION,
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".into(),
                display_name: "Native research refusal fixture".into(),
                description: "Expired input only; no DNS/HTTP".into(),
                input_schema: schema.clone(),
                output_schema: schema,
                risk_level: ToolRiskLevel::Moderate,
                declared_effects: vec![operation],
                required_grant: RequiredGrantTemplate {
                    operation,
                    target_scope: "exact-held-root".into(),
                    single_use: true,
                },
                timeout_ms: 4000,
            };
            let binding = PublicGetEffectBinding::prepare(
                &definition,
                call.clone(),
                &workspace,
                packet_at(101),
            )
            .unwrap();
            let mut registry = ToolRegistry::new();
            registry
                .register_tool(Box::new(Definition(definition)))
                .unwrap();
            let parent = runtime
                .issue_session_read(SessionReadGrantRequest {
                    grant_id: GrantId::from_raw("native-research-parent"),
                    actor_id: actor.clone(),
                    session_id: session.clone(),
                    task_id: task.clone(),
                    targets: vec![scope_target],
                    excluded_targets: vec![],
                    sensitivity: DataSensitivity::Ephemeral,
                    issued_at_epoch_ms: 1,
                    expires_at_epoch_ms: 10000,
                    nonce: GrantNonce::from_raw("native-research-parent-nonce"),
                    maximum_derived_operations: 1,
                    preview_sha256: "3".repeat(64),
                    policy_sha256: context.policy_sha256.clone(),
                })
                .unwrap();
            let approval = ApprovalId::from_raw("native-research-approval");
            let grant = runtime
                .derive_operation(
                    &parent.grant_id,
                    DerivedOperationGrantRequest {
                        grant_id: GrantId::from_raw("native-research-grant"),
                        approval_id: approval.clone(),
                        action_id: action.clone(),
                        action_kind: ActionKind::DeterministicTool,
                        operation,
                        tool_id: tool_id.clone(),
                        tool_version: "1.0.0".into(),
                        targets: vec![target],
                        argument_sha256: call.arguments.sha256.clone(),
                        preimages: vec![],
                        expected_side_effects: vec![GrantSideEffect {
                            operation,
                            target_indexes: vec![0],
                            details_sha256: prepared.packet().sha256().into(),
                        }],
                        rollback_description: "External disclosure cannot be undone".into(),
                        issued_at_epoch_ms: 100,
                        expires_at_epoch_ms: 10000,
                        nonce: GrantNonce::from_raw("native-research-operation-nonce"),
                        preview_sha256: "4".repeat(64),
                        policy_sha256: context.policy_sha256.clone(),
                    },
                )
                .unwrap();
            let turn =
                seal_runtime_event(next(&published, RuntimeEventKind::TurnStarted, 101, false))
                    .unwrap();
            runtime.record_runtime_event(turn.clone()).unwrap();
            let requested = next(
                &turn,
                RuntimeEventKind::ToolRequested {
                    tool_call_id: call.tool_call_id.clone(),
                    arguments_sha256: call.arguments.sha256.clone(),
                },
                102,
                true,
            );
            let requested = seal_runtime_event(requested).unwrap();
            runtime.record_runtime_event(requested.clone()).unwrap();
            let started = next(
                &requested,
                RuntimeEventKind::ToolStarted {
                    tool_call_id: call.tool_call_id.clone(),
                    authority_sha256: hash(&to_canonical_json(&grant).unwrap()),
                },
                103,
                true,
            );
            let started = seal_runtime_event(started).unwrap();
            let transaction_id = AuthorityTransactionId::from_raw("native-research-transaction");
            let request = AuthorityTransactionRequest::new(
                transaction_id.clone(),
                OperationAttemptId::from_raw("native-research-attempt"),
                approval,
                grant.grant_id.clone(),
                call.clone(),
                PolicyEvaluationContext {
                    actor_id: actor,
                    session_id: session,
                    task_id: task,
                    action_id: action,
                    action_kind: ActionKind::DeterministicTool,
                    tool_id,
                    tool_version: "1.0.0".into(),
                    targets: grant.targets.clone(),
                    argument_sha256: call.arguments.sha256.clone(),
                    preimages: vec![],
                    expected_side_effects: grant.expected_side_effects.clone(),
                    preview_sha256: grant.preview_sha256.clone(),
                    now_epoch_ms: 103,
                    network_scope: Some("https:docs.example.com:443".into()),
                    credential_scope: None,
                    publication_scope: None,
                },
                103,
                "1970-01-01T00:00:00.103Z",
            )
            .unwrap();
            let executable = std::path::PathBuf::from(
                std::env::var_os("AGENTMAGE_NATIVE_RESEARCH_TEST_WORKER").unwrap(),
            );
            let artifact = development_worker_artifact_kind(
                &executable,
                DevelopmentWorkerKind::PublicResearch,
            )
            .unwrap();
            let worker_sha256 = artifact.sha256;
            let runtime_files = ["libgcc_s.so.1", "libc.so.6", "ld-linux-x86-64.so.2"]
                .into_iter()
                .map(|name| {
                    LinuxWorkerRuntimeFile::new(
                        std::fs::canonicalize(Path::new("/usr/lib64").join(name)).unwrap(),
                        Path::new("/lib64").join(name),
                    )
                })
                .collect::<Vec<_>>();
            let manifest =
                LinuxPublicResearchManifest::with_worker(artifact, worker_sha256, &runtime_files)
                    .unwrap();
            let runner = LinuxPublicResearchRunner::new(
                manifest,
                LinuxSandboxLimits::new(128 * 1024 * 1024, 8, 25, 4, 128 * 1024).unwrap(),
            )
            .unwrap();
            let mut clock = FixedClock(if prelaunch_expired { 5000 } else { 103 });
            let mut driver = LinuxPublicResearchEffectDriver::new(
                runner, &workspace, binding, &mut clock, &NoCancel,
            );
            let (receipt, pending) = runtime
                .begin_research_effect_with_runtime_event(
                    &registry,
                    &policy,
                    request,
                    &mut driver,
                    started.clone(),
                    &payloads,
                    &context,
                    &prepared,
                    reservation,
                )
                .unwrap();
            let failure = driver.take_failure().unwrap();
            assert_eq!(failure.external_disclosure_uncertain(), !prelaunch_expired);
            assert!(driver.take_result().is_none());
            assert_eq!(
                receipt.outcome,
                if prelaunch_expired {
                    OperationOutcome::Denied
                } else {
                    OperationOutcome::Uncertain
                }
            );
            assert_eq!(
                failure.reason(),
                if prelaunch_expired {
                    "binding"
                } else {
                    "native-nonsuccess"
                }
            );
            let terminal = next(
                &started,
                RuntimeEventKind::ToolFailed {
                    tool_call_id: call.tool_call_id,
                    receipt_id: Some(receipt.receipt_id.clone()),
                    // The observed native reason remains in the driver's result;
                    // events use the existing closed redacted runtime vocabulary.
                    failure_code: "runtime.tool.failed".into(),
                },
                104,
                true,
            );
            runtime
                .finish_effect_with_runtime_event(pending, seal_runtime_event(terminal).unwrap())
                .unwrap();
            let transaction = runtime.current_transaction(&transaction_id).unwrap();
            assert_eq!(transaction.state, AuthorityTransactionState::Terminal);
            assert_eq!(transaction.uncertain_effect, !prelaunch_expired);
            assert_eq!(
                runtime
                    .research_budget_state(&context)
                    .unwrap()
                    .progress
                    .visits,
                1
            );
            drop(driver);
            drop(runtime);
            let mut reopened = DurableAuthorityRuntime::open(
                &root_path.join("authority.db"),
                private_root.observation(),
                &mut Key,
                6000,
            )
            .unwrap();
            assert_eq!(reopened.receipts(), [receipt]);
            assert!(
                reopened
                    .reserve_research_request(
                        &payloads,
                        &context,
                        &prepared,
                        ResearchOperation::Visit,
                        6000
                    )
                    .is_err()
            );
            assert_eq!(
                reopened
                    .research_budget_state(&context)
                    .unwrap()
                    .progress
                    .visits,
                1
            );
            drop(reopened);
            drop(payloads);
            drop(private_root);
            drop(workspace);
            std::fs::remove_dir_all(root_path).unwrap();
        }
    }
}
