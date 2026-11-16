// SPDX-License-Identifier: MIT OR Apache-2.0
//! Terminal record, release and separately approved original-checkout transfer.

use super::*;
use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

// Check before any model disclosure, again before saving, and before release.
// Canonical private paths prevent a saved transcript from becoming repository
// input. These are sampled checks; they do not lease directory ancestry.
fn archive_scope(directory: &Path, original: &str, worktree: &str) -> Result<()> {
    if !directory.is_absolute()
        || directory.as_os_str().len() > 4096
        || std::fs::canonicalize(directory)
            .map_err(|_| ())?
            .as_os_str()
            != directory.as_os_str()
        || directory.starts_with(original)
        || directory.starts_with(worktree)
    {
        return Err(());
    }
    crate::protected_directory(directory).map_err(|_| ())?;
    Ok(())
}

pub(super) fn run(
    client: &Client,
    args: &[String],
    archive: Option<&Path>,
    drive: bool,
) -> Result<()> {
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
    if let Some(directory) = archive {
        archive_scope(directory, original, worktree)?;
    }
    workflow::bounded(client, seconds.parse().map_err(|_| ())?, |client| {
        let request = endpoint::checked_selection(client, run, hash, true)?;
        if request.mode != agentmage_kernel_contracts::RuntimeSessionMode::ControlledWrite {
            return Err(());
        }
        let terminal = if drive {
            Some(endpoint::guided_steps(client, &request)?)
        } else {
            None
        };
        // Complete and flush the stdout record or explicitly approved private
        // save receipt before release is offered. Neither restores execution.
        if let Some(directory) = archive {
            archive_scope(directory, original, worktree)?;
            record::save_observed(client, request.clone(), directory)?;
            archive_scope(directory, original, worktree)?;
        } else {
            record::export_observed(client, request.clone())?;
        }
        client.confirm(
            &json!({"request":request,"repository":original,"worktree":worktree,
                "paths":paths,"task":task,
                "record_saved_to_private_directory":archive,
                "observed_terminal":terminal,
                "record_retention":if archive.is_some() {"private-file"} else {"stdout-only-retain-before-release"},
                "warning":"Retain the preceding complete terminal record or saved-file receipt before continuing. Releases this exact run and its process-only events and model ports. Requires a service started for original review; ordinary services discard ownership. Fresh read consent and exact diff approval follow. Release grants no write, test pass, new model run, commit or push. After a lost reply inspect service state and disk; never repeat automatically."}),
            &format!("release {hash}"),
        )?;
        if let Some(directory) = archive {
            archive_scope(directory, original, worktree)?;
        }
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
            "observed_terminal":terminal,"verification":"service-observation-only",
            "preview_sha256":result["preview_sha256"],
            "original_checkout_tested":false,"current_workspace_verified":false,
            "git_commit_created":false,
            "warning":"Only the separately approved original-checkout diff was transferred. The exported terminal record describes the earlier run, including its failures. No test, task-completion, commit or push claim follows."}),
        )
    })
}
