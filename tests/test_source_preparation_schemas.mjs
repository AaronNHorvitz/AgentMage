import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sha = "a".repeat(64);
const ajv = new Ajv2020({ allErrors: true, strict: true });
const sourceValidator = ajv.compile(JSON.parse(fs.readFileSync(
  path.join(ROOT, "schemas/runtime/prepared-source-manifest.schema.json"), "utf8",
)));
const contextValidator = ajv.compile(JSON.parse(fs.readFileSync(
  path.join(ROOT, "schemas/runtime/prepared-source-context-manifest.schema.json"), "utf8",
)));

function source() {
  return {
    schema_version: 2, source_id: "source-1", request_id: "request-1", source_kind: "paste",
    protected_origin_sha256: sha, media_type: "text/plain", media_family: "plain_text",
    encoding: "utf8", sensitivity: "internal", lifecycle: "current", retention: { kind: "ephemeral" },
    limits: { max_input_bytes: 26214400, max_output_bytes: 8388608, max_lines: 250000, max_sections: 8192, max_index_terms: 250000 },
    source_bytes: 12, source_sha256: sha, extraction_sha256: sha, extraction_state: "complete",
    total_lines: 1, retained_lines: 1, omitted_lines: 0, token_counter_id: "counter-1",
    token_counter_sha256: sha, tokenizer_sha256: sha, revision: 1, collected_at_epoch_ms: 1,
    manifest_sha256: sha,
  };
}

function context() {
  const item = {
    item_id: "source-context-1", kind: "evidence", sensitivity: "internal", admission: "eligible",
    authoritative_evidence: true, essential: false, source_id: "source-1", source_revision: sha,
    content_sha256: sha, bounded_excerpt: "bounded evidence", token_count: 2,
  };
  return {
    schema_version: 2, context_manifest_id: "context-1", context_window_plan_sha256: sha,
    token_counter_id: "counter-1", token_counter_sha256: sha, tokenizer_sha256: sha,
    allocated_tokens: 10, used_tokens: 2,
    records: [
      { source_id: "source-1", source_revision_sha256: sha, section_id: null, section_sha256: null, disposition: "included", reason_code: null, token_count: 0 },
      { source_id: "source-1", source_revision_sha256: sha, section_id: "section-1", section_sha256: sha, disposition: "included", reason_code: null, token_count: 2 },
    ],
    packet: {
      schema_version: 2, context_packet_id: "packet-1", max_bytes: 1024, max_tokens: 10,
      token_counter_id: "counter-1", items: [item], accounting: [{
        item_id: item.item_id, kind: item.kind, sensitivity: item.sensitivity, source_id: item.source_id,
        source_revision: item.source_revision, content_sha256: item.content_sha256,
        byte_count: 16, token_count: 2, included: true, omission: null,
      }], used_bytes: 16, used_tokens: 2, packet_sha256: sha,
    },
    manifest_sha256: sha,
  };
}

test("prepared source and context schemas accept closed canonical records", () => {
  assert.equal(sourceValidator(source()), true, JSON.stringify(sourceValidator.errors));
  assert.equal(contextValidator(context()), true, JSON.stringify(contextValidator.errors));
});

test("prepared source schemas reject paths, unknown fields, widening, and missing accounting", () => {
  const mutations = [
    [sourceValidator, { ...source(), path: "/private/source" }],
    [sourceValidator, { ...source(), lifecycle: "reattached_without_verification" }],
    [sourceValidator, { ...source(), source_bytes: 26214401 }],
    [contextValidator, { ...context(), prompt: "hidden prompt" }],
    [contextValidator, { ...context(), records: [] }],
    [contextValidator, { ...context(), used_tokens: -1 }],
  ];
  for (const [validator, value] of mutations) {
    assert.equal(validator(value), false);
  }
});
