//! Deterministic multi-resource ceilings with one sticky terminal result.

use std::collections::BTreeMap;

use agentmage_kernel_contracts::AgentStateKind;

/// Closed resource families governed by one deterministic agent run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgentCeilingKind {
    /// Observe-plan-act-review turns.
    Turns,
    /// Input tokens sent to model boundaries.
    InputTokens,
    /// Output tokens admitted from model boundaries.
    OutputTokens,
    /// Total context tokens in one model request.
    ContextTokens,
    /// Repair or dependency retry attempts.
    Retries,
    /// Repeated deterministic denials.
    Denials,
    /// Tool-call proposals admitted for policy evaluation.
    ToolCalls,
    /// Effect attempts admitted for exact authority evaluation.
    Effects,
    /// Monotonic elapsed milliseconds.
    ElapsedMilliseconds,
    /// Maximum observed memory bytes.
    MemoryBytes,
    /// Maximum observed disk bytes.
    DiskBytes,
    /// Maximum observed process count.
    ProcessCount,
    /// Repeated checkpoints with no verified progress.
    NoProgressCycles,
}

/// One positive inclusive limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgentCeiling {
    /// Governed resource family.
    pub kind: AgentCeilingKind,
    /// Positive inclusive maximum.
    pub limit: u64,
}

/// Exact first ceiling breach retained by a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgentCeilingBreach {
    /// Resource whose limit was exceeded.
    pub kind: AgentCeilingKind,
    /// Inclusive configured limit.
    pub limit: u64,
    /// Total the rejected attempt would have produced.
    pub attempted_total: u64,
    /// Exact terminal state selected for the breach.
    pub terminal: AgentStateKind,
}

/// Decision after one resource event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentCeilingDecision {
    /// Resource usage was admitted within the inclusive limit.
    Continue {
        /// Admitted cumulative usage.
        used: u64,
        /// Remaining capacity.
        remaining: u64,
    },
    /// The run has one sticky terminal breach.
    Terminal(AgentCeilingBreach),
}

/// Stable reason a ceiling controller cannot be created or updated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentCeilingError {
    /// The profile omitted one required resource family.
    MissingCeiling(AgentCeilingKind),
    /// A resource family appears more than once.
    DuplicateCeiling(AgentCeilingKind),
    /// A limit must be positive.
    ZeroLimit(AgentCeilingKind),
}

/// Stateful deterministic ceiling controller for one agent run.
pub struct AgentCeilingController {
    limits: BTreeMap<AgentCeilingKind, u64>,
    usage: BTreeMap<AgentCeilingKind, u64>,
    terminal: Option<AgentCeilingBreach>,
}

impl AgentCeilingController {
    /// Creates a controller only from one complete, unique, positive profile.
    pub fn new(ceilings: &[AgentCeiling]) -> Result<Self, AgentCeilingError> {
        let mut limits = BTreeMap::new();
        for ceiling in ceilings {
            if ceiling.limit == 0 {
                return Err(AgentCeilingError::ZeroLimit(ceiling.kind));
            }
            if limits.insert(ceiling.kind, ceiling.limit).is_some() {
                return Err(AgentCeilingError::DuplicateCeiling(ceiling.kind));
            }
        }
        for kind in ALL_AGENT_CEILINGS {
            if !limits.contains_key(&kind) {
                return Err(AgentCeilingError::MissingCeiling(kind));
            }
        }
        Ok(Self {
            usage: limits.keys().map(|kind| (*kind, 0)).collect(),
            limits,
            terminal: None,
        })
    }

    /// Admits one additive usage event or selects the first exact terminal state.
    pub fn consume(&mut self, kind: AgentCeilingKind, amount: u64) -> AgentCeilingDecision {
        if let Some(terminal) = self.terminal {
            return AgentCeilingDecision::Terminal(terminal);
        }
        let limit = self.limits[&kind];
        let used = self.usage[&kind];
        let Some(attempted_total) = used.checked_add(amount) else {
            return self.stop(kind, limit, u64::MAX);
        };
        if attempted_total > limit {
            return self.stop(kind, limit, attempted_total);
        }
        self.usage.insert(kind, attempted_total);
        AgentCeilingDecision::Continue {
            used: attempted_total,
            remaining: limit - attempted_total,
        }
    }

    /// Returns admitted usage for one resource.
    #[must_use]
    pub fn usage(&self, kind: AgentCeilingKind) -> u64 {
        self.usage[&kind]
    }

    /// Returns the first terminal breach, if any.
    #[must_use]
    pub const fn terminal(&self) -> Option<AgentCeilingBreach> {
        self.terminal
    }

    fn stop(
        &mut self,
        kind: AgentCeilingKind,
        limit: u64,
        attempted_total: u64,
    ) -> AgentCeilingDecision {
        let terminal = if kind == AgentCeilingKind::NoProgressCycles {
            AgentStateKind::Stalled
        } else {
            AgentStateKind::Exhausted
        };
        let breach = AgentCeilingBreach {
            kind,
            limit,
            attempted_total,
            terminal,
        };
        self.terminal = Some(breach);
        AgentCeilingDecision::Terminal(breach)
    }
}

/// Every required ceiling in stable order.
pub const ALL_AGENT_CEILINGS: [AgentCeilingKind; 13] = [
    AgentCeilingKind::Turns,
    AgentCeilingKind::InputTokens,
    AgentCeilingKind::OutputTokens,
    AgentCeilingKind::ContextTokens,
    AgentCeilingKind::Retries,
    AgentCeilingKind::Denials,
    AgentCeilingKind::ToolCalls,
    AgentCeilingKind::Effects,
    AgentCeilingKind::ElapsedMilliseconds,
    AgentCeilingKind::MemoryBytes,
    AgentCeilingKind::DiskBytes,
    AgentCeilingKind::ProcessCount,
    AgentCeilingKind::NoProgressCycles,
];

#[cfg(test)]
mod tests {
    use super::{
        ALL_AGENT_CEILINGS, AgentCeiling, AgentCeilingController, AgentCeilingDecision,
        AgentCeilingError, AgentCeilingKind,
    };
    use agentmage_kernel_contracts::AgentStateKind;

    fn profile(limit: u64) -> Vec<AgentCeiling> {
        ALL_AGENT_CEILINGS
            .into_iter()
            .map(|kind| AgentCeiling { kind, limit })
            .collect()
    }

    #[test]
    fn task_12_2_1_3_every_ceiling_is_inclusive_then_terminal() {
        for kind in ALL_AGENT_CEILINGS {
            let mut controller = AgentCeilingController::new(&profile(3)).expect("profile");
            assert_eq!(
                controller.consume(kind, 3),
                AgentCeilingDecision::Continue {
                    used: 3,
                    remaining: 0
                }
            );
            assert!(matches!(
                controller.consume(kind, 1),
                AgentCeilingDecision::Terminal(breach) if breach.kind == kind && breach.attempted_total == 4
            ));
            assert_eq!(controller.usage(kind), 3);
        }
    }

    #[test]
    fn task_12_2_1_3_first_terminal_result_is_sticky() {
        let mut controller = AgentCeilingController::new(&profile(1)).expect("profile");
        let first = controller.consume(AgentCeilingKind::Retries, 2);
        assert!(matches!(first, AgentCeilingDecision::Terminal(_)));
        for kind in ALL_AGENT_CEILINGS {
            assert_eq!(controller.consume(kind, u64::MAX), first);
            assert_eq!(controller.usage(kind), 0);
        }
    }

    #[test]
    fn task_12_2_1_3_no_progress_stalls_and_other_exhaustion_is_exact() {
        for kind in ALL_AGENT_CEILINGS {
            let mut controller = AgentCeilingController::new(&profile(1)).expect("profile");
            let AgentCeilingDecision::Terminal(breach) = controller.consume(kind, 2) else {
                panic!("breach must terminate");
            };
            let expected = if kind == AgentCeilingKind::NoProgressCycles {
                AgentStateKind::Stalled
            } else {
                AgentStateKind::Exhausted
            };
            assert_eq!(breach.terminal, expected);
            assert_eq!(controller.terminal(), Some(breach));
        }
    }

    #[test]
    fn task_12_2_1_3_incomplete_duplicate_or_zero_profiles_fail_closed() {
        let mut missing = profile(1);
        missing.pop();
        assert_eq!(
            AgentCeilingController::new(&missing).err(),
            Some(AgentCeilingError::MissingCeiling(
                AgentCeilingKind::NoProgressCycles
            ))
        );
        let mut duplicate = profile(1);
        duplicate.push(AgentCeiling {
            kind: AgentCeilingKind::Turns,
            limit: 2,
        });
        assert_eq!(
            AgentCeilingController::new(&duplicate).err(),
            Some(AgentCeilingError::DuplicateCeiling(AgentCeilingKind::Turns))
        );
        let mut zero = profile(1);
        zero[0].limit = 0;
        assert_eq!(
            AgentCeilingController::new(&zero).err(),
            Some(AgentCeilingError::ZeroLimit(AgentCeilingKind::Turns))
        );
    }

    #[test]
    fn task_12_2_1_3_counter_overflow_terminates_without_usage_mutation() {
        let mut ceilings = profile(1);
        ceilings[0].limit = u64::MAX;
        let mut controller = AgentCeilingController::new(&ceilings).expect("profile");
        assert!(matches!(
            controller.consume(AgentCeilingKind::Turns, u64::MAX),
            AgentCeilingDecision::Continue { .. }
        ));
        let AgentCeilingDecision::Terminal(breach) = controller.consume(AgentCeilingKind::Turns, 1)
        else {
            panic!("overflow must terminate");
        };
        assert_eq!(breach.attempted_total, u64::MAX);
        assert_eq!(breach.terminal, AgentStateKind::Exhausted);
        assert_eq!(controller.usage(AgentCeilingKind::Turns), u64::MAX);
    }

    #[test]
    fn task_12_2_1_3_usage_is_deterministic_across_event_partitioning() {
        let mut single = AgentCeilingController::new(&profile(10)).expect("profile");
        let mut partitioned = AgentCeilingController::new(&profile(10)).expect("profile");
        single.consume(AgentCeilingKind::InputTokens, 7);
        for amount in [2, 3, 2] {
            partitioned.consume(AgentCeilingKind::InputTokens, amount);
        }
        assert_eq!(single.usage(AgentCeilingKind::InputTokens), 7);
        assert_eq!(partitioned.usage(AgentCeilingKind::InputTokens), 7);
        assert_eq!(single.terminal(), partitioned.terminal());
    }

    #[test]
    fn d027_s12_state_every_ceiling_has_one_sticky_terminal_result() {
        for kind in ALL_AGENT_CEILINGS {
            let mut controller = AgentCeilingController::new(&profile(2)).expect("profile");
            assert_eq!(
                controller.consume(kind, 2),
                AgentCeilingDecision::Continue {
                    used: 2,
                    remaining: 0,
                }
            );
            let terminal = controller.consume(kind, 1);
            let AgentCeilingDecision::Terminal(breach) = terminal else {
                panic!("one unit beyond the inclusive limit must terminate");
            };
            let expected = if kind == AgentCeilingKind::NoProgressCycles {
                AgentStateKind::Stalled
            } else {
                AgentStateKind::Exhausted
            };
            assert_eq!(breach.terminal, expected);
            assert_eq!(controller.usage(kind), 2);
            assert_eq!(controller.consume(kind, u64::MAX), terminal);
        }
    }
}
