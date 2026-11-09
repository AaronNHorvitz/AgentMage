// SPDX-License-Identifier: Apache-2.0
//! Terminal record, release and separately approved original-checkout transfer.

use super::*;
use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

pub(super) fn run(client: &Client, args: &[String]) -> Result<()> {
    let [seconds, run, hash, original, worktree, task, paths @ ..] = args else {
        return Err(());
    };
    endpoint::binding(run, hash)?;
    // Inspect directory metadata only before offering irreversible release.
    // Refuse aliases without resolving them into an accepted selection. The
    // original owner still checks held identities and all source eligibility.
    for root in [original, worktree] {
        let root = Path::new(root);
        if !root.is_absolute()
            || root.as_os_str().len() > 4096
            || root.components().count() > 256
            || root.components().any(|part| {
                !matches!(
                    part,
                    std::path::Component::RootDir | std::path::Component::Normal(_)
                )
            })
            || !std::fs::symlink_metadata(root).map_err(|_| ())?.is_dir()
            || std::fs::canonicalize(root).map_err(|_| ())?.as_os_str() != root.as_os_str()
        {
            return Err(());
        }
    }
    if Path::new(original).starts_with(worktree)
        || Path::new(worktree).starts_with(original)
        || task.trim().is_empty()
        || task.len() > 4096
        || !(1..=16).contains(&paths.len())
    {
        return Err(());
    }
    let mut unique = BTreeSet::new();
    for path in paths {
        if path.len() > 256
            || !path
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
            || path
                .split('/')
                .any(|part| part.eq_ignore_ascii_case(".git"))
            || !unique.insert(path.to_ascii_lowercase())
            || WorkspacePath::new(WorkspaceId::from_raw("local-coding-read"), path.split('/'))
                .is_err()
        {
            return Err(());
        }
    }
    workflow::bounded(client, seconds.parse().map_err(|_| ())?, |client| {
        let request = endpoint::checked_selection(client, run, hash, true)?;
        if request.mode != agentmage_kernel_contracts::RuntimeSessionMode::ControlledWrite {
            return Err(());
        }
        // This must finish and flush before release is offered. The caller may
        // retain stdout; no file is created and no durability is implied.
        record::export_observed(client, request.clone())?;
        client.confirm(
            &json!({"request":request,"repository":original,"worktree":worktree,
                "paths":paths,"task":task,
                "warning":"The complete terminal event record was emitted to stdout, not saved to a file. Retain it before continuing. Releases this exact run and its process-only events and model ports. Requires a service started for original review; ordinary services discard ownership. Fresh read consent and exact diff approval follow. Release grants no write, test pass, new model run, commit or push. After a lost reply inspect service state and disk; never repeat automatically."}),
            &format!("release {hash}"),
        )?;
        let released = client.json(
            "POST",
            "/v1/code/endpoint/release",
            true,
            Some(endpoint::binding(run, hash)?),
        )?;
        if released != json!({"released":true}) {
            return Err(());
        }
        show(&released)?;
        let preview =
            original_apply_prepare_observed(client, original, worktree, task, paths, None)?;
        if preview["operation"] != "apply-to-original"
            || !preview["preview_sha256"].as_str().is_some_and(digest)
            || !preview["creation_receipt_sha256"]
                .as_str()
                .is_some_and(digest)
            || preview["tested"] != false
        {
            return Err(());
        }
        let result = original_apply_decide_observed(client, Some(original), Some(&preview))?;
        client.check_running()?;
        show(
            &json!({"workflow":"endpoint-original-transfer", "run_id":run,
            "request_sha256":hash,"paths":paths,"outcome":result["outcome"],
            "preview_sha256":result["preview_sha256"],
            "original_checkout_tested":false,"current_workspace_verified":false,
            "git_commit_created":false,
            "warning":"Only the separately approved original-checkout diff was transferred. The exported terminal record describes the earlier run, including its failures. No test, task-completion, commit or push claim follows."}),
        )
    })
}
