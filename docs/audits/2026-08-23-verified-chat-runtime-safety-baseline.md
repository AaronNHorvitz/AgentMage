# Verified Chat Runtime Safety Baseline

| Field | Recorded value |
|---|---|
| Date | 2026-08-23 |
| Branch | `claude/sprint-41-verification` |
| HEAD | `d59c43ece20d06d57d1fa1de3ad6bab1077b2fd2` |
| Upstream | `origin/claude/sprint-41-verification` |
| Ahead / behind | 1 / 0 |
| Tracked modifications | None |
| Staged files | None |
| Untracked files | Two non-normative instruction inputs listed below |
| Safety ref | `backup/verified-chat-runtime-20260823-verified-chat-runtime` |
| Backup directory | `/var/home/aaronnhorvitz/.local/state/agentmage-safety/20260823-verified-chat-runtime` |

## Preserved Inputs

The following local inputs were archived outside the repository and remain
untracked. They are evidence inputs, not normative AgentMage source:

- `AgentMage_Engineering_Runtime_and_Local_Remote_Model_Gateway_Instructions_for_GPT-5.6_Sol_Ultra.md`
  - SHA-256: `aa4de22d4faa03a181595ff50a7398c22e6a1c5496a34286840d44cc52b6e6b8`
- `AgentMage_Mandatory_Verified_Chat_Full_Runtime_and_Multi_Agent_Implementation_Directive.md`
  - SHA-256: `82f3ca22dfebf1bc4378c564d2e9f1e3d430829ea267906b6772f4889fefc21f`

The archive `untracked-directives.tar.gz` has SHA-256
`94ee553b595070538365bb48ac57624c25135d4fb89d20dab09a4e0c681d813f`.
The tracked working-tree patch is empty and has the canonical empty SHA-256
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

## Planning Artifact Hashes

| Artifact | SHA-256 at baseline |
|---|---|
| `ENGINEERING-RUNTIME.md` | `86d37032e69b29e1e063148cd7a9c8002f6af283301516aaac4a51e84405548d` |
| `MODEL-GATEWAY.md` | `2d53d0b5e0c53b6fb36fa3821758233cee9feb12109448683e1e70ef8d90db0b` |
| `ENGINEERING-CAPABILITY-REGISTRY.md` | `ae43ca752155b09b1995b976805d0295d20953b5533ca3fd78d28baa8e52e4b8` |
| Decision 0043 | `60070042b47516e1e2c377726adf824d9115aecac97127bff20c544280e5e4f9` |
| Decision 0044 | `9789b976748110c8cada9a2ea93ec38c44a9f07f678c730999afeec85bde5db4` |

## Worktree Limitation

Existing CodingMage worktrees were inventoried before implementation. They are
historical user-owned work and are outside this campaign. This implementation
must neither remove nor mutate them. The current AgentMage worktree is the only
working tree authorized for this campaign.
