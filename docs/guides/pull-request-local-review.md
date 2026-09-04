# Pull-Request Local Review Guide

Before review, require an AgentMage-owned isolated worktree and bind exact base, head, merge base,
fork, changed paths, dependencies, instructions, tests, and current hosted state. After analysis,
remap every finding against the refreshed head; never display a non-current result as current.

Review shadow fixes only as controlled-write proposals with complete diff, tests, risks, and
rollback. Reject any applied, committed, pushed, published, or hosted-effect state. Local contract
fixtures do not substitute for credentialed fetch, native sandbox execution, or hosted refresh.
