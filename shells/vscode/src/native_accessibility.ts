/** AgentMage-owned observations for one native Chat core workflow. */
export interface NativeAccessibilityObservation {
  readonly namesPresent: boolean;
  readonly focusOrderValid: boolean;
  readonly meaningHasText: boolean;
  readonly updatesOrdered: boolean;
  readonly forcedTimeoutAbsent: boolean;
  readonly generatedStructureValid: boolean;
}

export interface NativeAccessibilityDisposition {
  readonly passed: boolean;
  readonly blockers: readonly string[];
}

/** Fails closed on every missing AgentMage-owned accessibility property. */
export function evaluateNativeAccessibility(
  observation: NativeAccessibilityObservation,
): NativeAccessibilityDisposition {
  const blockers: string[] = [];
  if (!observation.namesPresent) blockers.push("accessibility.name.missing");
  if (!observation.focusOrderValid)
    blockers.push("accessibility.focus.order-invalid");
  if (!observation.meaningHasText)
    blockers.push("accessibility.meaning.color-only");
  if (!observation.updatesOrdered)
    blockers.push("accessibility.live-update.inaccessible");
  if (!observation.forcedTimeoutAbsent)
    blockers.push("accessibility.timeout.forced");
  if (!observation.generatedStructureValid)
    blockers.push("accessibility.structure.malformed");
  return { passed: blockers.length === 0, blockers };
}
