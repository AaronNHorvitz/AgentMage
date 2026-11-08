// SPDX-License-Identifier: Apache-2.0
//! Repository setup followed by the original guided workflow, on one clock.

use super::*;
use agentmage_kernel_engine::repository_safety::{RepositoryOperation, RepositoryOperationPlan};

pub(super) fn run(client: &Client, args: &[String], seconds: u64) -> Result<()> {
    let [
        root,
        owned,
        task_id,
        base,
        query,
        model,
        bytes,
        tokens,
        task,
        text,
    ] = args
    else {
        return Err(());
    };
    // Refuse malformed input before inspection consent or any API request.
    for path in [root, owned] {
        let path = Path::new(path);
        if !path.is_absolute()
            || path.as_os_str().len() > 4096
            || path.components().count() > 256
            || path.components().any(|part| {
                !matches!(
                    part,
                    std::path::Component::RootDir | std::path::Component::Normal(_)
                )
            })
        {
            return Err(());
        }
    }
    if Path::new(root).starts_with(owned)
        || Path::new(owned).starts_with(root)
        || task_id.is_empty()
        || task_id.len() > 64
        || !task_id.as_bytes()[0].is_ascii_alphanumeric()
        || !task_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        || ![40, 64].contains(&base.len())
        || !base
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || query.len() > 1024
        || query.chars().any(char::is_control)
        || !(1..=64).contains(
            &query
                .split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|term| !term.is_empty())
                .map(str::to_ascii_lowercase)
                .collect::<BTreeSet<_>>()
                .len(),
        )
        || model.is_empty()
        || model.len() > 256
        || model.chars().any(char::is_control)
        || !bytes
            .parse::<usize>()
            .is_ok_and(|n| (2048..=32768).contains(&n))
        || !tokens.parse::<u32>().is_ok_and(|n| (1..=4096).contains(&n))
        || task.trim().is_empty()
        || task.len() > 4096
    {
        return Err(());
    }
    workflow::definition(text)?;
    workflow::bounded(client, seconds, |client| {
        client.confirm(
            &json!({"repository":root,"management_directory":owned,"task_id":task_id,
                "base_commit":base,"source_query":query,"task":task,
                "maximum_running_seconds":seconds,
                "warning":"Runs hardened local Git inspection and reads the entire repository, including ignored and untracked work, to prepare an isolated worktree. Creation, source discovery, endpoint disclosure, edits and tests need separate approvals. No commit, push, removal or automatic cleanup. A lost reply stops; inspect records and disk before continuing."}),
            &format!("inspect {base}"),
        )?;
        let prepared = client.json(
            "POST",
            "/v1/code/repository/prepare",
            true,
            Some(json!({"root":root,"owned_root":owned,"task_id":task_id,
                "base_commit":base,"approve_inspection":true})),
        )?;
        let plan: RepositoryOperationPlan =
            serde_json::from_value(prepared["plan"].clone()).map_err(|_| ())?;
        plan.verify().map_err(|_| ())?;
        if prepared["operation"] != "create-worktree"
            || plan.operation != RepositoryOperation::WorktreeCreate
            || plan.branch_ref.as_deref() != Some(&format!("refs/heads/agentmage/tasks/{task_id}"))
            || plan.expected_new_object.as_deref() != Some(base.as_str())
            || plan.repository_path_sha256
                != agentmage_platform_linux::linux_repository_path_sha256(root).map_err(|_| ())?
        {
            return Err(());
        }
        let result = worktree_decide_observed(client, Some((root, owned)), Some(&prepared))?;
        let identity = result["worktree_root_sha256"]
            .as_str()
            .filter(|s| digest(s))
            .ok_or(())?;
        let creation = result["receipt"]["receipt_sha256"]
            .as_str()
            .filter(|s| digest(s))
            .ok_or(())?;
        if result["repository_root_sha256"] != prepared["repository_root_sha256"]
            || result["repository_receipt"]["plan_sha256"] != plan.plan_sha256
            || result["repository_receipt"]["outcome"] != "succeeded"
            || result["receipt"]["outcome"] != "succeeded"
        {
            return Err(());
        }
        client.idle()?;
        let worktree = Path::new(owned).join("worktree");
        LinuxCodingReadScope::verify_root(&worktree, identity).map_err(|_| ())?;
        client.confirm(&json!({"worktree":worktree,"worktree_root_sha256":identity,
            "source_query":query,"warning":"Enumerates bounded visible names in this created worktree and ranks paths by lexical query matches. Reads no source text and selects or discloses nothing yet."}),
            &format!("discover {identity}"))?;
        LinuxCodingReadScope::verify_root(&worktree, identity).map_err(|_| ())?;
        let map = client.json(
            "POST",
            "/v1/code/map",
            true,
            Some(json!({"root":worktree,"query":query,"approve_discovery":true})),
        )?;
        if map["root_sha256"] != identity {
            return Err(());
        }
        show(&map)?;
        let mut paths = map["entries"]
            .as_array()
            .ok_or(())?
            .iter()
            .filter(|entry| entry["score"].as_u64().is_some_and(|s| s > 0))
            .take(16)
            .map(|entry| entry["path"].as_str().map(str::to_owned).ok_or(()))
            .collect::<Result<Vec<_>>>()?;
        // Names only before source consent. The original API validates and reads
        // the allowlist after share approval; select() would read files here.
        paths.sort();
        if paths.is_empty() || paths.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(());
        }
        LinuxCodingReadScope::verify_root(&worktree, identity).map_err(|_| ())?;
        let settings = client.json("GET", "/v1/settings", true, None)?;
        let revision = settings["settings_revision"].as_u64().ok_or(())?;
        if settings["endpoint"].as_str().is_none() {
            return Err(());
        }
        let body = json!({"root":worktree,"paths":paths,"approve_read":true,
            "approve_model_context":true,"settings_revision":revision,
            "settings_endpoint":settings["endpoint"],"expected_root_sha256":identity});
        client.confirm(&json!({"selection":body,"settings":settings,
            "ranking":"lexical path match only; review every selected path",
            "warning":"Allows complete selected file text to be read and sent to this configured endpoint. Mapping is not a source or test result. This replaces the service's previous source selection."}),
            &format!("share {revision}"))?;
        LinuxCodingReadScope::verify_root(&worktree, identity).map_err(|_| ())?;
        let accepted = client.json("PUT", "/v1/code/workspace", true, Some(body))?;
        let selected = selection(client)?;
        if accepted["selected"] != true
            || accepted["workspace_selection_id"]
                .as_str()
                .is_none_or(str::is_empty)
            || selected["workspace_selection_id"] != accepted["workspace_selection_id"]
            || selected["workspace_revision"] != accepted["workspace_revision"]
            || selected["paths"] != json!(paths)
            || selected["model_context_approved"] != true
        {
            return Err(());
        }
        let origin = workflow::Origin {
            root: identity.into(),
            creation: json!(creation),
            selected,
            settings,
        };
        let args = [
            worktree.to_str().ok_or(())?.into(),
            model.clone(),
            bytes.clone(),
            tokens.clone(),
            task.clone(),
            text.clone(),
        ];
        workflow::run_steps(client, &args, seconds, Some(&origin))
    })
}
