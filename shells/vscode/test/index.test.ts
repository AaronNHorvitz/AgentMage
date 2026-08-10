import assert from "node:assert/strict";
import test from "node:test";

import { COMPONENT_ID, SHELL_AUTHORITY } from "../src/index.js";

void test("shell scaffold exposes only declared interface authority", () => {
  assert.equal(COMPONENT_ID, "shell-vscode");
  assert.deepEqual(SHELL_AUTHORITY, [
    "display",
    "interaction",
    "provider-registration",
    "authenticated-ipc-client",
  ]);
});
