# S-012-I02 Task Classification Results

**Status:** Pass for deterministic descriptive classification  
**Task:** `12.1.1.2` / legacy `S-012-I02`  
**Scope:** Task intent, task-shape complexity, action risk, and authority denial

## Result

The kernel classifies one validated `WorkPacket` using a closed eight-intent
vocabulary, exact task-shape thresholds, the packet's typed authority class,
and declared reversibility. It reads no task prose to infer authority. The
result is content-free, deterministic, hash-identified, and sealed into the
same always-denied descriptive-authority boundary as plans and actions.

Focused cases closed: **5 of 5**.

| Case | Boundary | Verified result |
|---|---|---|
| `TC-01` | Intent closure | Answer, explain, review, diagnose, plan, change, monitor, and wait classify repeatably and each fails as an authority candidate. |
| `TC-02` | Risk closure | All eight authority classes and both local-write reversibility states map to one of four exact risk classes across every intent. |
| `TC-03` | Complexity | Focused, bounded, and extended thresholds are deterministic and do not alter action risk. |
| `TC-04` | Invalid input | An invalid work packet produces validation findings and no classification. |
| `TC-05` | Independence | Changing intent changes classification identity but never required authority or risk for unchanged typed facts. |

## Risk Mapping

| Typed packet fact | Risk |
|---|---|
| Observe or draft | `Minimal` |
| Reversible local write | `Controlled` |
| Irreversible local write, remote write, or execute | `Elevated` |
| Deploy, secrets, or administration | `Critical` |

Intent labels do not issue, choose, narrow, or widen the packet's authority
class. The classification has no capability, grant, operation, completion, or
execution field or method. `reject_as_authority()` always returns
`authority.descriptive_artifact.denied` for it.

## Limits

- Intent is an explicit descriptive label supplied with the already typed task;
  natural-language intent parsing is not claimed.
- This is not the Decision 0027 semantic-classifier gate. Data sensitivity,
  model capability, advisory classifier restrictions, confidence, disagreement,
  and the 5,000/1,000-case campaigns remain Story 12.2 work.
- No tool, model, worker, persistence, network, platform, package, or release
  workflow is exercised or claimed.
- Manual fuzzing remains deferred and is not represented by these deterministic
  tests.
