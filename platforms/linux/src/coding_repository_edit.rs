// SPDX-License-Identifier: Apache-2.0
//! Exact edits scoped to this process's successfully created worktree.

use super::*;
use crate::{LinuxCodingPendingWrite, LinuxCodingReplacement};

pub(super) struct PendingEdit {
    scope: LinuxCodingReadScope,
    write: LinuxCodingPendingWrite,
    preview: Value,
    hash: String,
    creation_receipt_sha256: String,
    created_at: u64,
    edits: BTreeMap<String, AppliedEdit>,
}

impl LinuxCodingRepository {
    /// Explicitly selects exact source paths in this owner's held worktree.
    /// Call only after read consent. Caller paths cannot adopt a worktree, and
    /// selecting sources confers neither model disclosure nor write authority.
    pub fn select_created_sources(&self, paths: &[String]) -> Result<LinuxCodingReadScope> {
        let created = self.verify_created()?;
        let scope =
            LinuxCodingReadScope::select(&created.path, paths).map_err(|e| err(e.code()))?;
        self.verify_edit_scope(&scope)?;
        Ok(scope)
    }

    fn verify_edit_scope(&self, scope: &LinuxCodingReadScope) -> Result<&CreatedWorktree> {
        let created = self.verify_created()?;
        if scope.root_sha256() != created.held.root_path_sha256() {
            return Err(err("code.edit.scope"));
        }
        Ok(created)
    }

    /// Prepares the existing kernel's complete inert diff, binding it to held
    /// creation provenance. Only existing selected files can be changed. The
    /// exact outer preview must be approved separately; no write happens here.
    pub fn prepare_edit(
        &mut self,
        scope: LinuxCodingReadScope,
        replacements: &[LinuxCodingReplacement<'_>],
        intent: &str,
        now: u64,
    ) -> Result<Value> {
        let created = self.verify_edit_scope(&scope)?;
        let creation_receipt_sha256 = created.creation_receipt_sha256.clone();
        let mut edits = BTreeMap::new();
        for replacement in replacements {
            let prior = created.edits.get(replacement.path);
            if prior.is_some_and(|prior| prior.after_sha256 != replacement.before_sha256) {
                return Err(err("code.edit.lineage"));
            }
            edits.insert(
                replacement.path.to_owned(),
                AppliedEdit {
                    before_sha256: prior.map_or_else(
                        || replacement.before_sha256.to_owned(),
                        |p| p.before_sha256.clone(),
                    ),
                    after_sha256: hash(replacement.content.as_bytes()),
                },
            );
        }
        if created
            .edits
            .keys()
            .chain(edits.keys())
            .collect::<BTreeSet<_>>()
            .len()
            > 16
        {
            return Err(err("code.edit.lineage-limit"));
        }
        let write = scope
            .prepare_write(replacements, intent, now)
            .map_err(|e| err(e.code()))?;
        self.verify_edit_scope(&scope)?;
        let mut preview = json!({"version":1,"operation":"edit-worktree",
            "worktree_root_sha256":scope.root_sha256(),
            "creation_receipt_sha256":creation_receipt_sha256,
            "write_preview":write.preview(),"created_at_epoch_ms":now,"expires_at_epoch_ms":write.expires_at_epoch_ms(),
            "retention":"process-only","tested":false,"automatic_replay":false,
            "warning":"Writes only the complete displayed diff in the worktree held after this service's creation. No tests, original-repository apply, commit or push. Records are process-only; inspect disk after a lost reply or restart. Dispatch invalidates pending command approvals."});
        let digest = hash(&serde_json::to_vec(&preview).map_err(|_| err("code.edit.preview"))?);
        preview["preview_sha256"] = json!(digest);
        self.edit = Some(PendingEdit {
            scope,
            write,
            preview: preview.clone(),
            hash: digest,
            creation_receipt_sha256,
            created_at: now,
            edits,
        });
        Ok(preview)
    }

    /// Shows the process-only edit preview and last dispatch/result. These are
    /// not canonical durable write receipts and cannot restore ownership.
    pub fn inspect_edit(&self) -> Result<Value> {
        self.authority
            .revalidate_root()
            .map_err(|_| err("code.repository.state"))?;
        Ok(
            json!({"version":1,"pending":self.edit.as_ref().map(|e| &e.preview),
            "latest":self.edit_result,"retention":"process-only","automatic_replay":false,
            "worktree_root_sha256":self.created.as_ref().map(|c| c.held.root_path_sha256()),
            "tested":false}),
        )
    }

    /// Consumes exact trusted consent once, including denial and errors after
    /// matching. The existing native write transaction checks preimages and
    /// reconciles partial failures. No command or validation pass is implied.
    pub fn decide_edit(&mut self, digest: &str, approve: bool, now: u64) -> Result<Value> {
        let pending = self.edit.as_ref().ok_or(err("code.edit.choice-required"))?;
        if pending.hash != digest {
            return Err(err("code.edit.stale"));
        }
        let pending = self.edit.take().ok_or(err("code.edit.choice-required"))?;
        if !approve {
            let result = json!({"outcome":"denied","effect":"none","preview_sha256":digest});
            self.edit_result = Some(result.clone());
            return Ok(result);
        }
        if now < pending.created_at || now >= pending.write.expires_at_epoch_ms() {
            return Err(err("code.edit.expired"));
        }
        let created = self.verify_edit_scope(&pending.scope)?;
        if created.creation_receipt_sha256 != pending.creation_receipt_sha256 {
            return Err(err("code.edit.stale"));
        }
        // Preserve an explicit uncertain state if dispatch unwinds or loses its
        // reply. Never restore the consumed preview or retry automatically.
        self.edit_result =
            Some(json!({"preview_sha256":digest,"outcome":"dispatching","effect":"unknown"}));
        self.command = None;
        self.apply = None;
        // Uncertain writes must never leave an older transferable postimage.
        if let Some(created) = &mut self.created {
            for path in pending.edits.keys() {
                created.edits.remove(path);
            }
        }
        let result = pending.write.apply(
            &pending.scope,
            pending.preview["write_preview"]["change_set_sha256"]
                .as_str()
                .ok_or(err("code.edit.preview"))?,
            pending.preview["write_preview"]["preview_sha256"]
                .as_str()
                .ok_or(err("code.edit.preview"))?,
            now,
        );
        let checked = self
            .verify_edit_scope(&pending.scope)
            .map_err(|_| err("code.edit.uncertain"));
        let value = match (result, checked) {
            (Ok(value), Ok(_)) => json!({"preview_sha256":digest,
                "creation_receipt_sha256":pending.creation_receipt_sha256,
                "worktree_root_sha256":pending.scope.root_sha256(),
                "outcome":value["outcome"],"write_result":value,
                "ownership_scope":"service-created-worktree","tested":false,
                "retention":"process-only","automatic_replay":false}),
            (Err(e), Ok(_)) => {
                self.edit_result =
                    Some(json!({"preview_sha256":digest,"error":e.code(),"effect":"unknown"}));
                return Err(err(e.code()));
            }
            (_, Err(e)) => {
                self.edit_result =
                    Some(json!({"preview_sha256":digest,"error":e.code(),"effect":"unknown"}));
                return Err(e);
            }
        };
        if value["outcome"] == "committed" {
            self.created
                .as_mut()
                .ok_or(err("code.edit.uncertain"))?
                .edits
                .extend(pending.edits);
        }
        self.edit_result = Some(value.clone());
        Ok(value)
    }
}
