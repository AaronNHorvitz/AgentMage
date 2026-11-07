// SPDX-License-Identifier: Apache-2.0
//! Separately approved transfer of this owner's exact successful worktree edits.

use super::*;
use crate::coding_workspace::LinuxCodingPendingCreate;
use crate::{LinuxCodingPendingWrite, LinuxCodingReplacement};

enum ApplyWrite {
    Existing(Box<LinuxCodingPendingWrite>),
    Create(Box<LinuxCodingPendingCreate>),
}
impl ApplyWrite {
    fn preview(&self) -> Value {
        match self {
            Self::Existing(w) => json!(w.preview()),
            Self::Create(w) => w.preview(),
        }
    }
    fn expires_at_epoch_ms(&self) -> u64 {
        match self {
            Self::Existing(w) => w.expires_at_epoch_ms(),
            Self::Create(w) => w.expires_at_epoch_ms(),
        }
    }
    fn apply(
        self,
        scope: &LinuxCodingReadScope,
        now: u64,
    ) -> std::result::Result<Value, crate::LinuxCodingWriteError> {
        match self {
            Self::Existing(w) => {
                let change = w.preview().change_set_sha256.clone();
                let preview = w.preview().preview_sha256.clone();
                w.apply(scope, &change, &preview, now)
            }
            Self::Create(w) => w.apply(scope, now),
        }
    }
}

pub(super) struct PendingApply {
    original: LinuxCodingReadScope,
    worktree: LinuxCodingReadScope,
    edits: BTreeMap<String, AppliedEdit>,
    write: ApplyWrite,
    preview: Value,
    hash: String,
    creation_receipt_sha256: String,
    created_at: u64,
}

impl LinuxCodingRepository {
    fn verify_apply_roots(&self) -> Result<&CreatedWorktree> {
        let created = self.verify_created()?;
        created
            .original
            .revalidate()
            .map_err(|_| err("code.apply.scope"))?;
        LinuxCodingReadScope::verify_root(
            &created.original_path,
            created.original.root_path_sha256(),
        )
        .map_err(|_| err("code.apply.scope"))?;
        Ok(created)
    }

    /// Content-free process state. Lost ownership or restart cannot adopt an
    /// original checkout, recover a preview or repeat a consumed write.
    pub fn inspect_apply(&self) -> Result<Value> {
        self.authority
            .revalidate_root()
            .map_err(|_| err("code.repository.state"))?;
        Ok(
            json!({"version":1,"pending":self.apply.as_ref().map(|p| &p.preview),
            "latest":self.apply_result,"retention":"process-only","automatic_replay":false,
            "repository_root_sha256":self.created.as_ref().map(|c| c.original.root_path_sha256()),
            "worktree_root_sha256":self.created.as_ref().map(|c| c.held.root_path_sha256()),
            "available_paths":self.created.as_ref().map(|c| c.edits.keys().collect::<Vec<_>>()),
            "available_changes":self.created.as_ref().map(|c| c.edits.iter().map(|(path, edit)|
                json!({"path":path,"kind":if edit.created_mode.is_some() {"create"} else {"edit"},
                    "creation_mode":edit.created_mode})).collect::<Vec<_>>()),
            "tested":false}),
        )
    }

    /// After explicit read consent, prepares the complete native diff in the
    /// original checkout. Paths must have an uninterrupted chain of successful
    /// edits in this owner's created worktree. Original bytes must still equal
    /// that chain's first preimage; verified creations require an absent original
    /// destination and bind the native creation mode. Mixed existing/created
    /// selections refuse; each preview uses one kernel transaction family.
    /// Worktree bytes must equal the last verified postimage.
    /// Both expected root identities must match the roots displayed for read
    /// consent. A mismatch refuses before inspecting roots or reading sources,
    /// leaving any older pending preview intact. Digests cannot adopt roots.
    /// No caller root, patch, receipt or model answer can adopt either side.
    pub fn prepare_apply(
        &mut self,
        paths: &[String],
        intent: &str,
        expected_original_root_sha256: &str,
        expected_worktree_root_sha256: &str,
        now: u64,
    ) -> Result<Value> {
        let created = self
            .created
            .as_ref()
            .ok_or(err("code.command.worktree-required"))?;
        if expected_original_root_sha256 != created.original.root_path_sha256()
            || expected_worktree_root_sha256 != created.held.root_path_sha256()
        {
            return Err(err("code.apply.read-scope"));
        }
        let created = self.verify_apply_roots()?;
        let edits = paths
            .iter()
            .map(|path| {
                created
                    .edits
                    .get(path)
                    .cloned()
                    .map(|edit| (path.clone(), edit))
                    .ok_or(err("code.apply.lineage"))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let worktree = self.select_created_sources(paths)?;
        let sources = worktree.read(paths).map_err(|e| err(e.code()))?;
        for source in &sources {
            if hash(source.content.as_bytes()) != edits[&source.path].after_sha256 {
                return Err(err("code.apply.source-drift"));
            }
        }
        let creations = edits
            .values()
            .filter(|edit| edit.created_mode.is_some())
            .count();
        // One exact transaction family per preview. Do not split an approved
        // multi-file operation into two separately committing transactions.
        if creations != 0 && creations != edits.len() {
            return Err(err("code.apply.mixed-kinds"));
        }
        let original = if creations == 0 {
            LinuxCodingReadScope::select(&created.original_path, paths)
        } else {
            LinuxCodingReadScope::select_targets(&created.original_path, paths)
        }
        .map_err(|e| err(e.code()))?;
        if original.root_sha256() != created.original.root_path_sha256() {
            return Err(err("code.apply.scope"));
        }
        let write = if creations == 0 {
            let replacements = sources
                .iter()
                .map(|source| LinuxCodingReplacement {
                    path: &source.path,
                    before_sha256: &edits[&source.path].before_sha256,
                    content: &source.content,
                })
                .collect::<Vec<_>>();
            ApplyWrite::Existing(Box::new(
                original
                    .prepare_write(&replacements, intent, now)
                    .map_err(|e| err(e.code()))?,
            ))
        } else {
            let files = sources
                .iter()
                .map(|source| {
                    (
                        source.path.as_str(),
                        source.content.as_str(),
                        edits[&source.path].created_mode.expect("creation-only"),
                    )
                })
                .collect::<Vec<_>>();
            ApplyWrite::Create(Box::new(
                original
                    .prepare_create(&files, intent, now)
                    .map_err(|e| err(e.code()))?,
            ))
        };
        self.verify_apply_roots()?;
        let mut preview = json!({"version":1,"operation":"apply-to-original",
            "repository_root_sha256":original.root_sha256(),"worktree_root_sha256":worktree.root_sha256(),
            "creation_receipt_sha256":created.creation_receipt_sha256,
            "write_preview":write.preview(),"created_at_epoch_ms":now,"expires_at_epoch_ms":write.expires_at_epoch_ms(),
            "retention":"process-only","tested":false,"automatic_replay":false,
            "warning":"Writes only this complete displayed diff in the original checkout. Requires separate exact approval. No index change, commit, push or test claim. Inspect disk after a lost reply; never resend approval."});
        let digest = hash(&serde_json::to_vec(&preview).map_err(|_| err("code.apply.preview"))?);
        preview["preview_sha256"] = json!(digest);
        let creation_receipt_sha256 = created.creation_receipt_sha256.clone();
        self.apply = Some(PendingApply {
            original,
            worktree,
            edits,
            write,
            preview: preview.clone(),
            hash: digest,
            creation_receipt_sha256,
            created_at: now,
        });
        Ok(preview)
    }

    /// Consumes separate exact trusted consent once. Every selected worktree
    /// postimage and original preimage is rechecked before the existing native
    /// transaction. Only committed writes consume transferable edit lineage.
    /// The transaction reconciles partial failures; uncertainty is never replay.
    pub fn decide_apply(&mut self, digest: &str, approve: bool, now: u64) -> Result<Value> {
        if self
            .apply
            .as_ref()
            .ok_or(err("code.apply.choice-required"))?
            .hash
            != digest
        {
            return Err(err("code.apply.stale"));
        }
        let pending = self.apply.take().ok_or(err("code.apply.choice-required"))?;
        if !approve {
            let result = json!({"outcome":"denied","effect":"none","preview_sha256":digest});
            self.apply_result = Some(result.clone());
            return Ok(result);
        }
        if now < pending.created_at || now >= pending.write.expires_at_epoch_ms() {
            return Err(err("code.apply.expired"));
        }
        let created = self.verify_apply_roots()?;
        if created.creation_receipt_sha256 != pending.creation_receipt_sha256
            || created.original.root_path_sha256() != pending.original.root_sha256()
            || created.held.root_path_sha256() != pending.worktree.root_sha256()
            || pending.edits.iter().any(|(path, edit)| {
                created.edits.get(path).is_none_or(|current| {
                    current.before_sha256 != edit.before_sha256
                        || current.after_sha256 != edit.after_sha256
                        || current.created_mode != edit.created_mode
                })
            })
        {
            return Err(err("code.apply.stale"));
        }
        let paths = pending.edits.keys().cloned().collect::<Vec<_>>();
        for source in pending.worktree.read(&paths).map_err(|e| err(e.code()))? {
            if hash(source.content.as_bytes()) != pending.edits[&source.path].after_sha256 {
                return Err(err("code.apply.source-drift"));
            }
        }
        self.apply_result =
            Some(json!({"preview_sha256":digest,"outcome":"dispatching","effect":"unknown"}));
        self.edit = None;
        let result = pending.write.apply(&pending.original, now);
        let checked = self.verify_apply_roots();
        let value = match (result, checked) {
            (Ok(value), Ok(_)) => value,
            (Err(error), _) => {
                self.apply_result =
                    Some(json!({"preview_sha256":digest,"error":error.code(),"effect":"unknown"}));
                return Err(err(error.code()));
            }
            (_, Err(_)) => {
                self.apply_result = Some(
                    json!({"preview_sha256":digest,"error":"code.apply.uncertain","effect":"unknown"}),
                );
                return Err(err("code.apply.uncertain"));
            }
        };
        if value["outcome"] == "committed" {
            for path in paths {
                self.created
                    .as_mut()
                    .ok_or(err("code.apply.uncertain"))?
                    .edits
                    .remove(&path);
            }
        }
        let result = json!({"preview_sha256":digest,"creation_receipt_sha256":pending.creation_receipt_sha256,
            "outcome":value["outcome"],"write_result":value,"ownership_scope":"original-of-service-created-worktree",
            "tested":false,"retention":"process-only","automatic_replay":false});
        self.apply_result = Some(result.clone());
        Ok(result)
    }
}

#[cfg(test)]
#[path = "coding_repository_apply_tests.rs"]
mod tests;
