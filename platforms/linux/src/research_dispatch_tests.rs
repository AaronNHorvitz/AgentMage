// Existing canonical authority owner, two original refusal cases and an explicitly
// selected connected native source case. No provider/model/product qualification.
mod dispatch_fixture {
    use super::*;
    use agentmage_kernel_contracts::*;
    use agentmage_kernel_engine::authority_transaction::AuthorityTransactionRequest;
    use agentmage_kernel_engine::grants::{DerivedOperationGrantRequest, SessionReadGrantRequest};
    use agentmage_kernel_engine::operational_store::{
        DurableAuthorityRuntime, OperationalStoreKeyError, OperationalStoreKeyProvider,
        PendingRuntimeEffectCommit,
    };
    use agentmage_kernel_engine::policy::{
        PolicyDocument, PolicyEngine, PolicyEvaluationContext, ScopeRules, ToolPolicyBinding,
    };
    use agentmage_kernel_engine::propagation::CancellationObservationError;
    use agentmage_kernel_engine::public_research::{PublicSearchRequest, PublicSourceType};
    use agentmage_kernel_engine::research_fetch::PublicSearchEndpoint;
    use agentmage_kernel_engine::research_journal::ResearchBudgetContext;
    use agentmage_kernel_engine::research_plan::{PreparedResearchPlan, ResearchPlanDraft};
    use agentmage_kernel_engine::research_retrieval::{
        PublicGetArtifactBundle, PublicGetArtifactOutput,
    };
    use agentmage_kernel_engine::runtime_artifact::{
        runtime_artifact_ref, runtime_payload_reference, seal_runtime_artifact_manifest,
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
    #[derive(Clone, Copy)]
    enum NativeCase {
        ExpiredWorker,
        ExpiredParent,
        PublicSource,
    }
    impl NativeCase {
        fn connected(self) -> bool {
            matches!(self, Self::PublicSource)
        }
        fn now(self, synthetic: u64) -> u64 {
            if self.connected() {
                u64::try_from(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis(),
                )
                .unwrap()
            } else {
                synthetic
            }
        }
        fn domain(self) -> &'static str {
            if self.connected() {
                "example.com"
            } else {
                "docs.example.com"
            }
        }
        fn prepared(
            self,
            plan: &PreparedResearchPlan,
            task_started: u64,
            prepared_at: u64,
        ) -> PreparedPublicGet {
            if !self.connected() {
                return packet_at(101);
            }
            PreparedPublicGet::prepare(
                plan.scope(),
                PublicGetDraft {
                    schema_version: 1,
                    operation_id: "research-probe-1".into(),
                    target: PublicGetTarget {
                        domain: self.domain().into(),
                        path: "/".into(),
                        query: vec![],
                    },
                    maximum_response_bytes: 8 * 1024,
                    redirect_limit: 0,
                    timeout_ms: 4000,
                },
                task_started,
                prepared_at,
            )
            .unwrap()
        }
    }
    struct NativeFixtureClock(NativeCase);
    impl RuntimeClock for NativeFixtureClock {
        fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
            Ok(self.0.now(if matches!(self.0, NativeCase::ExpiredParent) {
                5000
            } else {
                103
            }))
        }
    }

    // Test-only full-source assembly uses the existing canonical owners.
    include!("research_native_source_fixture.rs");

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
        for case in [NativeCase::ExpiredWorker, NativeCase::ExpiredParent] {
            exercise_native_dispatch(case);
        }
    }

    #[test]
    #[ignore = "explicit real public native HTTPS plus canonical source/reopen/lifecycle; no provider or model"]
    fn native_public_source_is_bound_to_canonical_receipt_and_remains_lifecycle_checked() {
        // Even a broad --ignored invocation must not accidentally perform the
        // newly added connected case. The explicit bounded campaign opts in.
        assert_eq!(
            std::env::var("AGENTMAGE_NATIVE_RESEARCH_CONNECTED_CASE")
                .ok()
                .as_deref(),
            Some("public-example-get-v1"),
            "connected native case requires explicit campaign selection"
        );
        exercise_native_dispatch(NativeCase::PublicSource);
    }

    fn exercise_native_dispatch(case: NativeCase) {
        let prelaunch_expired = matches!(case, NativeCase::ExpiredParent);
        let initial_at = case.now(1);
        let expires_at = if case.connected() {
            initial_at.checked_add(10_000).unwrap()
        } else {
            10_000
        };
        let network_scope = format!("https:{}:443", case.domain());
        let root_path = fixture_root("canonical-dispatch");
        if case.connected() {
            // Private campaign output only. Keep successful and failed canonical
            // state for inspection; the supervisor still must reap its worker.
            eprintln!("native-connected-evidence-root={}", root_path.display());
        }
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
        let mut payloads = crate::runtime_artifact_store::LinuxRuntimeArtifactPayloadStore::open(
            &private_root,
            crate::runtime_artifact_crypto::derive_artifact_payload_key(&[42; 32]).unwrap(),
        )
        .unwrap();
        let mut runtime = DurableAuthorityRuntime::open(
            &root_path.join("authority.db"),
            private_root.observation(),
            &mut Key,
            initial_at,
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
            WorkspaceScopePath::new(workspace.workspace_id().clone(), std::iter::empty::<&str>())
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
            network_scopes: rules(network_scope.clone()),
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
            schema_version: 2,
            task_id: task.as_str().into(),
            depth: ResearchDepth::Quick,
            network_mode: ResearchNetworkMode::Ask,
            limits: ResearchLimits::ceiling(ResearchDepth::Quick),
            // The provider endpoint has its own domain; source visits use the case domain.
            destination_domains: BTreeSet::from([
                case.domain().into(),
                "search.example.com".into(),
            ]),
            queries: vec![PublicSearchRequest {
                request_id: "native-research-query".into(),
                query: "synthetic public query".into(),
                domains: vec![case.domain().into()],
                recency_days: 30,
                source_types: vec![PublicSourceType::PrimaryDocumentation],
                max_results: 1,
                max_total_bytes: 1024,
            }],
            search_endpoint: Some(PublicSearchEndpoint {
                domain: "search.example.com".into(),
                path: "/search".into(),
                query_field: "q".into(),
                fixed_fields: vec![],
            }),
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
            occurred_at_epoch_ms: initial_at,
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
            created_at_epoch_ms: case.now(2),
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
            case.now(3),
            false,
        );
        published.turn_id = None;
        published.payload_reference =
            Some(runtime_payload_reference(&publication.manifest).unwrap());
        published = seal_runtime_event(published).unwrap();
        runtime.record_runtime_event(published.clone()).unwrap();
        let budget_started = case.now(100);
        runtime
            .open_research_budget(
                &payloads,
                &context,
                &publication.reference,
                plan.scope(),
                budget_started,
            )
            .unwrap();
        let prepared_at = case.now(101);
        let prepared = case.prepared(&plan, budget_started, prepared_at);
        let reservation = runtime
            .reserve_research_request(
                &payloads,
                &context,
                &prepared,
                prepared_at,
            )
            .unwrap();
        let reservation_sha256 = reservation.reservation_sha256().to_owned();
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
            display_name: "Explicit native research fixture".into(),
            description: "Ignored exact-case test; not product admission".into(),
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
            case.prepared(&plan, budget_started, prepared_at),
        )
        .unwrap();
        let output_schema = definition.output_schema.clone();
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
                issued_at_epoch_ms: case.now(1),
                expires_at_epoch_ms: expires_at,
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
                    issued_at_epoch_ms: case.now(100),
                    expires_at_epoch_ms: expires_at,
                    nonce: GrantNonce::from_raw("native-research-operation-nonce"),
                    preview_sha256: "4".repeat(64),
                    policy_sha256: context.policy_sha256.clone(),
                },
            )
            .unwrap();
        let turn = seal_runtime_event(next(
            &published,
            RuntimeEventKind::TurnStarted,
            case.now(101),
            false,
        ))
        .unwrap();
        runtime.record_runtime_event(turn.clone()).unwrap();
        let requested = next(
            &turn,
            RuntimeEventKind::ToolRequested {
                tool_call_id: call.tool_call_id.clone(),
                arguments_sha256: call.arguments.sha256.clone(),
            },
            case.now(102),
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
            case.now(103),
            true,
        );
        let started = seal_runtime_event(started).unwrap();
        let transaction_id = AuthorityTransactionId::from_raw("native-research-transaction");
        // Authority preflight requires the exact same trusted consumption and
        // ToolStarted timestamp; do not independently resample these two fields.
        let consumed_at = started.occurred_at_epoch_ms;
        let occurred_at = if case.connected() {
            format!("epoch-ms:{consumed_at}")
        } else {
            "1970-01-01T00:00:00.103Z".into()
        };
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
                now_epoch_ms: consumed_at,
                network_scope: Some(network_scope),
                credential_scope: None,
                publication_scope: None,
            },
            consumed_at,
            &occurred_at,
        )
        .unwrap();
        let executable = std::path::PathBuf::from(
            std::env::var_os("AGENTMAGE_NATIVE_RESEARCH_TEST_WORKER").unwrap(),
        );
        let artifact =
            development_worker_artifact_kind(&executable, DevelopmentWorkerKind::PublicResearch)
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
        let expected_native = runner.native_identity().clone();
        if case.connected() {
            eprintln!(
                "native-connected-pins worker={} manifest={} confinement={}",
                expected_native.worker_sha256,
                expected_native.manifest_sha256,
                expected_native.confinement_sha256
            );
        }
        let mut clock = NativeFixtureClock(case);
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
        if case.connected() {
            // Do not print destination/source bodies. Preserve native refusal in
            // the retained test output; a refusal is not converted to success.
            if let Some(failure) = driver.take_failure() {
                // Preserve the real failed receipt and terminal event before
                // failing this diagnostic. The disposable evidence directory
                // deliberately remains available after the assertion failure.
                let terminal = seal_runtime_event(next(
                    &started,
                    RuntimeEventKind::ToolFailed {
                        tool_call_id: call.tool_call_id.clone(),
                        receipt_id: Some(receipt.receipt_id.clone()),
                        failure_code: "runtime.tool.failed".into(),
                    },
                    case.now(0),
                    true,
                ))
                .unwrap();
                runtime
                    .finish_effect_with_runtime_event(pending, terminal)
                    .unwrap();
                panic!(
                    "connected source refused: outcome={:?}, reason={}, uncertainty={}",
                    receipt.outcome,
                    failure.reason(),
                    failure.external_disclosure_uncertain()
                );
            }
            let native_result = driver.take_result().unwrap();
            drop(driver);
            assert_eq!(native_result.native_identity(), &expected_native);
            assert_eq!(receipt.outcome, OperationOutcome::Succeeded);
            assert!(!native_result.response().body().is_empty());
            let body_sha256 = native_result.response().observation().body_sha256.clone();
            let (bundle, frame) = retain_native_source(
                &mut runtime,
                &mut payloads,
                &context,
                pending,
                &NativeSourceAssembly {
                    call: &call,
                    started: &started,
                    receipt: &receipt,
                    transaction_id: &transaction_id,
                    reservation_sha256: &reservation_sha256,
                    packet: prepared.packet(),
                    result: &native_result,
                    output_schema: &output_schema,
                },
                &mut clock,
            );
            let transaction = runtime.current_transaction(&transaction_id).unwrap();
            assert_eq!(transaction.state, AuthorityTransactionState::Terminal);
            assert!(!transaction.uncertain_effect);
            let read = |runtime: &mut DurableAuthorityRuntime,
                        source_payloads: &crate::runtime_artifact_store::LinuxRuntimeArtifactPayloadStore,
                        now_epoch_ms| {
                runtime.read_public_get_source(
                    source_payloads,
                    &registry,
                    &agentmage_kernel_engine::research_retrieval::PublicGetReadRequest {
                        context: &context,
                        bundle: &bundle,
                        expected_native: &expected_native,
                        now_epoch_ms,
                    },
                )
            };
            let before = runtime.research_budget_state(&context).unwrap();
            let source = read(&mut runtime, &payloads, case.now(0)).unwrap();
            assert_eq!(source.response().observation().body_sha256, body_sha256);
            assert_eq!(source.receipt_id(), &receipt.receipt_id);
            assert_eq!(
                before.head_sha256,
                runtime.research_budget_state(&context).unwrap().head_sha256
            );
            drop(source);
            drop(runtime);
            drop(payloads);
            let payloads = crate::runtime_artifact_store::LinuxRuntimeArtifactPayloadStore::open(
                &private_root,
                crate::runtime_artifact_crypto::derive_artifact_payload_key(&[42; 32]).unwrap(),
            )
            .unwrap();
            let mut reopened = DurableAuthorityRuntime::open(
                &root_path.join("authority.db"),
                private_root.observation(),
                &mut Key,
                case.now(0),
            )
            .unwrap();
            assert_eq!(reopened.receipts(), [receipt]);
            assert_eq!(
                read(&mut reopened, &payloads, case.now(0))
                    .unwrap()
                    .response()
                    .observation()
                    .body_sha256,
                body_sha256
            );
            assert!(
                reopened
                    .reserve_research_request(
                        &payloads,
                        &context,
                        &prepared,
                        case.now(0),
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
            reopened
                .release_runtime_artifact(
                    &context.session_id,
                    &context.task_id,
                    &context.policy_sha256,
                    &frame,
                    case.now(0),
                )
                .unwrap();
            assert!(read(&mut reopened, &payloads, case.now(0)).is_err());
            eprintln!(
                "native-connected-source: receipt/full-bytes/reopen/no-replay/release verified body={body_sha256} body_bytes={} frame_bytes={} worker_elapsed_ms={}",
                native_result.response().body().len(),
                native_result.response().frame().len(),
                native_result.response().observation().completed_epoch_ms
                    - native_result.response().observation().started_epoch_ms
            );
            drop(reopened);
            drop(payloads);
            drop(private_root);
            drop(workspace);
            // Retain encrypted canonical state and full source artifacts, not
            // merely this success line. Temporary worker roots are separately
            // supervised/cleaned before publication. No process remains here.
            return;
        }
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
