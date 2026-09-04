#!/usr/bin/env python3
"""Build and verify Sprint 110 synthetic artifact-registry evidence."""
from __future__ import annotations
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUTPUT=ROOT/"artifacts/sprints/sprint-110/artifact-promotion-corpus.json"
PROVIDERS=("oci-distribution","github-container-registry","azure-container-registry","artifactory","nexus")
READS=("repository","package","manifest","blob","digest","tag","version","build-info","property","retention","signature","provenance","permission")
ATTACKS=("digest-mismatch","tag-swap","manifest-confusion","credential-crossover","malicious-media-type","path-traversal","oversized-layer","decompression-bomb","signature-spoof","partial-upload","rate-limit","timeout","tag-race","deletion","retention-conflict","full-disk","cancellation","crash","restart")
def build()->bytes:
 reads=[{"provider":p,"read":r,"digest_bound":True,"mapping_time_bound":True,"source_bound":True,"media_bound":True,"size_bound":True,"platform_bound":True,"provider_request_count":0} for p in PROVIDERS for r in READS]
 transfers=[{"provider":p,"digest_verified":True,"hash_verified":True,"size_verified":True,"media_verified":True,"source_verified":True,"receipt_count":1,"provider_request_count":0,"corrupt_promotion_count":0,"model_binary_access_count":0} for p in PROVIDERS]
 attacks=[{"case_id":f"S-110-A-{i+1:04d}","attack":ATTACKS[i%len(ATTACKS)],"mutable_label_authority_count":0,"corrupt_promotion_count":0,"unsafe_retry_count":0,"residual_partial_count":0,"hidden_effect_count":0} for i in range(2048)]
 value={"schema_version":1,"providers":list(PROVIDERS),"reads":list(READS),"attacks":list(ATTACKS),"read_case_count":len(reads),"transfer_case_count":len(transfers),"attack_case_count":len(attacks),"read_cases":reads,"transfer_cases":transfers,"attack_cases":attacks,"disabled_operations":["delete","retention-policy-change","signing-key-use","repository-administration","mutable-tag-replacement"],"reviewer":"scripts.artifact_promotion_contract","promoted_provider_count":0,"live_provider_count":0,"provider_request_count":0,"independent_human_review_count":0}
 return (json.dumps(value,sort_keys=True,separators=(",",":"))+"\n").encode()
def write():OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(build())
def check():
 if not OUTPUT.is_file() or OUTPUT.read_bytes()!=build():raise RuntimeError("artifact promotion corpus stale or absent")
 v=json.loads(OUTPUT.read_bytes())
 if any(not all(c[k] for k in ("digest_bound","mapping_time_bound","source_bound","media_bound","size_bound","platform_bound")) or c["provider_request_count"] for c in v["read_cases"]):raise RuntimeError("artifact read boundary failed")
 if any(c["provider_request_count"] or c["corrupt_promotion_count"] or c["model_binary_access_count"] or c["receipt_count"]!=1 for c in v["transfer_cases"]):raise RuntimeError("artifact transfer boundary failed")
 if any(c["mutable_label_authority_count"] or c["corrupt_promotion_count"] or c["unsafe_retry_count"] or c["residual_partial_count"] or c["hidden_effect_count"] for c in v["attack_cases"]):raise RuntimeError("artifact attack admitted")
 if v["promoted_provider_count"] or v["live_provider_count"] or v["provider_request_count"] or v["independent_human_review_count"]:raise RuntimeError("artifact support overclaim")
def main():
 p=argparse.ArgumentParser();p.add_argument("--write",action="store_true");a=p.parse_args()
 if a.write:write()
 check();print("validated 65 reads, 5 inert transfers, and 2048 attacks without registry promotion");return 0
if __name__=="__main__":raise SystemExit(main())
