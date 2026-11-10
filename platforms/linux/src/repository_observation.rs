// SPDX-License-Identifier: Apache-2.0
//! All collector Git reads share the existing offline inspection worker.

use super::*;
use agentmage_kernel_contracts::{
    AdapterInstanceId, BoundaryKind, CorrelationId, TaskId, WorkspaceAuthorizationId, WorkspaceId,
};
use std::collections::BTreeMap;

struct InventoryRequest {
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    stdout_limit: usize,
    workspace: crate::LinuxAuthorizedWorkspace,
    deadline: Option<Instant>,
}

impl LinuxRepositoryCollector {
    pub(crate) fn observe_confined_inventory_until(
        &self,
        scope: &LinuxRepositoryScope,
        arguments: &[&str],
        maximum_bytes: usize,
        cancellation: Option<&CancellationToken>,
        deadline: Option<Instant>,
    ) -> Result<Option<Vec<u8>>, LinuxRepositoryError> {
        observe_until_with(
            scope,
            arguments,
            maximum_bytes,
            cancellation,
            deadline,
            |request, token| {
                revalidate_git_artifact(&self.git)?;
                let manifest = LinuxRepositoryInspectionManifest::verify(
                    "/usr/bin/systemd-run",
                    "/usr/bin/systemctl",
                    "/usr/bin/bwrap",
                    self.git.launch_path(),
                )?;
                let executor = match &scope.linked_read_only {
                    Some(linked) => LinuxBoundedRepositoryInspectionExecutor::for_linked_worktree(
                        manifest,
                        linked.clone(),
                    )?,
                    None => LinuxBoundedRepositoryInspectionExecutor::new(manifest)?,
                };
                executor
                    .run_read_only(
                        &request.arguments,
                        &request.environment,
                        request.stdout_limit,
                        &request.workspace,
                        token,
                        request.deadline,
                    )
                    .map_err(|_| failed())
            },
        )
    }
}

// Private seam for worker-result and race tests. No product caller can supply
// an executor or substitute a Git observation; both collector entry paths above
// dispatch only through the existing trusted inspection owner.
#[cfg(test)]
fn observe_with(
    scope: &LinuxRepositoryScope,
    arguments: &[&str],
    maximum_bytes: usize,
    cancellation: Option<&CancellationToken>,
    execute: impl FnOnce(
        &InventoryRequest,
        &CancellationToken,
    ) -> Result<RepositoryInspectionPlatformResult, LinuxRepositoryError>,
) -> Result<Option<Vec<u8>>, LinuxRepositoryError> {
    observe_until_with(scope, arguments, maximum_bytes, cancellation, None, execute)
}

fn observe_until_with(
    scope: &LinuxRepositoryScope,
    arguments: &[&str],
    maximum_bytes: usize,
    cancellation: Option<&CancellationToken>,
    deadline: Option<Instant>,
    execute: impl FnOnce(
        &InventoryRequest,
        &CancellationToken,
    ) -> Result<RepositoryInspectionPlatformResult, LinuxRepositoryError>,
) -> Result<Option<Vec<u8>>, LinuxRepositoryError> {
    check_deadline(deadline)?;
    scope.revalidate_linked()?;
    if maximum_bytes == 0
        || maximum_bytes > crate::repository_inventory::MAX_INVENTORY_BYTES
        || cancellation.is_some_and(CancellationToken::is_cancelled)
    {
        return Err(failed());
    }
    let mut prepared_arguments = hardened_observation_prefix()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if scope.linked_read_only.is_none() {
        // Scope selection can name metadata below the checkout other than .git.
        // Preserve that exact selection in the guest namespace; never rediscover
        // an ambient repository or pass a host path to the worker.
        let relative = scope
            .git_directory
            .strip_prefix(&scope.checkout_root)
            .map_err(|_| failed())?;
        if relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(failed());
        }
        let guest_git = Path::new("/work").join(relative);
        let guest_git = guest_git.to_str().ok_or_else(failed)?;
        prepared_arguments.push(format!("--git-dir={guest_git}"));
        prepared_arguments.push("--work-tree=/work".into());
    }
    prepared_arguments.extend(arguments.iter().map(|s| (*s).to_owned()));
    revalidate_scope(scope)?;
    let workspace = crate::authorize_workspace_root(
        &scope.checkout_root,
        WorkspaceId::from_raw("repository-observation"),
        WorkspaceAuthorizationId::from_raw("repository-observation"),
        AdapterInstanceId::from_raw("repository-observation"),
    )
    .map_err(|_| failed())?;
    let request = InventoryRequest {
        arguments: prepared_arguments,
        environment: [
            ("GIT_CONFIG_NOSYSTEM", "1"),
            ("GIT_CONFIG_GLOBAL", "/dev/null"),
            ("GIT_TERMINAL_PROMPT", "0"),
            ("GIT_OPTIONAL_LOCKS", "0"),
            ("GIT_LFS_SKIP_SMUDGE", "1"),
            ("GCM_INTERACTIVE", "Never"),
            ("HOME", "/nonexistent"),
            ("XDG_CONFIG_HOME", "/nonexistent"),
            ("LANG", "C"),
            ("LC_ALL", "C"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect(),
        stdout_limit: linked_inventory_limit(maximum_bytes),
        workspace,
        deadline,
    };
    let fallback = CancellationToken::root(
        BoundaryKind::Shell,
        TaskId::from_raw("repository-observation"),
        CorrelationId::from_raw("repository-observation"),
    );
    let token = cancellation.unwrap_or(&fallback);
    check_deadline(deadline)?;
    let result = execute(&request, token)?;
    check_deadline(deadline)?;
    revalidate_scope(scope)?;
    request.workspace.revalidate().map_err(|_| failed())?;
    if token.is_cancelled()
        || result.termination != RepositoryInspectionTermination::Exited
        || !result.descendants_terminated
        || result.stdout.len() > request.stdout_limit
        || result.stdout_bytes != result.stdout.len() as u64
        || result.stdout_sha256 != hash(&result.stdout)
    {
        return Err(failed());
    }
    match result.exit_code {
        Some(0) => Ok(Some(result.stdout)),
        Some(1 | 128) => Ok(None),
        _ => Err(failed()),
    }
}

fn check_deadline(deadline: Option<Instant>) -> Result<(), LinuxRepositoryError> {
    if deadline.is_some_and(|limit| Instant::now() >= limit) {
        return Err(failed());
    }
    Ok(())
}

fn revalidate_scope(scope: &LinuxRepositoryScope) -> Result<(), LinuxRepositoryError> {
    scope.revalidate_linked()?;
    if verify_owned_directory(&scope.checkout_root)? != scope.checkout_root
        || verify_owned_directory(&scope.git_directory)? != scope.git_directory
        || path_identity(&scope.checkout_root)? != scope.repository_path_sha256
    {
        return Err(failed());
    }
    Ok(())
}

fn failed() -> LinuxRepositoryError {
    error(LinuxRepositoryErrorKind::ObservationFailed)
}

#[cfg(test)]
#[path = "repository_observation_tests.rs"]
mod tests;
