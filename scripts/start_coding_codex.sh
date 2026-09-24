#!/usr/bin/env bash
set -euo pipefail

repo="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
brief="$repo/docs/guides/codex-coding-implementation-brief.md"
state="$HOME/.local/state/agentmage-codex-coding"
old_stop="$HOME/.local/share/agentmage-run/STOP-CLAUDE"

command -v codex >/dev/null
command -v flock >/dev/null
test -r "$brief"
cd -- "$repo"

branch="$(git symbolic-ref --quiet --short HEAD)"
if [[ "$branch" != "main" ]]; then
    printf 'Refusing unexpected branch: %s\n' "$branch" >&2
    exit 1
fi
if command -v tmux >/dev/null && tmux has-session -t '=agentmage-claude' 2>/dev/null; then
    printf 'The old agentmage-claude session exists; inspect it before restarting.\n' >&2
    exit 1
fi

umask 077
mkdir -p -- "$state"
exec 9>"$state/worker.lock"
if ! flock -n 9; then
    printf 'Another coding worker launched by this script still holds the lock.\n' >&2
    exit 1
fi
if [[ -e "$state/STOP-CODEX" || -L "$state/STOP-CODEX" ]]; then
    printf 'Stop marker remains active: %s/STOP-CODEX\n' "$state" >&2
    exit 1
fi

# Invoking this launcher is the owner's explicit restart, not model preparation.
if [[ -e "$old_stop" || -L "$old_stop" ]]; then
    archive="$old_stop.before-codex.$(date -u +%Y%m%dT%H%M%S).$$"
    mv -- "$old_stop" "$archive"
    printf 'Archived prior stop marker: %s\n' "$archive"
fi

printf 'Starting AgentMage coding work on %s with full access and no approvals.\n' "$branch"
# Keep this shell alive so the writer lock survives child descriptor cleanup.
codex --cd "$repo" \
    --model gpt-5.6-sol \
    --config 'model_reasoning_effort="high"' \
    --ask-for-approval never \
    --sandbox danger-full-access \
    --search --no-alt-screen \
    "$(<"$brief")"
