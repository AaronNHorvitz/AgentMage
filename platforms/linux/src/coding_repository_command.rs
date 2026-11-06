// SPDX-License-Identifier: Apache-2.0
//! Exact commands in a worktree held after this owner's successful creation.

use super::*;
use crate::{LinuxBoundedCommandExecutor, LinuxCommandManifest};
use agentmage_kernel_contracts::{OperationOutcome, WorkspaceScopePath};
use agentmage_kernel_engine::command_runner::{
    BoundedCommandExecutor, CommandEffectDriver, CommandRegistry, CommandRequest, CommandSpec,
    CommandWorkingDirectory, PreparedCommand, prepare_command, verify_command_receipt,
};

const COMMAND_TOOL: &str = "local-coding-command";
// Kept at this composition boundary: the registered tool validates the exact
// canonical request bytes, rather than accepting caller-supplied command fields.
const COMMAND_REQUEST_SCHEMA: &str = r#"{
  "type":"object","additionalProperties":false,
  "required":["schema_version","command_attempt_id","template_id","template_version","spec_sha256"],
  "properties":{
    "schema_version":{"const":1},
    "command_attempt_id":{"type":"string","minLength":1,"maxLength":128},
    "template_id":{"type":"string","minLength":1,"maxLength":128},
    "template_version":{"type":"string","minLength":1,"maxLength":128},
    "spec_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}
  }
}"#;
pub(super) struct PendingCommand {
    pub(super) prepared: PreparedCommand,
    pub(super) preview: Value,
    pub(super) hash: String,
    pub(super) validation: Option<super::validation::PendingValidation>,
    created_at: u64,
    expires_at: u64,
    deadline: Instant,
}

impl LinuxCodingRepository {
    /// Inspects process-only command state. Canonical authority receipts are
    /// available through `inspect`; output and command previews are not durable.
    pub fn inspect_command(&self) -> Result<Value> {
        self.authority
            .revalidate_root()
            .map_err(|_| err("code.repository.state"))?;
        Ok(
            json!({"version":1,"pending":self.command.as_ref().filter(|p| p.validation.is_none()).map(|p| &p.preview),
            "worktree_root_sha256":self.created.as_ref().map(|c| c.held.root_path_sha256()),
            "ownership_retention":"process-only","output_retention":"response-only",
            "automatic_replay":false,"test_suite_verified":false}),
        )
    }

    /// Prepares an exact person-chosen command. No shell expansion, model
    /// authority or launch. Requires a worktree created successfully by this
    /// service instance and the existing trusted native launcher manifest.
    pub fn prepare_command(&mut self, spec: CommandSpec, now: u64) -> Result<Value> {
        self.verify_created()?;
        let registry = command_registry(spec)?;
        // Verify executable bytes and launcher trust before presenting approval.
        // The same checks repeat immediately before durable consumption.
        native_executor(&registry)?;
        let pending = self.command_preview(&registry, now)?;
        let preview = pending.preview.clone();
        self.command = Some(pending);
        Ok(preview)
    }

    /// Consumes a separate exact decision once. Exit success establishes only
    /// command success, never a parsed test result or repository correctness.
    /// Any uncertain dispatch requires manual inspection, never automatic retry.
    pub fn decide_command(&mut self, digest: &str, approve: bool, now: u64) -> Result<Value> {
        if self
            .command
            .as_ref()
            .is_some_and(|p| p.validation.is_some())
        {
            return Err(err("code.command.validation-choice-required"));
        }
        let Some(pending) = self.take_command(digest, approve, now)? else {
            return Ok(json!({"outcome":"denied","effect":"none"}));
        };
        self.verify_created()?;
        let registry = command_registry(pending.prepared.command().clone())?;
        let executor = native_executor(&registry)?;
        self.run_command(pending, executor, now)
    }

    pub(super) fn verify_created(&self) -> Result<&CreatedWorktree> {
        self.authority
            .revalidate_root()
            .map_err(|_| err("code.repository.state"))?;
        let created = self
            .created
            .as_ref()
            .ok_or(err("code.command.worktree-required"))?;
        created
            .held
            .revalidate()
            .map_err(|_| err("code.command.scope"))?;
        created
            .owned
            .revalidate()
            .map_err(|_| err("code.command.scope"))?;
        LinuxCodingReadScope::verify_root(&created.path, created.held.root_path_sha256())
            .map_err(|_| err("code.command.scope"))?;
        let owned = created.path.parent().ok_or(err("code.command.scope"))?;
        LinuxCodingReadScope::verify_root(owned, created.owned.root_path_sha256())
            .map_err(|_| err("code.command.scope"))?;
        Ok(created)
    }

    pub(super) fn command_preview(
        &self,
        registry: &CommandRegistry,
        now: u64,
    ) -> Result<PendingCommand> {
        let created = self.verify_created()?;
        let mut random = [0; 16];
        if rustix::rand::getrandom(&mut random, rustix::rand::GetRandomFlags::empty()) != Ok(16) {
            return Err(err("code.repository.state"));
        }
        let spec = &registry.commands()[0];
        let prepared = prepare_command(
            registry,
            CommandRequest::new(format!("command-{}", hash(&random)), spec),
        )
        .map_err(|e| err(e.code()))?;
        let expires_at = now
            .checked_add(LIFETIME_MS)
            .ok_or(err("code.repository.clock"))?;
        let mut preview = json!({"version":1,"operation":"execute-command",
            "command":prepared.preview(),"request_sha256":prepared.request_sha256(),
            "worktree_root_sha256":created.held.root_path_sha256(),
            "creation_receipt_sha256":created.creation_receipt_sha256,
            "created_at_epoch_ms":now,"expires_at_epoch_ms":expires_at,
            "warning":"Runs the exact command offline with a read-only worktree and temporary scratch. Output is untrusted and response-only. Exit zero does not verify a test suite. No commit or push. Inspect receipts after a lost reply; never resend approval.",
            "retention":"canonical-authority","preview_retention":"process-only"});
        let digest = hash(&serde_json::to_vec(&preview).map_err(|_| err("code.repository.state"))?);
        preview["preview_sha256"] = json!(digest);
        Ok(PendingCommand {
            prepared,
            validation: None,
            preview,
            hash: digest,
            created_at: now,
            expires_at,
            deadline: Instant::now() + Duration::from_millis(LIFETIME_MS),
        })
    }

    pub(super) fn take_command(
        &mut self,
        digest: &str,
        approve: bool,
        now: u64,
    ) -> Result<Option<PendingCommand>> {
        let pending = self
            .command
            .as_ref()
            .ok_or(err("code.command.choice-required"))?;
        if pending.hash != digest {
            return Err(err("code.command.stale"));
        }
        let pending = self
            .command
            .take()
            .ok_or(err("code.command.choice-required"))?;
        if !approve {
            return Ok(None);
        }
        if now < pending.created_at
            || now >= pending.expires_at
            || Instant::now() >= pending.deadline
        {
            return Err(err("code.command.expired"));
        }
        Ok(Some(pending))
    }

    // Generic only for component tests; public execution always uses the native
    // bounded Linux executor. There is no production injection hook.
    pub(super) fn run_command<
        E: BoundedCommandExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    >(
        &mut self,
        pending: PendingCommand,
        executor: E,
        now: u64,
    ) -> Result<Value> {
        let created = self.verify_created()?;
        if pending.preview["worktree_root_sha256"] != created.held.root_path_sha256()
            || pending.preview["creation_receipt_sha256"] != created.creation_receipt_sha256
        {
            return Err(err("code.command.stale"));
        }
        // Account for manifest verification and other preparation time using the
        // monotonic preview deadline, even if the caller's wall clock is stale.
        let remaining = pending.deadline.saturating_duration_since(Instant::now());
        let elapsed = LIFETIME_MS.saturating_sub(u64::try_from(remaining.as_millis()).unwrap_or(0));
        let dispatch_now = now.max(pending.created_at.saturating_add(elapsed));
        if dispatch_now >= pending.expires_at || remaining.is_zero() {
            return Err(err("code.command.expired"));
        }
        let prepared = pending.prepared;
        let validation = pending.validation;
        // Spend before dispatch, including an uncertain authority/runner result.
        // A lost result never becomes automatic retry authority.
        let id = prepared.preview().command_attempt_id.clone();
        let target = GrantTarget::held_workspace_root(&created.held)
            .map_err(|_| err("code.command.scope"))?;
        let scope = GrantTarget::workspace_scope(
            &created.held,
            WorkspaceScopePath::new(WorkspaceId::from_raw(TOOL), Vec::<String>::new())
                .map_err(|_| err("code.command.scope"))?,
        )
        .map_err(|_| err("code.command.scope"))?;
        let rerun_plan = validation
            .as_ref()
            .map(|validation| self.consume_validation(validation, &id, &pending.hash))
            .transpose()?
            .flatten();
        let exact = ExactEffect {
            id: id.clone(),
            operation: OperationBinding::new(GrantOperation::CommandExecute),
            target,
            scope,
            plan_sha256: prepared.preview().preview_sha256.clone(),
            request_sha256: prepared.request_sha256().into(),
            tool: command_tool(&prepared),
            preview_hash: pending.hash.clone(),
            expires_at: pending.expires_at,
            now: dispatch_now,
        };
        let cancellation = CancellationToken::root(
            BoundaryKind::Kernel,
            TaskId::from_raw(&id),
            CorrelationId::from_raw(&id),
        );
        // Split borrows: the existing driver holds the exact owned-worktree root.
        let created = self
            .created
            .as_ref()
            .ok_or(err("code.command.worktree-required"))?;
        let mut driver =
            CommandEffectDriver::new(executor, &created.held, prepared.clone(), cancellation);
        let receipt = exact.execute(self.authority.authority_mut(), &mut driver)?;
        let command_receipt = driver.take_receipt();
        let output = driver.take_output();
        let failure = driver.take_error();
        drop(driver);
        self.verify_created()
            .map_err(|_| err("code.command.uncertain"))?;
        if let Some(failure) = failure {
            return Err(err(failure.code()));
        }
        if receipt.outcome == OperationOutcome::Uncertain {
            return Err(err("code.command.uncertain"));
        }
        let command_receipt = command_receipt.ok_or(err("code.command.uncertain"))?;
        let output = output.ok_or(err("code.command.uncertain"))?;
        if !verify_command_receipt(&prepared, &command_receipt)
            || command_receipt.stdout_retained_bytes != output.stdout().len() as u64
            || command_receipt.stderr_retained_bytes != output.stderr().len() as u64
        {
            return Err(err("code.command.uncertain"));
        }
        // Byte arrays preserve arbitrary output exactly; clients must escape
        // control characters before displaying untrusted command output.
        let validation_result = validation
            .map(|validation| {
                self.finish_validation(
                    validation,
                    &prepared,
                    &command_receipt,
                    &output,
                    &pending.hash,
                )
            })
            .transpose()?;
        Ok(
            json!({"outcome":receipt.outcome,"receipt":receipt,"command_receipt":command_receipt,
            "stdout":output.stdout(),"stderr":output.stderr(),"output_retention":"response-only",
            "validation":validation_result,"rerun_plan":rerun_plan,
            "test_suite_verified":false,"automatic_replay":false}),
        )
    }
}

pub(super) fn command_registry(spec: CommandSpec) -> Result<CommandRegistry> {
    if spec.working_directory != CommandWorkingDirectory::OwnedWorktree
        || spec.bounds.timeout_ms > 120_000
        || spec.bounds.memory_bytes > 4 * 1024 * 1024 * 1024
        || spec.bounds.cpu_percent > 400
        || spec.bounds.task_count > 64
        || spec.bounds.stdout_bytes > 64 * 1024
        || spec.bounds.stderr_bytes > 64 * 1024
    {
        return Err(err("code.command.bounds"));
    }
    CommandRegistry::build(vec![spec]).map_err(|e| err(e.code()))
}
pub(super) fn native_executor(registry: &CommandRegistry) -> Result<LinuxBoundedCommandExecutor> {
    let manifest = LinuxCommandManifest::verify(
        "/usr/bin/systemd-run",
        "/usr/bin/systemctl",
        "/usr/bin/bwrap",
        registry,
    )
    .map_err(|e| err(e.kind().code()))?;
    LinuxBoundedCommandExecutor::new(manifest).map_err(|e| err(e.kind().code()))
}
fn command_tool(prepared: &PreparedCommand) -> RepositoryTool {
    let mut tool = RepositoryTool::new(prepared.request_bytes());
    let definition = &mut tool.definition;
    definition.tool_id = ToolId::from_raw(COMMAND_TOOL);
    definition.display_name = "Run exact confined command".into();
    definition.description =
        "Run one separately approved offline command in the created worktree".into();
    definition.declared_effects = vec![OperationBinding::new(GrantOperation::CommandExecute)];
    definition.required_grant.operation = OperationBinding::new(GrantOperation::CommandExecute);
    definition.required_grant.target_scope = "exact-held-owned-worktree-root".into();
    definition.input_schema = SchemaReference {
        schema_id: SchemaId::from_raw("command-request-v1"),
        schema_version: 1,
        schema_sha256: hash(COMMAND_REQUEST_SCHEMA.as_bytes()),
    };
    definition.output_schema = SchemaReference {
        schema_id: SchemaId::from_raw("command-receipt-v1"),
        schema_version: 1,
        schema_sha256: hash(include_bytes!(
            "../../../schemas/runtime/command-receipt.schema.json"
        )),
    };
    definition.timeout_ms = prepared.command().bounds.timeout_ms;
    tool
}

#[cfg(test)]
#[path = "coding_repository_command_tests.rs"]
mod tests;
