import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";

const schema = JSON.parse(await readFile(new URL("../schemas/model/disabled-gateway-candidate.schema.json", import.meta.url), "utf8"));
const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
const sha = (value) => value.repeat(64);

function candidate(endpointClass = "strict_local") {
  return {
    schema_version: 1, candidate_id: "candidate-1",
    model: { model_profile_id: "model-1", manifest_id: "manifest-1", manifest_sha256: sha("1"), model_revision_id: "revision-1", artifact_sha256: sha("2") },
    runtime_adapter: { adapter_id: "adapter-1", contract_version: 1, runtime_build_id: "runtime-1", runtime_sha256: sha("3") },
    protocol_codec: { protocol_codec_id: "codec-1", protocol_codec_version: "1", protocol_codec_sha256: sha("4") },
    endpoint_profile_id: "endpoint-1", endpoint_profile_sha256: sha("5"), endpoint_class: endpointClass,
    route: { route_id: "route-1", route_version: 1, route_policy_sha256: sha("6") },
    operator: { operator_id: "operator-1", registry_version: 1, operator_sha256: sha("7") },
    credential: endpointClass.startsWith("remote_") ? { credential_reference: "credential://broker/reference-1", broker_version: "1", reference_sha256: sha("8") } : null,
    qualification: { qualification_id: "qualification-1", qualification_version: 1, qualification_sha256: sha("9") },
    enabled: false, automatic_fallback: false, candidate_sha256: sha("a"),
  };
}

test("disabled gateway schema accepts all four complete candidate classes", () => {
  for (const value of ["strict_local", "local_network_private", "remote_private", "remote_managed"]) {
    assert.equal(validate(candidate(value)), true, JSON.stringify(validate.errors));
  }
});

test("disabled gateway schema rejects activation, fallback, missing identity, and widening", () => {
  const mutations = [
    (value) => { value.enabled = true; },
    (value) => { value.automatic_fallback = true; },
    (value) => { delete value.runtime_adapter.runtime_sha256; },
    (value) => { value.protocol_codec.protocol_codec_sha256 = "ambiguous"; },
    (value) => { value.operator.secret = "forbidden"; },
    (value) => { value.endpoint_class = "public_unmanaged"; },
  ];
  for (const mutate of mutations) { const value = candidate(); mutate(value); assert.equal(validate(value), false); }
});
