//! Fail-closed model acquisition and lifecycle state machine.
#![allow(missing_docs)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagementIntent {
    List,
    Explain,
    Recommend,
    Download,
    Import,
    Resume,
    Verify,
    Activate,
    Compare,
    Cancel,
    RollBack,
    Remove,
    CleanStorage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcquisitionPlan {
    pub intent: ManagementIntent,
    pub profile_id: String,
    pub publisher: String,
    pub license_sha256: String,
    pub source_host: String,
    pub artifact_sha256: String,
    pub tokenizer_sha256: String,
    pub template_sha256: String,
    pub codec_id: String,
    pub runtime_id: String,
    pub context_id: String,
    pub decoding_id: String,
    pub modality: String,
    pub expected_bytes: u64,
    pub required_free_bytes: u64,
    pub available_free_bytes: u64,
    pub hardware_evidence_sha256: String,
    pub destination_sha256: String,
    pub checks_sha256: String,
    pub limitations_sha256: String,
    pub rollback_sha256: String,
    pub catalog_revision: String,
    pub policy_revision: String,
    pub support_revision: String,
    pub preview_digest: String,
    pub confirmed_digest: String,
    pub catalog_approved: bool,
    pub compatible: bool,
    pub measured_hardware_fit: bool,
    pub network_required: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstallerScope {
    pub plan_count: u8,
    pub workspace_authority: bool,
    pub session_authority: bool,
    pub provider_authority: bool,
    pub credential_authority: bool,
    pub shell_authority: bool,
    pub inference_authority: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleStage {
    Quarantine,
    Download,
    Import,
    Verify,
    Scan,
    SelfTest,
    Activate,
    RollBack,
    Remove,
    CleanStorage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StageEvidence {
    pub stage: LifecycleStage,
    pub plan_digest_matches: bool,
    pub artifact_digest_matches: bool,
    pub signature_valid: bool,
    pub scan_passed: bool,
    pub format_valid: bool,
    pub runtime_self_test_passed: bool,
    pub postconditions_passed: bool,
    pub interrupted: bool,
    pub cleanup_complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelLifecycle {
    plan_digest: String,
    quarantined: bool,
    verified: bool,
    active_profile: Option<String>,
    prior_profile: Option<String>,
    partial_activation: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagementError {
    InvalidPlan,
    StaleConfirmation,
    InsufficientResources,
    ExcessAuthority,
    WrongStage,
    VerificationFailed,
    Interrupted,
}

fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 4096
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn validate_plan(plan: &AcquisitionPlan) -> Result<(), ManagementError> {
    let identities = [
        &plan.profile_id,
        &plan.publisher,
        &plan.source_host,
        &plan.codec_id,
        &plan.runtime_id,
        &plan.context_id,
        &plan.decoding_id,
        &plan.modality,
        &plan.catalog_revision,
        &plan.policy_revision,
        &plan.support_revision,
    ];
    let digests = [
        &plan.license_sha256,
        &plan.artifact_sha256,
        &plan.tokenizer_sha256,
        &plan.template_sha256,
        &plan.hardware_evidence_sha256,
        &plan.destination_sha256,
        &plan.checks_sha256,
        &plan.limitations_sha256,
        &plan.rollback_sha256,
        &plan.preview_digest,
        &plan.confirmed_digest,
    ];
    if identities.iter().any(|value| !id(value))
        || digests.iter().any(|value| !digest(value))
        || plan.expected_bytes == 0
        || plan.required_free_bytes < plan.expected_bytes
        || !plan.catalog_approved
        || !plan.compatible
        || !plan.measured_hardware_fit
    {
        return Err(ManagementError::InvalidPlan);
    }
    if plan.available_free_bytes < plan.required_free_bytes {
        return Err(ManagementError::InsufficientResources);
    }
    if plan.preview_digest != plan.confirmed_digest {
        return Err(ManagementError::StaleConfirmation);
    }
    Ok(())
}

pub fn validate_installer_scope(scope: InstallerScope) -> Result<(), ManagementError> {
    if scope.plan_count != 1
        || scope.workspace_authority
        || scope.session_authority
        || scope.provider_authority
        || scope.credential_authority
        || scope.shell_authority
        || scope.inference_authority
    {
        Err(ManagementError::ExcessAuthority)
    } else {
        Ok(())
    }
}

impl ModelLifecycle {
    pub fn begin(
        plan: &AcquisitionPlan,
        prior_profile: Option<String>,
    ) -> Result<Self, ManagementError> {
        validate_plan(plan)?;
        Ok(Self {
            plan_digest: plan.confirmed_digest.clone(),
            quarantined: true,
            verified: false,
            active_profile: prior_profile.clone(),
            prior_profile,
            partial_activation: false,
        })
    }

    pub fn apply(
        &mut self,
        profile_id: &str,
        evidence: StageEvidence,
    ) -> Result<(), ManagementError> {
        if evidence.interrupted {
            self.partial_activation = false;
            return Err(ManagementError::Interrupted);
        }
        if !evidence.plan_digest_matches || !evidence.artifact_digest_matches {
            return Err(ManagementError::VerificationFailed);
        }
        match evidence.stage {
            LifecycleStage::Quarantine | LifecycleStage::Download | LifecycleStage::Import => {
                if !self.quarantined {
                    return Err(ManagementError::WrongStage);
                }
            }
            LifecycleStage::Verify | LifecycleStage::Scan | LifecycleStage::SelfTest => {
                if !self.quarantined
                    || !evidence.signature_valid
                    || !evidence.scan_passed
                    || !evidence.format_valid
                    || !evidence.runtime_self_test_passed
                {
                    return Err(ManagementError::VerificationFailed);
                }
                self.verified = true;
            }
            LifecycleStage::Activate => {
                if !self.quarantined || !self.verified || !evidence.postconditions_passed {
                    self.active_profile = self.prior_profile.clone();
                    self.partial_activation = false;
                    return Err(ManagementError::VerificationFailed);
                }
                self.active_profile = Some(profile_id.to_owned());
                self.quarantined = false;
            }
            LifecycleStage::RollBack => {
                self.active_profile = self.prior_profile.clone();
                self.quarantined = false;
                self.partial_activation = false;
            }
            LifecycleStage::Remove => {
                if self.active_profile.as_deref() == Some(profile_id) {
                    self.active_profile = None;
                }
            }
            LifecycleStage::CleanStorage => {
                if !evidence.cleanup_complete {
                    return Err(ManagementError::VerificationFailed);
                }
                self.quarantined = false;
            }
        }
        Ok(())
    }

    pub fn active_profile(&self) -> Option<&str> {
        self.active_profile.as_deref()
    }

    pub const fn partial_activation(&self) -> bool {
        self.partial_activation
    }

    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_confirmation_and_excess_authority_are_inert() {
        let mut plan = valid_plan();
        plan.confirmed_digest = "b".repeat(64);
        assert_eq!(
            validate_plan(&plan),
            Err(ManagementError::StaleConfirmation)
        );
        assert_eq!(
            validate_installer_scope(InstallerScope {
                plan_count: 1,
                workspace_authority: true,
                session_authority: false,
                provider_authority: false,
                credential_authority: false,
                shell_authority: false,
                inference_authority: false,
            }),
            Err(ManagementError::ExcessAuthority)
        );
    }

    #[test]
    fn activation_is_atomic_and_preserves_prior_profile() {
        let plan = valid_plan();
        let mut lifecycle = ModelLifecycle::begin(&plan, Some("prior".into())).expect("valid plan");
        let mut evidence = evidence(LifecycleStage::Verify);
        lifecycle
            .apply("next", evidence)
            .expect("verified quarantine");
        evidence.stage = LifecycleStage::Activate;
        evidence.postconditions_passed = false;
        assert_eq!(
            lifecycle.apply("next", evidence),
            Err(ManagementError::VerificationFailed)
        );
        assert_eq!(lifecycle.active_profile(), Some("prior"));
        assert!(!lifecycle.partial_activation());
        evidence.postconditions_passed = true;
        lifecycle
            .apply("next", evidence)
            .expect("atomic activation");
        assert_eq!(lifecycle.active_profile(), Some("next"));
    }

    #[test]
    fn every_interruption_retains_no_partial_activation() {
        for stage in [
            LifecycleStage::Quarantine,
            LifecycleStage::Download,
            LifecycleStage::Import,
            LifecycleStage::Verify,
            LifecycleStage::Scan,
            LifecycleStage::SelfTest,
            LifecycleStage::Activate,
            LifecycleStage::RollBack,
            LifecycleStage::Remove,
            LifecycleStage::CleanStorage,
        ] {
            let plan = valid_plan();
            let mut lifecycle = ModelLifecycle::begin(&plan, Some("prior".into())).expect("valid");
            let mut observed = evidence(stage);
            observed.interrupted = true;
            assert_eq!(
                lifecycle.apply("next", observed),
                Err(ManagementError::Interrupted)
            );
            assert_eq!(lifecycle.active_profile(), Some("prior"));
            assert!(!lifecycle.partial_activation());
        }
    }

    fn valid_plan() -> AcquisitionPlan {
        AcquisitionPlan {
            intent: ManagementIntent::Download,
            profile_id: "profile".into(),
            publisher: "publisher".into(),
            license_sha256: "1".repeat(64),
            source_host: "source.example".into(),
            artifact_sha256: "2".repeat(64),
            tokenizer_sha256: "3".repeat(64),
            template_sha256: "4".repeat(64),
            codec_id: "codec".into(),
            runtime_id: "runtime".into(),
            context_id: "context".into(),
            decoding_id: "decoding".into(),
            modality: "text".into(),
            expected_bytes: 10,
            required_free_bytes: 20,
            available_free_bytes: 30,
            hardware_evidence_sha256: "5".repeat(64),
            destination_sha256: "6".repeat(64),
            checks_sha256: "7".repeat(64),
            limitations_sha256: "8".repeat(64),
            rollback_sha256: "9".repeat(64),
            catalog_revision: "catalog".into(),
            policy_revision: "policy".into(),
            support_revision: "support".into(),
            preview_digest: "a".repeat(64),
            confirmed_digest: "a".repeat(64),
            catalog_approved: true,
            compatible: true,
            measured_hardware_fit: true,
            network_required: true,
        }
    }

    fn evidence(stage: LifecycleStage) -> StageEvidence {
        StageEvidence {
            stage,
            plan_digest_matches: true,
            artifact_digest_matches: true,
            signature_valid: true,
            scan_passed: true,
            format_valid: true,
            runtime_self_test_passed: true,
            postconditions_passed: true,
            interrupted: false,
            cleanup_complete: true,
        }
    }
}
