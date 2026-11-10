// SPDX-License-Identifier: Apache-2.0
//! Complete preservation collection with cancellation through every Git read.

use super::*;

pub(super) fn check(cancellation: Option<&CancellationToken>) -> Result<(), LinuxRepositoryError> {
    if cancellation.is_some_and(CancellationToken::is_cancelled) {
        Err(error(LinuxRepositoryErrorKind::ObservationFailed))
    } else {
        Ok(())
    }
}

pub(super) fn check_until(
    cancellation: Option<&CancellationToken>,
    deadline: Instant,
) -> Result<(), LinuxRepositoryError> {
    check(cancellation)?;
    if Instant::now() >= deadline {
        return Err(error(LinuxRepositoryErrorKind::ObservationFailed));
    }
    Ok(())
}

// Private seam: production supplies only the existing confined collector.
// Tests can cancel while a synthetic worker returns without native admission.
struct Observer<'a, F> {
    cancellation: Option<&'a CancellationToken>,
    deadline: Instant,
    execute: F,
}
impl<F> Observer<'_, F>
where
    F: FnMut(
        &[&str],
        Option<&CancellationToken>,
        Instant,
    ) -> Result<Option<Vec<u8>>, LinuxRepositoryError>,
{
    fn check(&self) -> Result<(), LinuxRepositoryError> {
        check_until(self.cancellation, self.deadline)
    }
    fn optional(&mut self, arguments: &[&str]) -> Result<Option<Vec<u8>>, LinuxRepositoryError> {
        self.check()?;
        let output = (self.execute)(arguments, self.cancellation, self.deadline)?;
        self.check()?;
        Ok(output)
    }
    fn required(&mut self, arguments: &[&str]) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.optional(arguments)?
            .ok_or_else(|| error(LinuxRepositoryErrorKind::ObservationFailed))
    }
    fn references(&mut self, reference: &str) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.required(&[
            "for-each-ref",
            "--format=%(refname)%00%(objectname)%00",
            reference,
        ])
    }
}

#[cfg(test)]
fn collect_with(
    scope: &LinuxRepositoryScope,
    cancellation: Option<&CancellationToken>,
    mut execute: impl FnMut(
        &[&str],
        Option<&CancellationToken>,
    ) -> Result<Option<Vec<u8>>, LinuxRepositoryError>,
) -> Result<RepositoryPreservationManifest, LinuxRepositoryError> {
    collect_until(
        scope,
        cancellation,
        Instant::now() + PROCESS_TIMEOUT,
        |args, token, _| execute(args, token),
    )
}

pub(super) fn collect_until(
    scope: &LinuxRepositoryScope,
    cancellation: Option<&CancellationToken>,
    deadline: Instant,
    execute: impl FnMut(
        &[&str],
        Option<&CancellationToken>,
        Instant,
    ) -> Result<Option<Vec<u8>>, LinuxRepositoryError>,
) -> Result<RepositoryPreservationManifest, LinuxRepositoryError> {
    let mut observe = Observer {
        cancellation,
        deadline,
        execute,
    };
    observe.check()?;
    let files = preservation_fs::Files::new_until(scope, cancellation, deadline)?;
    let object_format = text_required(observe.required(&["rev-parse", "--show-object-format"])?)?;
    let head_object = observe
        .optional(&["rev-parse", "--verify", "HEAD"])?
        .map(text_required)
        .transpose()?;
    let symbolic_head = observe
        .optional(&["symbolic-ref", "--quiet", "HEAD"])?
        .map(text_required)
        .transpose()?;
    let detached = symbolic_head.is_none();
    let upstream = if let Some(branch) = &symbolic_head {
        observe
            .optional(&["for-each-ref", "--format=%(upstream)", "--count=1", branch])?
            .map(text_required)
            .transpose()?
            .filter(|value| !value.is_empty())
    } else {
        None
    };

    let index_paths = observe.required(&["ls-files", "--stage", "-z"])?;
    observe.check()?;
    let worktree_state = files.worktree()?;
    observe.check()?;
    let untracked_paths =
        observe.required(&["ls-files", "--others", "--exclude-standard", "-z"])?;
    let ignored_paths = observe.required(&[
        "ls-files",
        "--others",
        "--ignored",
        "--exclude-standard",
        "-z",
    ])?;
    let untracked_count = nul_record_count(&untracked_paths)?;
    let ignored_count = nul_record_count(&ignored_paths)?;
    let path_dispositions = framed_observations(&[
        ("index", &index_paths),
        ("worktree", &worktree_state),
        ("untracked", &untracked_paths),
        ("ignored", &ignored_paths),
    ])?;
    let config = observe.required(&["config", "--local", "--no-includes", "--null", "--list"])?;
    let config_lower = config
        .iter()
        .map(u8::to_ascii_lowercase)
        .collect::<Vec<_>>();

    let local_branches = observe.required(&[
        "for-each-ref",
        "--format=%(refname)%00%(objectname)%00",
        "--exclude=refs/heads/agentmage/**",
        "refs/heads",
    ])?;
    let agent_branches = observe.required(&[
        "for-each-ref",
        "--format=%(refname)%00%(objectname)%00",
        "refs/heads/agentmage",
        "refs/agentmage",
    ])?;
    let remote_refs = observe.references("refs/remotes")?;
    let tags = observe.references("refs/tags")?;
    let notes = observe.references("refs/notes")?;
    let stash = observe.references("refs/stash")?;
    let replacements = observe.references("refs/replace")?;
    let worktrees = observe.required(&["worktree", "list", "--porcelain", "-z"])?;
    let objects = observe.required(&["count-objects", "-v"])?;

    observe.check()?;
    let index_sha256 = files.hash_optional(true, "index")?;
    let gitmodules = files.hash_optional(false, ".gitmodules")?;
    let attributes = files.read_optional(false, ".gitattributes")?;
    let hooks = files.metadata("hooks")?;
    let user_reflogs = files.reflogs("logs", true)?;
    let agent_reflogs = files.reflogs("logs/refs/heads/agentmage", false)?;
    observe.check()?;
    let operations = files.operations()?;
    let alternates = files.exists("objects/info/alternates")?;
    let shallow = files.exists("shallow")?;
    let partial = contains_any(
        &config_lower,
        &[
            b"extensions.partialclone",
            b"promisor=true",
            b"promisor\ntrue",
        ],
    );
    let executable_config = hazardous_config(&config_lower);
    let executable_attributes = attributes.as_deref().is_some_and(hazardous_attributes);
    let hook_present = files.hooks()?;
    let hazardous_configuration = executable_config
        || executable_attributes
        || hook_present
        || alternates
        || !replacements.is_empty();

    observe.check()?;
    let mut agent_state = agent_branches;
    agent_state.extend(agent_reflogs);
    let manifest = RepositoryPreservationManifest::seal(RepositoryPreservationManifest {
        schema_version: 0,
        repository_sha256: scope.repository_path_sha256.clone(),
        common_directory_sha256: files.common_identity()?,
        object_format,
        safe_ownership: true,
        head_object,
        current_branch: symbolic_head,
        upstream,
        detached,
        index_sha256,
        path_dispositions_sha256: hash(&path_dispositions),
        untracked_count,
        ignored_count,
        local_branches_sha256: hash(&local_branches),
        remote_tracking_refs_sha256: hash(&remote_refs),
        tags_sha256: hash(&tags),
        notes_sha256: hash(&notes),
        stash_sha256: hash(&stash),
        replacement_refs_sha256: hash(&replacements),
        reflogs_sha256: hash(&user_reflogs),
        agentmage_refs_sha256: hash(&agent_state),
        worktrees_sha256: hash(&worktrees),
        submodules_sha256: gitmodules,
        lfs_sha256: hash(attributes.as_deref().unwrap_or_default()),
        object_database_sha256: hash(&objects),
        configuration_sha256: hash(&config),
        hooks_sha256: hash(&hooks),
        content_drivers_sha256: hash(attributes.as_deref().unwrap_or_default()),
        remotes_sha256: hash(&config),
        operations_sha256: hash(&operations),
        shallow,
        partial,
        hazardous_configuration,
        manifest_sha256: String::new(),
    })
    .map_err(|_| error(LinuxRepositoryErrorKind::ObservationFailed))?;
    files.revalidate()?;
    observe.check()?;
    Ok(manifest)
}

#[cfg(test)]
#[path = "repository_preservation_tests.rs"]
mod tests;
