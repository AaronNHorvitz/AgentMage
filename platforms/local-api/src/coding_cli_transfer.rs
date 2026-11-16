// SPDX-License-Identifier: MIT OR Apache-2.0
//! Separately confirmed original-checkout transfer after the guided test cycle.

use super::*;

/// Remains on the setup client's original deadline and interruption latch. No
/// transfer is offered until the original guided workflow observes a full pass.
/// A test receipt is historical: original-checkout contents are not test-verified.
pub(super) fn run(
    client: &Client,
    original: &str,
    worktree: &Path,
    task: &str,
    setup: &Value,
    origin: &workflow::Origin,
) -> Result<()> {
    client.idle()?;
    let root = setup["repository_root_sha256"]
        .as_str()
        .filter(|s| digest(s))
        .ok_or(())?;
    let tree = origin.root.as_str();
    LinuxCodingReadScope::verify_root(Path::new(original), root).map_err(|_| ())?;
    LinuxCodingReadScope::verify_root(worktree, tree).map_err(|_| ())?;
    let status = client.json("GET", "/v1/code/repository/apply", true, None)?;
    if status["repository_root_sha256"] != root
        || status["worktree_root_sha256"] != tree
        || !status["pending"].is_null()
    {
        return Err(());
    }
    let selected = origin.selected["paths"].as_array().ok_or(())?;
    let paths = status["available_paths"]
        .as_array()
        .filter(|paths| (1..=16).contains(&paths.len()))
        .ok_or(())?
        .iter()
        .map(|path| {
            if !selected.contains(path) {
                return Err(());
            }
            path.as_str().map(str::to_owned).ok_or(())
        })
        .collect::<Result<Vec<_>>>()?;
    if paths.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(());
    }
    let prepared = original_apply_prepare_observed(
        client,
        original,
        worktree.to_str().ok_or(())?,
        task,
        &paths,
        Some((root, tree)),
    )?;
    if prepared["operation"] != "apply-to-original"
        || prepared["repository_root_sha256"] != root
        || prepared["worktree_root_sha256"] != tree
        || prepared["creation_receipt_sha256"] != origin.creation
        || !prepared["preview_sha256"].as_str().is_some_and(digest)
    {
        return Err(());
    }
    let result = original_apply_decide_observed(client, Some(original), Some(&prepared))?;
    client.check_running()?;
    show(&json!({"workflow":"guided-original-transfer","paths":paths,
        "repository_root_sha256":root,"worktree_root_sha256":tree,
        "creation_receipt_sha256":origin.creation,"preview_sha256":result["preview_sha256"],
        "outcome":result["outcome"],"original_checkout_tested":false,
        "current_workspace_verified":false,"git_commit_created":false,
        "warning":"Only the separately approved original-checkout diff was transferred. The passing test receipt describes the earlier worktree execution. Inspect current files and records; no commit or push occurred."}))
}
