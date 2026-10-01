//! The development catalog host (Decision 0130).
//!
//! The catalog host is the second closed operation of the development host.
//! It validates the same disposable activation and serves the authenticated
//! IPC session of its parent like the development host, but it builds no
//! repository composition, model, tool or runtime. It answers only catalog
//! operations: documentation packs and the read of an ended run's stored
//! action histories. Every run operation is refused. It opens the operational
//! store for each operation and closes it afterwards, so nothing it holds
//! outlives an answer except a staged documentation pack import.

use agentmage_kernel_contracts::{
    CancellationId, RuntimeApprovalResponse, RuntimeEventCursor, RuntimeRunId, RuntimeRunRequest,
};
use agentmage_kernel_engine::doc_pack_store::DurableDocPackCatalog;
use agentmage_kernel_engine::run_action_history_store::{
    DurableRunActionHistories, RunActionHistoryStoreError,
};

use crate::coding_action_history::{EndedRunActionHistories, read_ended_run_action_histories};
use crate::coding_doc_packs::{
    DocPackAnswer, DocPackOwner, DocPackRefusal, DocPackRequest, iso_date_of_epoch_ms,
};
use crate::runtime_transport::{
    RuntimePrepareInput, RuntimeTransportError, RuntimeTransportPort, RuntimeTransportStep,
};

/// The store handles of one catalog operation. The store closes when the
/// handles and their owner are dropped.
pub struct CatalogStoreHandles {
    doc_packs: DurableDocPackCatalog,
    histories: DurableRunActionHistories,
    _owner: Box<dyn std::any::Any>,
}

impl CatalogStoreHandles {
    /// The handles of one open store and the value that owns it.
    #[must_use]
    pub fn new(
        doc_packs: DurableDocPackCatalog,
        histories: DurableRunActionHistories,
        owner: Box<dyn std::any::Any>,
    ) -> Self {
        Self {
            doc_packs,
            histories,
            _owner: owner,
        }
    }
}

/// The operational store and clock a catalog host uses.
pub trait CatalogStore {
    /// Opens the store for one operation.
    fn open(&mut self) -> Result<CatalogStoreHandles, RuntimeTransportError>;

    /// The host clock in Unix epoch milliseconds, if it can be read.
    fn now_epoch_ms(&self) -> Option<u64>;
}

/// The catalog host's service over one authenticated session.
pub struct CodingCatalogService<S> {
    store: S,
    doc_packs: DocPackOwner,
}

impl<S> std::fmt::Debug for CodingCatalogService<S> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CodingCatalogService")
            .field("doc_packs", &self.doc_packs)
            .finish_non_exhaustive()
    }
}

impl<S: CatalogStore> CodingCatalogService<S> {
    /// A service with no staged import.
    pub fn new(store: S) -> Self {
        Self {
            store,
            doc_packs: DocPackOwner::default(),
        }
    }
}

impl<S: CatalogStore> RuntimeTransportPort for CodingCatalogService<S> {
    fn prepare(
        &mut self,
        _input: RuntimePrepareInput,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    fn start(
        &mut self,
        _request: RuntimeRunRequest,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    fn advance(
        &mut self,
        _run_id: &RuntimeRunId,
        _request_sha256: &str,
        _after_event_cursor: Option<&RuntimeEventCursor>,
        _response: Option<&RuntimeApprovalResponse>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    fn cancel(
        &mut self,
        _run_id: &RuntimeRunId,
        _request_sha256: &str,
        _cancellation_id: CancellationId,
        _after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    fn release(
        &mut self,
        _run_id: &RuntimeRunId,
        _request_sha256: &str,
    ) -> Result<(), RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    fn ended_run_action_histories(
        &mut self,
        run_id: &RuntimeRunId,
    ) -> Result<EndedRunActionHistories, RuntimeTransportError> {
        let now = self
            .store
            .now_epoch_ms()
            .ok_or(RuntimeTransportError::RuntimeFailed)?;
        let handles = self.store.open()?;
        read_ended_run_action_histories(&handles.histories, run_id.as_str(), now).map_err(|error| {
            match error {
                RunActionHistoryStoreError::InvalidInput => RuntimeTransportError::RequestDenied,
                _ => RuntimeTransportError::RuntimeFailed,
            }
        })
    }

    fn doc_pack(
        &mut self,
        request: DocPackRequest,
    ) -> Result<DocPackAnswer, RuntimeTransportError> {
        let today = self.store.now_epoch_ms().and_then(iso_date_of_epoch_ms);
        let store = &mut self.store;
        let mut held = None;
        let answer = self.doc_packs.answer(
            request,
            &mut || {
                let handles = store.open().map_err(|_| DocPackRefusal::StoreUnavailable)?;
                let catalog = handles.doc_packs.clone();
                held = Some(handles);
                Ok(catalog)
            },
            today.as_deref(),
        );
        drop(held);
        Ok(answer)
    }
}

/// The development catalog host's store: the disposable activation's private
/// operational store, opened under its development key for each operation.
#[cfg(target_os = "linux")]
pub struct DevelopmentCatalogStore {
    activation: crate::coding_development_activation::CodingDevelopmentActivation,
    platform: agentmage_platform_linux::LinuxDevelopmentPlatformAdapter,
}

#[cfg(target_os = "linux")]
impl DevelopmentCatalogStore {
    /// Activates the platform adapter for the validated activation; nothing
    /// else is observed or composed.
    pub fn new(
        activation: crate::coding_development_activation::CodingDevelopmentActivation,
    ) -> Result<Self, &'static str> {
        activation
            .revalidate()
            .map_err(|_| "coding.catalog.activation-failed")?;
        let adapter_id = agentmage_kernel_contracts::AdapterInstanceId::from_raw(format!(
            "coding-development-{}",
            &activation.marker_sha256()[..24]
        ));
        let platform = agentmage_platform_linux::LinuxDevelopmentPlatformAdapter::activate(
            crate::coding_development_activation::CODING_DEVELOPMENT_ACTIVATION,
            activation.marker_sha256().to_owned(),
            adapter_id,
        )
        .map_err(|_| "coding.catalog.platform-failed")?;
        Ok(Self {
            activation,
            platform,
        })
    }
}

#[cfg(target_os = "linux")]
impl CatalogStore for DevelopmentCatalogStore {
    fn open(&mut self) -> Result<CatalogStoreHandles, RuntimeTransportError> {
        self.activation
            .revalidate()
            .map_err(|_| RuntimeTransportError::RequestDenied)?;
        let mut key = crate::coding_development_activation::CodingDevelopmentKeyProvider::open(
            &self.activation,
        )
        .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        let now = self
            .now_epoch_ms()
            .ok_or(RuntimeTransportError::RuntimeFailed)?;
        let authority = agentmage_platform_linux::open_linux_development_authority(
            &self.platform,
            self.activation.state_root(),
            &mut key,
            now,
        )
        .map_err(|_| {
            eprintln!("coding.catalog.store-unavailable");
            RuntimeTransportError::RuntimeFailed
        })?;
        let doc_packs = authority.authority().doc_pack_catalog();
        let histories = authority.authority().run_action_histories();
        Ok(CatalogStoreHandles::new(
            doc_packs,
            histories,
            Box::new(authority),
        ))
    }

    fn now_epoch_ms(&self) -> Option<u64> {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok())
    }
}

#[cfg(all(test, feature = "source-artifacts", feature = "workflow-supervisor"))]
#[path = "coding_catalog_host_tests.rs"]
mod tests;
