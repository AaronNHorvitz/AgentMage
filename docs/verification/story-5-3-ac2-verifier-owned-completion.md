# Story 5.3 AC2 verifier-owned completion

Story acceptance criterion `5.3.AC2` passes for the current deterministic verifier and terminal
result scope. Success requires one opaque proof built from the exact expected output and current
state, ordered terminal observations and receipts, complete current evidence, every required
postcondition, every preserved invariant, and the absence of every prohibited effect.

Exit zero and persuasive model or tool text are not verifier inputs. Missing, stale, reordered,
duplicated, failed, uncertain, or integrity-mismatched evidence cannot construct the proof. Only
changed verified evidence maps to `verified_success`; unchanged verified evidence maps separately
to `verified_no_op`. Blocked, denied, failed, cancelled, timed out, resource exhausted, and
uncertain remain distinct non-success outcomes with no alternate success path.

This closes the current pure verifier and terminal-result criterion with synthetic evidence. It
does not execute a native tool, provider, model, or network effect. Installed-product and
cross-platform evidence, independent review, Story, Sprint, packaging, and release completion
remain separate gates.
