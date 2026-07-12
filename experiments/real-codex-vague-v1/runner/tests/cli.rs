use selta_codex_eval::{
    sha256_hex, AttemptRole, Evidence, EvidenceState, Outcome, Prediction, RawArtifacts,
};
use serde_json::{json, Value};
use std::fs;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
#[test]
fn dry_run_and_validation_require_no_model_call() {
    let directory = tempfile::tempdir().unwrap();
    let inputs = directory.path().join("inputs.jsonl");
    let prompt = directory.path().join("p0.txt");
    let schema = directory.path().join("response.schema.json");
    let selta_schema = directory.path().join("response.selta.json");
    let output = directory.path().join("run");
    fs::write(
        &inputs,
        serde_json::to_string(&json!({
            "id": "case-1",
            "domain_family": "fixture",
            "clarity": "clear",
            "claim": "Alpha occurred.",
            "text": "Alpha occurred."
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();
    fs::write(&prompt, "Assess the supplied claim and text.").unwrap();
    fs::write(
        &schema,
        serde_json::to_vec(&json!({
            "type": "object",
            "properties": {
                "support": {
                    "type": "array", "maxItems": 3, "uniqueItems": true,
                    "items": {"type": "string", "minLength": 1, "maxLength": 160}
                },
                "refute": {
                    "type": "array", "maxItems": 3, "uniqueItems": true,
                    "items": {"type": "string", "minLength": 1, "maxLength": 160}
                }
            },
            "required": ["support", "refute"],
            "additionalProperties": false
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &selta_schema,
        serde_json::to_vec(&json!({
            "type": "object",
            "fields": {
                "support": {
                    "type": "array", "required": true, "len": {"max": 3},
                    "item": {"type": "str", "verify": [{
                        "ext": "len", "config": {"min": 1, "max": 160}
                    }]}
                },
                "refute": {
                    "type": "array", "required": true, "len": {"max": 3},
                    "item": {"type": "str", "verify": [{
                        "ext": "len", "config": {"min": 1, "max": 160}
                    }]}
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_selta-codex-eval"))
        .arg("run")
        .arg("--mode")
        .arg("engineering-smoke")
        .arg("--inputs")
        .arg(&inputs)
        .arg("--prompt")
        .arg(format!("p0={}", prompt.display()))
        .arg("--schema")
        .arg(&schema)
        .arg("--selta-schema")
        .arg(&selta_schema)
        .arg("--output")
        .arg(&output)
        .arg("--model")
        .arg("fixture-model")
        .arg("--reasoning")
        .arg("low")
        .arg("--seed")
        .arg("7")
        .arg("--concurrency")
        .arg("4")
        .arg("--codex")
        .arg(env!("CARGO_BIN_EXE_selta-codex-eval"))
        .arg("--dry-run")
        .status()
        .unwrap();
    assert!(status.success());
    assert!(output.join("manifest.json").is_file());
    assert!(output.join("jobs.jsonl").is_file());
    assert!(!output.join("predictions.jsonl").exists());
    let completion: Value =
        serde_json::from_slice(&fs::read(output.join("completion.json")).unwrap()).unwrap();
    assert_eq!(completion["status"], "dry_run_validated");

    let predictions = directory.path().join("predictions.jsonl");
    let prediction = Prediction {
        attempt_id: "a0001".into(),
        attempt_role: AttemptRole::ScoredFirstAttempt,
        job_index: 0,
        case_id: "case-1".into(),
        prompt_id: "p0".into(),
        prompt_sha256: "a".repeat(64),
        request_sha256: "b".repeat(64),
        outcome: Outcome::Admitted {
            state: EvidenceState::SupportOnly,
            response: Evidence {
                support: vec!["Alpha occurred.".into()],
                refute: vec![],
            },
        },
        response_contract_valid: true,
        latency_ms: 5,
        usage: None,
        raw: RawArtifacts::default(),
    };
    fs::write(
        &predictions,
        serde_json::to_string(&prediction).unwrap() + "\n",
    )
    .unwrap();
    let validation = Command::new(env!("CARGO_BIN_EXE_selta-codex-eval"))
        .arg("validate")
        .arg("--inputs")
        .arg(&inputs)
        .arg("--predictions")
        .arg(&predictions)
        .arg("--selta-schema")
        .arg(&selta_schema)
        .output()
        .unwrap();
    assert!(validation.status.success());
    let report: Value = serde_json::from_slice(&validation.stdout).unwrap();
    assert_eq!(report["admitted"], 1);
    assert!(report.get("macro_f1").is_none());

    let source_home = directory.path().join("source-codex-home");
    fs::create_dir(&source_home).unwrap();
    fs::write(source_home.join("auth.json"), "{}\n").unwrap();
    fs::write(source_home.join("AGENTS.md"), "must not be copied\n").unwrap();
    let fake_codex = directory.path().join("fake-codex.sh");
    fs::write(
        &fake_codex,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "codex-cli fake"
  exit 0
fi
count=0
for entry in "$CODEX_HOME"/*; do
  count=$((count + 1))
  [ "$(basename "$entry")" = "auth.json" ] || exit 41
done
[ "$count" -eq 1 ] || exit 41
if env | grep -E '^(CODEX_|OPENAI_|CHATGPT_)' | grep -v '^CODEX_HOME=' >/dev/null; then exit 42; fi
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output-last-message" ]; then shift; out="$1"; fi
  shift
done
cat >/dev/null
printf '%s' '{"support":[],"refute":[]}' > "$out"
printf '%s\n' '{"type":"thread.started"}' '{"type":"turn.started"}' '{"type":"item.completed","item":{"type":"agent_message"}}' '{"type":"turn.completed","usage":{"input_tokens":12,"cached_input_tokens":4,"output_tokens":3}}'
"#,
    )
    .unwrap();
    fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o700)).unwrap();
    let completed_run = directory.path().join("completed-run");
    let completed = Command::new(env!("CARGO_BIN_EXE_selta-codex-eval"))
        .arg("run")
        .arg("--mode")
        .arg("engineering-smoke")
        .arg("--inputs")
        .arg(&inputs)
        .arg("--prompt")
        .arg(format!("p0={}", prompt.display()))
        .arg("--schema")
        .arg(&schema)
        .arg("--selta-schema")
        .arg(&selta_schema)
        .arg("--output")
        .arg(&completed_run)
        .arg("--model")
        .arg("fixture-model")
        .arg("--reasoning")
        .arg("low")
        .arg("--seed")
        .arg("7")
        .arg("--codex")
        .arg(&fake_codex)
        .env("CODEX_HOME", &source_home)
        .env("CODEX_THREAD_ID", "must-be-cleared")
        .env("OPENAI_API_KEY", "must-be-cleared")
        .status()
        .unwrap();
    assert!(completed.success());
    let bound = Command::new(env!("CARGO_BIN_EXE_selta-codex-eval"))
        .arg("validate")
        .arg("--run")
        .arg(&completed_run)
        .output()
        .unwrap();
    assert!(
        bound.status.success(),
        "{}",
        String::from_utf8_lossy(&bound.stderr)
    );

    let predictions_path = completed_run.join("predictions.jsonl");
    let prediction_bytes = fs::read(&predictions_path).unwrap();
    fs::set_permissions(&predictions_path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(
        &predictions_path,
        [prediction_bytes.as_slice(), b"\n"].concat(),
    )
    .unwrap();
    let tampered = Command::new(env!("CARGO_BIN_EXE_selta-codex-eval"))
        .arg("validate")
        .arg("--run")
        .arg(&completed_run)
        .status()
        .unwrap();
    assert!(!tampered.success());
    fs::write(&predictions_path, &prediction_bytes).unwrap();

    let events_path = completed_run.join("raw/a0001.events.jsonl");
    let event_bytes = fs::read(&events_path).unwrap();
    fs::write(&events_path, [event_bytes.as_slice(), b"{}\n"].concat()).unwrap();
    let raw_tampered = Command::new(env!("CARGO_BIN_EXE_selta-codex-eval"))
        .arg("validate")
        .arg("--run")
        .arg(&completed_run)
        .status()
        .unwrap();
    assert!(!raw_tampered.success());

    let orphan_marker = directory.path().join("orphan-marker");
    let timeout_codex = directory.path().join("timeout-codex.sh");
    fs::write(
        &timeout_codex,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "codex-cli timeout-fixture"; exit 0; fi
cat >/dev/null
(sleep 2; printf orphan > "$MARKER_PATH") &
sleep 10
"#,
    )
    .unwrap();
    fs::set_permissions(&timeout_codex, fs::Permissions::from_mode(0o700)).unwrap();
    let timeout_run = directory.path().join("timeout-run");
    let timed = Command::new(env!("CARGO_BIN_EXE_selta-codex-eval"))
        .arg("run")
        .arg("--mode")
        .arg("engineering-smoke")
        .arg("--inputs")
        .arg(&inputs)
        .arg("--prompt")
        .arg(format!("p0={}", prompt.display()))
        .arg("--schema")
        .arg(&schema)
        .arg("--selta-schema")
        .arg(&selta_schema)
        .arg("--output")
        .arg(&timeout_run)
        .arg("--model")
        .arg("fixture-model")
        .arg("--reasoning")
        .arg("low")
        .arg("--seed")
        .arg("8")
        .arg("--timeout-seconds")
        .arg("1")
        .arg("--codex")
        .arg(&timeout_codex)
        .env("CODEX_HOME", &source_home)
        .env("MARKER_PATH", &orphan_marker)
        .status()
        .unwrap();
    assert!(timed.success());
    std::thread::sleep(std::time::Duration::from_secs(3));
    assert!(!orphan_marker.exists(), "timed-out grandchild survived");

    let oracle = directory.path().join("oracle.jsonl");
    let nonce = directory.path().join("nonce.txt");
    let commitment = directory.path().join("commitment.json");
    let oracle_bytes = b"{\"id\":\"case-1\",\"state\":\"support_only\"}\n";
    let nonce_bytes = [7_u8; 32];
    let nonce_hex = nonce_bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let input_bytes = fs::read(&inputs).unwrap();
    let mut committed = nonce_bytes.to_vec();
    committed.push(0);
    committed.extend_from_slice(oracle_bytes);
    fs::write(&oracle, oracle_bytes).unwrap();
    fs::write(&nonce, nonce_hex).unwrap();
    fs::write(
        &commitment,
        serde_json::to_vec(&json!({
            "version": 1,
            "construction": "sha256(hex_decode(nonce) || 0x00 || exact_oracle_bytes)",
            "nonce": {"decoded_byte_length": 32},
            "oracle": {
                "record_count": 1,
                "byte_length": oracle_bytes.len(),
                "commitment_sha256": sha256_hex(&committed)
            },
            "inputs": {
                "record_count": 1,
                "byte_length": input_bytes.len(),
                "sha256": sha256_hex(&input_bytes)
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let verified = Command::new(env!("CARGO_BIN_EXE_selta-codex-eval"))
        .arg("verify-commitment")
        .arg("--commitment")
        .arg(&commitment)
        .arg("--inputs")
        .arg(&inputs)
        .arg("--oracle")
        .arg(&oracle)
        .arg("--nonce")
        .arg(&nonce)
        .output()
        .unwrap();
    assert!(verified.status.success());
    let report: Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(report["valid"], true);

    let rejected_commitment = Command::new(env!("CARGO_BIN_EXE_selta-codex-eval"))
        .arg("run")
        .arg("--mode")
        .arg("engineering-smoke")
        .arg("--inputs")
        .arg(&inputs)
        .arg("--prompt")
        .arg(format!("p0={}", prompt.display()))
        .arg("--schema")
        .arg(&schema)
        .arg("--selta-schema")
        .arg(&selta_schema)
        .arg("--output")
        .arg(directory.path().join("invalid-commitment-mode"))
        .arg("--model")
        .arg("fixture-model")
        .arg("--reasoning")
        .arg("low")
        .arg("--seed")
        .arg("9")
        .arg("--codex")
        .arg(&fake_codex)
        .arg("--commitment")
        .arg(&commitment)
        .arg("--dry-run")
        .status()
        .unwrap();
    assert!(!rejected_commitment.success());
}
