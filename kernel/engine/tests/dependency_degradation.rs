use std::collections::BTreeSet;

use agentmage_kernel_engine::dependency_degradation::{
    DependencyDegradationError, DependencyDegradationState, DependencyHealth,
    DependencyPolicyEnvelope, DependencyRequirement, QualifiedDependencySubstitution,
    RuntimeDependency, RuntimeDependencyKind, RuntimeDependencyObservation,
    evaluate_workflow_dependencies,
};

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn every_dependency_family_is_classified_and_available() {
    let kinds = [
        RuntimeDependencyKind::Runtime,
        RuntimeDependencyKind::Parser,
        RuntimeDependencyKind::Model,
        RuntimeDependencyKind::Codec,
        RuntimeDependencyKind::Endpoint,
        RuntimeDependencyKind::Tool,
        RuntimeDependencyKind::Verifier,
        RuntimeDependencyKind::Store,
        RuntimeDependencyKind::ClientFeature,
        RuntimeDependencyKind::OptionalCapability,
    ];
    let dependencies = kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| {
            dependency(
                &format!("dependency-{index}"),
                kind,
                DependencyRequirement::Required,
            )
        })
        .collect::<Vec<_>>();
    let observations = dependencies.iter().map(available).collect::<Vec<_>>();
    let disposition = evaluate_workflow_dependencies(&dependencies, &observations, &[])
        .expect("complete exact inventory is ready");
    assert_eq!(disposition.state, DependencyDegradationState::Ready);
    assert!(disposition.execution_permitted);
    assert_eq!(disposition.dependencies.len(), kinds.len());
}

#[test]
fn required_optional_disabled_and_quarantined_loss_are_visible() {
    let dependencies = vec![
        dependency(
            "required",
            RuntimeDependencyKind::Store,
            DependencyRequirement::Required,
        ),
        dependency(
            "optional",
            RuntimeDependencyKind::Parser,
            DependencyRequirement::Optional,
        ),
        dependency(
            "disabled",
            RuntimeDependencyKind::ClientFeature,
            DependencyRequirement::Optional,
        ),
        dependency(
            "quarantined",
            RuntimeDependencyKind::Model,
            DependencyRequirement::Required,
        ),
    ];
    let mut observations = dependencies.iter().map(available).collect::<Vec<_>>();
    observations[0].health = DependencyHealth::Missing;
    observations[1].health = DependencyHealth::Failed;
    observations[2].health = DependencyHealth::Disabled;
    observations[3].health = DependencyHealth::Quarantined;
    let disposition = evaluate_workflow_dependencies(&dependencies, &observations, &[])
        .expect("losses have closed visible states");
    assert_eq!(
        disposition
            .dependencies
            .iter()
            .map(|item| item.state)
            .collect::<Vec<_>>(),
        [
            DependencyDegradationState::Blocked,
            DependencyDegradationState::Degraded,
            DependencyDegradationState::Disabled,
            DependencyDegradationState::Quarantined,
        ]
    );
    assert!(!disposition.execution_permitted);
    assert_eq!(disposition.state, DependencyDegradationState::Quarantined);
}

#[test]
fn optional_loss_remains_permitted_but_never_silent() {
    let dependencies = vec![dependency(
        "optional-parser",
        RuntimeDependencyKind::Parser,
        DependencyRequirement::Optional,
    )];
    let mut observation = available(&dependencies[0]);
    observation.health = DependencyHealth::Disabled;
    let disposition = evaluate_workflow_dependencies(&dependencies, &[observation], &[])
        .expect("optional disable is a closed visible state");
    assert_eq!(disposition.state, DependencyDegradationState::Disabled);
    assert!(disposition.execution_permitted);
    assert!(disposition.dependencies[0].execution_permitted);
    assert_eq!(
        disposition.dependencies[0].reason_code,
        "runtime.dependency.disabled"
    );
}

#[test]
fn fresh_narrower_stronger_same_kind_substitute_is_explicitly_selected() {
    let source = dependency(
        "primary",
        RuntimeDependencyKind::Runtime,
        DependencyRequirement::Substitutable,
    );
    let mut substitute = dependency(
        "alternate",
        RuntimeDependencyKind::Runtime,
        DependencyRequirement::Optional,
    );
    substitute.policy.authority_classes.remove("write");
    substitute.policy.security_strength += 1;
    substitute.policy.verification_strength += 1;
    let dependencies = vec![source, substitute];
    let mut observations = dependencies.iter().map(available).collect::<Vec<_>>();
    observations[0].health = DependencyHealth::Missing;
    let disposition =
        evaluate_workflow_dependencies(&dependencies, &observations, &[substitution()])
            .expect("fresh stricter replacement is admitted");
    assert!(disposition.execution_permitted);
    assert_eq!(
        disposition.dependencies[0]
            .selected_dependency_id
            .as_deref(),
        Some("alternate")
    );
}

#[test]
fn missing_stale_hidden_cross_kind_or_weaker_substitution_is_denied() {
    let source = dependency(
        "primary",
        RuntimeDependencyKind::Runtime,
        DependencyRequirement::Substitutable,
    );
    let substitute = dependency(
        "alternate",
        RuntimeDependencyKind::Runtime,
        DependencyRequirement::Optional,
    );
    let dependencies = vec![source, substitute];
    let mut observations = dependencies.iter().map(available).collect::<Vec<_>>();
    observations[0].health = DependencyHealth::Missing;
    let base = substitution();

    let mut stale = base.clone();
    stale.qualification_generation += 1;
    let mut hidden = base.clone();
    hidden.user_visible = false;
    let mut unqualified = base.clone();
    unqualified.qualified = false;
    let unavailable = evaluate_workflow_dependencies(&dependencies, &observations, &[])
        .expect("absent substitute is visibly unavailable");
    assert_eq!(
        unavailable.dependencies[0].state,
        DependencyDegradationState::Unavailable
    );
    assert!(!unavailable.execution_permitted);
    for candidate in [vec![stale], vec![hidden], vec![unqualified]] {
        assert_eq!(
            evaluate_workflow_dependencies(&dependencies, &observations, &candidate),
            Err(DependencyDegradationError::SubstitutionDenied)
        );
    }

    let mut cross_kind = dependencies.clone();
    cross_kind[1].kind = RuntimeDependencyKind::Endpoint;
    assert_eq!(
        evaluate_workflow_dependencies(&cross_kind, &observations, std::slice::from_ref(&base)),
        Err(DependencyDegradationError::SubstitutionDenied)
    );
    let mut weaker = dependencies.clone();
    weaker[1].policy.verification_strength = 1;
    weaker[0].policy.verification_strength = 2;
    assert_eq!(
        evaluate_workflow_dependencies(&weaker, &observations, &[base]),
        Err(DependencyDegradationError::SubstitutionDenied)
    );
}

#[test]
fn inventory_and_exact_identity_drift_never_silently_omit() {
    let dependencies = vec![dependency(
        "store",
        RuntimeDependencyKind::Store,
        DependencyRequirement::Required,
    )];
    assert_eq!(
        evaluate_workflow_dependencies(&dependencies, &[], &[]),
        Err(DependencyDegradationError::InventoryMismatch)
    );
    let mut duplicate = dependencies.clone();
    duplicate.push(dependencies[0].clone());
    assert_eq!(
        evaluate_workflow_dependencies(&duplicate, &[available(&dependencies[0])], &[]),
        Err(DependencyDegradationError::InvalidInput)
    );
    let mut drift = available(&dependencies[0]);
    drift.version = "changed".to_owned();
    let disposition = evaluate_workflow_dependencies(&dependencies, &[drift], &[])
        .expect("required identity drift is visibly blocked");
    assert_eq!(disposition.state, DependencyDegradationState::Blocked);
    assert!(!disposition.execution_permitted);
}

fn dependency(
    id: &str,
    kind: RuntimeDependencyKind,
    requirement: DependencyRequirement,
) -> RuntimeDependency {
    RuntimeDependency {
        dependency_id: id.to_owned(),
        kind,
        requirement,
        version: "1.0.0".to_owned(),
        identity_sha256: SHA.to_owned(),
        substitution_policy_sha256: (requirement == DependencyRequirement::Substitutable)
            .then(|| SHA.to_owned()),
        policy: DependencyPolicyEnvelope {
            data_policy_sha256: SHA.to_owned(),
            authority_classes: ["read".to_owned(), "write".to_owned()]
                .into_iter()
                .collect::<BTreeSet<_>>(),
            security_strength: 2,
            verification_strength: 2,
            completion_policy_sha256: SHA.to_owned(),
        },
    }
}

fn available(dependency: &RuntimeDependency) -> RuntimeDependencyObservation {
    RuntimeDependencyObservation {
        dependency_id: dependency.dependency_id.clone(),
        version: dependency.version.clone(),
        identity_sha256: dependency.identity_sha256.clone(),
        health: DependencyHealth::Available,
        qualification_generation: 7,
    }
}

fn substitution() -> QualifiedDependencySubstitution {
    QualifiedDependencySubstitution {
        dependency_id: "primary".to_owned(),
        substitute_dependency_id: "alternate".to_owned(),
        qualification_id: "qualification-7".to_owned(),
        policy_sha256: SHA.to_owned(),
        qualification_generation: 7,
        qualified: true,
        user_visible: true,
        reason_code: "runtime.dependency.explicit-substitute".to_owned(),
    }
}
