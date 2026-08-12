import assert from "node:assert/strict";
import test from "node:test";

import {
  COMPONENT_ID,
  RegistrationSlot,
  SHELL_AUTHORITY,
} from "../src/index.js";

void test("shell scaffold exposes only declared interface authority", () => {
  assert.equal(COMPONENT_ID, "shell-vscode");
  assert.deepEqual(SHELL_AUTHORITY, [
    "display",
    "interaction",
    "provider-registration",
    "authenticated-ipc-client",
  ]);
});

void test("provider registration replacement and deactivation are exact", () => {
  const slot = new RegistrationSlot();
  let firstDisposals = 0;
  let secondDisposals = 0;
  slot.replace({ dispose: () => (firstDisposals += 1) });
  slot.replace({ dispose: () => (secondDisposals += 1) });
  assert.equal(firstDisposals, 1);
  assert.equal(secondDisposals, 0);
  slot.dispose();
  slot.dispose();
  assert.equal(secondDisposals, 1);
});
