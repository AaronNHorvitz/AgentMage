import assert from "node:assert/strict";
import test from "node:test";

import { evaluateNativeAccessibility } from "../src/native_accessibility.js";

const admitted = {
  namesPresent: true,
  focusOrderValid: true,
  meaningHasText: true,
  updatesOrdered: true,
  forcedTimeoutAbsent: true,
  generatedStructureValid: true,
} as const;

void test("complete AgentMage-owned accessibility observation passes", () => {
  assert.deepEqual(evaluateNativeAccessibility(admitted), {
    passed: true,
    blockers: [],
  });
});

void test("each seeded accessibility defect blocks the core workflow", () => {
  const seeds = [
    ["namesPresent", "accessibility.name.missing"],
    ["focusOrderValid", "accessibility.focus.order-invalid"],
    ["meaningHasText", "accessibility.meaning.color-only"],
    ["updatesOrdered", "accessibility.live-update.inaccessible"],
    ["forcedTimeoutAbsent", "accessibility.timeout.forced"],
    ["generatedStructureValid", "accessibility.structure.malformed"],
  ] as const;
  for (const [field, code] of seeds) {
    const disposition = evaluateNativeAccessibility({
      ...admitted,
      [field]: false,
    });
    assert.equal(disposition.passed, false, field);
    assert.deepEqual(disposition.blockers, [code], field);
  }
});

void test("combined defects remain ordered and none is suppressed", () => {
  const disposition = evaluateNativeAccessibility({
    namesPresent: false,
    focusOrderValid: false,
    meaningHasText: false,
    updatesOrdered: false,
    forcedTimeoutAbsent: false,
    generatedStructureValid: false,
  });
  assert.equal(disposition.passed, false);
  assert.equal(disposition.blockers.length, 6);
  assert.equal(new Set(disposition.blockers).size, 6);
});
