/** Display-only projection of the kernel-owned effective autonomy policy. */
export interface AutonomyCenterProjection {
  readonly level:
    | "disabled"
    | "read_only"
    | "draft_only"
    | "confirm_each_write"
    | "scoped_autonomy"
    | "autonomous_within_policy";
  readonly reason: string;
  readonly expiresAt: number;
  readonly remainingBudget: number;
  readonly revision: number;
  readonly narrowerCeilings: readonly string[];
  readonly emergencyDisabled: boolean;
}

/** Rejects malformed or stale projections; this shell never computes or broadens authority. */
export function renderAutonomyCenter(
  value: unknown,
  currentRevision: number,
): AutonomyCenterProjection | undefined {
  if (typeof value !== "object" || value === null) return undefined;
  const v = value as Partial<AutonomyCenterProjection>;
  const levels = [
    "disabled",
    "read_only",
    "draft_only",
    "confirm_each_write",
    "scoped_autonomy",
    "autonomous_within_policy",
  ];
  if (
    !levels.includes(v.level ?? "") ||
    v.revision !== currentRevision ||
    typeof v.reason !== "string" ||
    typeof v.expiresAt !== "number" ||
    typeof v.remainingBudget !== "number" ||
    !Array.isArray(v.narrowerCeilings) ||
    typeof v.emergencyDisabled !== "boolean"
  )
    return undefined;
  return Object.freeze({ ...v }) as AutonomyCenterProjection;
}
