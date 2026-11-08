// SPDX-License-Identifier: Apache-2.0
//! Guided terminal scheduling of the existing separately approved API operations.
//! No tools, permission grants, model transport or execution recovery live here.

use super::*;
use agentmage_kernel_engine::{
    command_runner::{
        CommandReceipt, CommandRegistry, CommandRequest, PreparedCommand, prepare_command,
    },
    validation_result::{ValidationReceipt, plan_failed_test_rerun, verify_validation_receipt},
    validation_template::{ValidationTemplate, verify_validation_template},
};

const MAX_ATTEMPTS: u64 = 3;

fn definition(text: &str) -> Result<Value> {
    if text.len() > MAX_REQUEST {
        return Err(());
    }
    crate::object_json::<crate::coding_repository_api::PrepareValidation>(text.as_bytes())
        .map_err(|_| ())?;
    let body: Value = serde_json::from_str(text).map_err(|_| ())?;
    if body["approve_definition"] != true || !body["prior_receipt_sha256"].is_null() {
        return Err(());
    }
    Ok(body)
}

fn check_preview(
    preview: &Value,
    body: &Value,
    root: &str,
    attempt: u64,
    prior: Option<&str>,
) -> Result<ValidationTemplate> {
    let template: ValidationTemplate =
        serde_json::from_value(preview["validation_template"].clone()).map_err(|_| ())?;
    let parser = body["output_format"]
        .as_str()
        .unwrap_or("agent_mage_json_v1");
    if preview["operation"] != "execute-validation"
        || preview["worktree_root_sha256"] != root
        || !preview["creation_receipt_sha256"]
            .as_str()
            .is_some_and(digest)
        || !preview["preview_sha256"].as_str().is_some_and(digest)
        || preview["attempt"] != attempt
        || preview["maximum_attempts"] != MAX_ATTEMPTS
        || preview["prior_receipt_sha256"] != json!(prior)
        || !verify_validation_template(&template)
        || json!(template.command) != body["command"]
        || json!(template.kind) != body["kind"]
        || json!(template.minimum_test_count) != body["minimum_test_count"]
        || json!(template.parser) != parser
    {
        return Err(());
    }
    prepared_command(preview, &template)?;
    Ok(template)
}

fn prepared_command(preview: &Value, template: &ValidationTemplate) -> Result<PreparedCommand> {
    let registry = CommandRegistry::build(vec![template.command.clone()]).map_err(|_| ())?;
    let prepared = prepare_command(
        &registry,
        CommandRequest::new(
            preview["command"]["command_attempt_id"]
                .as_str()
                .ok_or(())?,
            &template.command,
        ),
    )
    .map_err(|_| ())?;
    if json!(prepared.preview()) != preview["command"]
        || json!(prepared.request_sha256()) != preview["request_sha256"]
    {
        return Err(());
    }
    Ok(prepared)
}

fn observed_receipt(
    result: &Value,
    preview: &Value,
    template: &ValidationTemplate,
) -> Result<ValidationReceipt> {
    let command: CommandReceipt =
        serde_json::from_value(result["command_receipt"].clone()).map_err(|_| ())?;
    let receipt: ValidationReceipt =
        serde_json::from_value(result["validation"]["receipt"].clone()).map_err(|_| ())?;
    let prepared = prepared_command(preview, template)?;
    if !verify_validation_receipt(template, &prepared, &command, &receipt)
        || receipt.approval_sha256 != preview["preview_sha256"].as_str().ok_or(())?
        || result["outcome"] != json!(command.outcome)
        || result["validation"]["attempt"] != preview["attempt"]
        || result["validation"]["maximum_attempts"] != MAX_ATTEMPTS
        || result["validation"]["prior_receipt_sha256"] != preview["prior_receipt_sha256"]
        || result["validation"]["validation_passed"] != receipt.is_full_pass()
    {
        return Err(());
    }
    Ok(receipt)
}

/// Uses an already selected/disclosed source set and already created worktree.
/// A clean validation series is required; interrupted work is inspected manually.
pub(super) fn run(client: &Client, args: &[String]) -> Result<()> {
    let [root, model, bytes, tokens, task, text] = args else {
        return Err(());
    };
    // Refuse piped execution before credentials, requests or source access.
    if !std::io::stdin().is_terminal()
        || !bytes
            .parse::<usize>()
            .is_ok_and(|n| (2048..=32768).contains(&n))
        || !tokens.parse::<u32>().is_ok_and(|n| (1..=4096).contains(&n))
    {
        return Err(());
    }
    let mut body = definition(text)?;
    let selected = selection(client)?;
    if selected["model_context_approved"] != true {
        return Err(());
    }
    let settings = client.json("GET", "/v1/settings", true, None)?;
    if settings["settings_revision"].as_u64().is_none() || settings["endpoint"].as_str().is_none() {
        return Err(());
    }
    let owner = client.json("GET", "/v1/code/repository/edit", true, None)?;
    let identity = owner["worktree_root_sha256"]
        .as_str()
        .filter(|s| digest(s))
        .ok_or(())?;
    LinuxCodingReadScope::verify_root(Path::new(root), identity).map_err(|_| ())?;
    if !owner["pending"].is_null() {
        return Err(());
    }
    let state = client.json("GET", "/v1/code/repository/validation", true, None)?;
    if state["attempts"] != 0
        || state["maximum_attempts"] != MAX_ATTEMPTS
        || !state["latest"].is_null()
        || !state["pending"].is_null()
    {
        return Err(());
    }
    show(
        &json!({"workflow":"guided-worktree","model":model,"task":task,
        "worktree_root_sha256":identity,"selection":selected,"settings":settings,
        "definition":body,"maximum_test_attempts":MAX_ATTEMPTS,
        "warning":"Each read, edit, failure disclosure and test still needs its own exact terminal approval. A lost reply stops this command; inspect the service and disk before continuing. Receipts describe approved-command tests, not an independent task grade. Workflow state is process-only."}),
    )?;
    // Admit the exact native command before any model request. This is only
    // preparation; an edit invalidates it and every execution is prepared anew.
    let preflight = client.json(
        "POST",
        "/v1/code/repository/validation/prepare",
        true,
        Some(body.clone()),
    )?;
    let template = check_preview(&preflight, &body, identity, 1, None)?;
    let creation = preflight["creation_receipt_sha256"].clone();
    let mut prior: Option<String> = None;
    for attempt in 1..=MAX_ATTEMPTS {
        LinuxCodingReadScope::verify_root(Path::new(root), identity).map_err(|_| ())?;
        // Pin settings and selection across the entire guided operation. There
        // is no lease across human waits; server-side exact previews still rule.
        if selection(client)? != selected
            || client.json("GET", "/v1/settings", true, None)? != settings
        {
            return Err(());
        }
        propose(
            client,
            model,
            bytes,
            tokens,
            task,
            Some(root),
            prior.as_deref().map(|receipt| ProposalRepair {
                receipt: Some(receipt),
            }),
        )?;
        // The proposal helper prepared an inert diff after separate read consent.
        // Existing exact diff approval performs the only native write.
        let edit = client.json("GET", "/v1/code/repository/edit", true, None)?;
        if edit["pending"]["worktree_root_sha256"] != identity
            || edit["pending"]["creation_receipt_sha256"] != creation
        {
            return Err(());
        }
        edit_decide_bound(client, Some(root), Some(&edit["pending"]))?;
        client.idle()?;
        body["prior_receipt_sha256"] = json!(prior);
        let preview = client.json(
            "POST",
            "/v1/code/repository/validation/prepare",
            true,
            Some(body.clone()),
        )?;
        if check_preview(&preview, &body, identity, attempt, prior.as_deref())? != template
            || preview["creation_receipt_sha256"] != creation
        {
            return Err(());
        }
        let (observed, result) = command_decide_observed(client, Some(root), true, Some(&preview))?;
        let receipt = observed_receipt(&result, &observed, &template)?;
        show(&json!({"workflow":"guided-worktree","attempt":attempt,
            "receipt_sha256":receipt.receipt_sha256,"status":receipt.status,
            "approved_command_tests_passed":receipt.is_full_pass(),
            "independent_task_grade":false,"current_workspace_verified":false}))?;
        if receipt.is_full_pass() {
            return Ok(());
        }
        // Reuse the kernel's closed eligibility rules. Never interpret model
        // prose, zero exit alone, flaky/malformed output or transport failure as
        // a reason to retry. The service separately enforces the same ceiling.
        if attempt == MAX_ATTEMPTS {
            return Err(());
        }
        plan_failed_test_rerun(
            &template,
            &receipt,
            "guided-repair-eligibility",
            "0".repeat(64),
        )
        .map_err(|_| ())?;
        prior = Some(receipt.receipt_sha256);
        client.idle()?;
    }
    Err(())
}
