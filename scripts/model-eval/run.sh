#!/usr/bin/env bash
# The M20 evaluation (docs/plan.md): the small decision model with the
# grammar, against the grammar alone and against a perfect model.
#
#   scripts/model-eval/run.sh SPEC [OUT_DIR]
#
# SPEC names the model as for `en --lm`: gguf:/path/model.gguf (a build with
# `--features llama`), http://127.0.0.1:8080 (llama-server, mlx_lm.server,
# Ollama, ...) or http://HOST:PORT#MODEL. Run from the repository root,
# after `cargo build --release --examples` (and `--features llama` for
# gguf:). The steps run one at a time: two parses at once can exhaust
# memory. Each writes OUT_DIR/<step>.out (default: model-eval-out).
set -euo pipefail
spec=${1:?usage: run.sh SPEC [OUT_DIR]}
out=${2:-model-eval-out}
mkdir -p "$out"
ex=target/release/examples
en=target/release/en
# An in-process model is loaded once per thread.
case "$spec" in gguf:*) threads=1 ;; *) threads=4 ;; esac

echo "1. calibration: fit on one half of the real-error questions, measure on the other"
python3 scripts/model-eval/questions.py 0 > "$out/q0.tsv"
python3 scripts/model-eval/questions.py 1 > "$out/q1.tsv"
"$en" decide --lm "$spec" --eval "$out/q0.tsv" | tee "$out/calibrate.out"
temp=$(grep -o 'fitted temperature [0-9.]*' "$out/calibrate.out" | awk '{print $3}' | head -1)
echo "   fitted temperature: ${temp:-none}"
"$en" decide --lm "$spec" ${temp:+--temperature "$temp"} --eval "$out/q1.tsv" | tee "$out/heldout.out"

echo "2. recall on the 377 real errors: grammar alone, perfect model, the model"
"$ex/real_errors" "$out/re-grammar.tsv" > "$out/re-grammar.out"
ORACLE=1 "$ex/real_errors" "$out/re-oracle.tsv" > "$out/re-oracle.out"
THREADS=$threads EMDYSI_LM="$spec" "$ex/real_errors" "$out/re-model.tsv" > "$out/re-model.out"
tail -n 3 "$out"/re-*.out

echo "3. false flags on edited text with the model (6 and 79 flags without one, 2026-10-13)"
THREADS=$threads EMDYSI_LM="$spec" "$ex/false_flags" "$out/ff1-model.tsv" > "$out/ff1-model.out"
SET=dev2 THREADS=$threads EMDYSI_LM="$spec" "$ex/false_flags" "$out/ff2-model.tsv" > "$out/ff2-model.out"
tail -n 1 "$out"/ff*-model.out
echo "done; to compare with a larger model, run again with its SPEC and another OUT_DIR"
