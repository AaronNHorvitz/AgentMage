import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  LOCAL_HANDOFF_NOTICE,
  parseHandoffReview,
  parseLocalHandoffReceipt,
  parseRenderedHandoff,
} from "../src/handoff.js";

const SHA = "a".repeat(64);

void test("exact review, packet, manifest, and local receipt round trip", () => {
  const review = reviewedPacket();
  assert.equal(parseHandoffReview(review).manifest.delivered, false);
  const packetSha256 = review.manifest.packet_sha256;
  assert.equal(typeof packetSha256, "string");
  const receipt = localReceipt(packetSha256 as string);
  const rendered = {
    schema_version: 2,
    packet_markdown: review.packet_markdown,
    manifest: review.manifest,
    receipt,
  };
  const parsed = parseRenderedHandoff(rendered);
  assert.equal(parsed.packet_markdown, review.packet_markdown);
  assert.equal(parsed.receipt.external_delivery_attempted, false);
});

void test("every review and manifest mutation fails before display", () => {
  const mutations: Array<(value: ReturnType<typeof reviewedPacket>) => void> = [
    (value) => {
      value.packet_markdown += "unreviewed";
    },
    (value) => {
      value.manifest.packet_sha256 = "b".repeat(64);
    },
    (value) => {
      value.manifest.delivered = true;
    },
    (value) => {
      value.manifest.entry_sha256 = [];
    },
    (value) => {
      value.local_only_notice = "contacted";
    },
    (value) => {
      value.confirmation_sha256 = "b".repeat(64);
    },
  ];
  for (const mutate of mutations) {
    const changed = structuredClone(reviewedPacket());
    mutate(changed);
    assert.throws(() => parseHandoffReview(changed));
  }
});

void test("receipt cannot claim delivery, wrong action, or unreviewed packet", () => {
  const receipt = localReceipt(SHA);
  assert.equal(parseLocalHandoffReceipt(receipt).outcome, "rendered");
  for (const mutate of [
    (value: Record<string, unknown>) => {
      value.external_delivery_attempted = true;
    },
    (value: Record<string, unknown>) => {
      value.packet_sha256 = "b".repeat(64);
    },
    (value: Record<string, unknown>) => {
      value.prohibited_action = "network_call";
    },
    (value: Record<string, unknown>) => {
      value.receipt_sha256 = "b".repeat(64);
    },
  ]) {
    const changed = structuredClone(receipt);
    mutate(changed);
    assert.throws(() => parseLocalHandoffReceipt(changed));
  }
});

function reviewedPacket(): {
  schema_version: number;
  preview_id: string;
  packet_markdown: string;
  manifest: Record<string, unknown>;
  local_only_notice: string;
  expires_at_ms: number;
  confirmation_sha256: string;
} {
  const packet = "# Manual Codex Handoff\n\nLocal packet.\n";
  const unsignedManifest: Record<string, unknown> = {
    schema_version: 2,
    handoff_id: "handoff-0001",
    draft_sha256: SHA,
    entry_sha256: [SHA],
    packet_sha256: digestText(packet),
    packet_bytes: Buffer.byteLength(packet, "utf8"),
    destination: "manual_codex_interface",
    acknowledgment_required: true,
    delivered: false,
  };
  const manifest = {
    ...unsignedManifest,
    manifest_sha256: digest(unsignedManifest),
  };
  const unsigned = {
    schema_version: 2,
    preview_id: "preview-0001",
    packet_markdown: packet,
    manifest,
    local_only_notice: LOCAL_HANDOFF_NOTICE,
    expires_at_ms: 10,
  };
  return { ...unsigned, confirmation_sha256: digest(unsigned) };
}

function localReceipt(packetSha256: string): Record<string, unknown> {
  const unsigned = {
    schema_version: 2,
    attempt_id: "attempt-0001",
    handoff_id: "handoff-0001",
    outcome: "rendered",
    result_code: "handoff.local.rendered",
    prohibited_action: null,
    packet_sha256: packetSha256,
    external_delivery_attempted: false,
  };
  return { ...unsigned, receipt_sha256: digest(unsigned) };
}

function digest(value: unknown): string {
  return createHash("sha256")
    .update(JSON.stringify(value), "utf8")
    .digest("hex");
}

function digestText(value: string): string {
  return createHash("sha256").update(value, "utf8").digest("hex");
}
