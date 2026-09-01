//! Closed runtime feature activation with unavailable surfaces fixed off.

use crate::runtime_tools::NativeRuntimeFeatures;

/// Complete independent activation set for the foundational runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeFeatureActivation {
    /// Prepared artifact ingress and artifact tool registration.
    pub artifact_ingress: bool,
    /// Built-in UTF-8/plain-text extractor.
    pub plain_text_extractor: bool,
    /// Built-in bounded log extractor.
    pub log_extractor: bool,
    /// Future DOCX extractor.
    pub docx_extractor: bool,
    /// Future PDF extractor.
    pub pdf_extractor: bool,
    /// Future XLSX extractor.
    pub xlsx_extractor: bool,
    /// Future optional OCR adapter.
    pub ocr: bool,
    /// Native lexical prepared-source retrieval.
    pub retrieval: bool,
    /// Verified workflow supervisor attachment.
    pub workflow_supervision: bool,
    /// Future bounded model-assisted repair.
    pub model_assisted_repair: bool,
    /// Stable native `@agentmage` participant registration.
    pub native_participant: bool,
    /// Native Language Model Chat Provider compatibility registration.
    pub native_provider_compatibility: bool,
    /// Future artifact-tool MCP exposure.
    pub mcp: bool,
}

impl RuntimeFeatureActivation {
    /// Current implemented local feature set; unavailable later features remain fixed off.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            artifact_ingress: true,
            plain_text_extractor: true,
            log_extractor: true,
            docx_extractor: false,
            pdf_extractor: false,
            xlsx_extractor: false,
            ocr: false,
            retrieval: true,
            workflow_supervision: true,
            model_assisted_repair: false,
            native_participant: true,
            native_provider_compatibility: true,
            mcp: false,
        }
    }

    /// Validates dependency closure and refuses activation of unimplemented surfaces.
    pub const fn validate(self) -> Result<Self, FeatureActivationError> {
        if self.docx_extractor
            || self.pdf_extractor
            || self.xlsx_extractor
            || self.ocr
            || self.model_assisted_repair
            || self.mcp
        {
            return Err(FeatureActivationError::UnavailableFeature);
        }
        if (self.plain_text_extractor || self.log_extractor) && !self.artifact_ingress {
            return Err(FeatureActivationError::DependencyDisabled);
        }
        if self.retrieval && (!self.artifact_ingress || !self.plain_text_extractor) {
            return Err(FeatureActivationError::DependencyDisabled);
        }
        Ok(self)
    }

    /// Projects only authority-neutral native tool registration flags.
    pub const fn native_runtime_features(self) -> NativeRuntimeFeatures {
        NativeRuntimeFeatures {
            artifact_ingress: self.artifact_ingress,
            retrieval: self.retrieval,
        }
    }
}

/// Stable closed feature-activation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureActivationError {
    /// A requested feature has no admitted implementation.
    UnavailableFeature,
    /// An enabled feature depends on an explicitly disabled prerequisite.
    DependencyDisabled,
}

impl FeatureActivationError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnavailableFeature => "runtime.feature.unavailable",
            Self::DependencyDisabled => "runtime.feature.dependency-disabled",
        }
    }
}

impl std::fmt::Display for FeatureActivationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for FeatureActivationError {}

#[cfg(test)]
mod tests {
    use agentmage_capability_read_only::{
        ARTIFACT_TOOL_VERSION, ArtifactToolKind, ReadOnlyToolKind,
    };
    use agentmage_kernel_contracts::ToolId;
    use agentmage_kernel_engine::tooling::ToolRegistry;

    use super::*;
    use crate::runtime_tools::register_read_only_runtime_tools_with_features;

    #[test]
    fn current_activation_is_closed_and_unavailable_features_are_off() {
        let current = RuntimeFeatureActivation::current()
            .validate()
            .expect("current activation is valid");
        assert!(current.artifact_ingress && current.retrieval && current.workflow_supervision);
        assert!(!current.docx_extractor && !current.pdf_extractor && !current.xlsx_extractor);
        assert!(!current.ocr && !current.model_assisted_repair && !current.mcp);
    }

    #[test]
    fn every_unavailable_feature_and_dependency_broadening_fails_closed() {
        for mutate in [
            |value: &mut RuntimeFeatureActivation| value.docx_extractor = true,
            |value: &mut RuntimeFeatureActivation| value.pdf_extractor = true,
            |value: &mut RuntimeFeatureActivation| value.xlsx_extractor = true,
            |value: &mut RuntimeFeatureActivation| value.ocr = true,
            |value: &mut RuntimeFeatureActivation| value.model_assisted_repair = true,
            |value: &mut RuntimeFeatureActivation| value.mcp = true,
        ] {
            let mut value = RuntimeFeatureActivation::current();
            mutate(&mut value);
            assert_eq!(
                value.validate(),
                Err(FeatureActivationError::UnavailableFeature)
            );
        }
        let mut missing_ingress = RuntimeFeatureActivation::current();
        missing_ingress.artifact_ingress = false;
        assert_eq!(
            missing_ingress.validate(),
            Err(FeatureActivationError::DependencyDisabled)
        );
    }

    #[test]
    fn disabled_ingress_and_retrieval_leave_no_tool_registration() {
        let mut activation = RuntimeFeatureActivation::current();
        activation.artifact_ingress = false;
        activation.plain_text_extractor = false;
        activation.log_extractor = false;
        activation.retrieval = false;
        let activation = activation.validate().expect("bounded baseline is valid");
        let mut registry = ToolRegistry::new();
        register_read_only_runtime_tools_with_features(
            &mut registry,
            activation.native_runtime_features(),
        )
        .expect("baseline tools register");
        assert_eq!(registry.list_tools().len(), ReadOnlyToolKind::ALL.len() + 1);
        assert!(ArtifactToolKind::ALL.into_iter().all(|kind| {
            registry
                .get_tool(&ToolId::from_raw(kind.id()), ARTIFACT_TOOL_VERSION)
                .is_none()
        }));
    }
}
