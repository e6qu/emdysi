#!/usr/bin/env bash
# The M20 evaluation (docs/plan.md) on a sample: the decision model with
# the grammar, against the grammar alone and a perfect model.
#
#   scripts/model-eval/run.sh SPEC [OUT_DIR] [SHARD]
#
# SPEC names the model as for `en --lm`: gguf:/path/model.gguf (a build
# with `--features llama`), http://127.0.0.1:8080 (llama-server, ...) or
# http://HOST:PORT#MODEL. Run from the repository root after
# `cargo build --release --features llama --examples --bins`. SHARD
# (default 0/4) samples the real-error pairs and the documents of the
# first development set; "" runs everything. Each step writes
# OUT_DIR/<step>.out (default: model-eval-out); the real-error and
# false-flag runs journal their progress and resume when run again. Watch
# the model run with
#   BASE=OUT_DIR scripts/model-eval/status.sh OUT_DIR/re-model.tsv \
#     OUT_DIR/re-model.err OUT_DIR/re-model.log
set -euo pipefail
spec=${1:?usage: run.sh SPEC [OUT_DIR] [SHARD]}
out=${2:-model-eval-out}
shard=${3-0/4}
mkdir -p "$out"
ex=target/release/examples
en=target/release/en
# Two pairs at once, each model on two of four cores: one pair parses
# while the other's model runs. Two runs with models at once are far
# slower than one after the other.
export THREADS=${THREADS:-2} EMDYSI_LM_THREADS=${EMDYSI_LM_THREADS:-2}

echo "1. the real-error questions as minimal pairs, by likelihood"
python3 scripts/model-eval/questions.py > "$out/questions.tsv"
"$en" decide --lm "$spec" --likelihood --eval "$out/questions.tsv" | tee "$out/decide.out"

echo "2. recall on the real errors (SHARD=$shard): grammar alone, perfect model, the model"
[ -f "$out/grammar.tsv" ] || SHARD=$shard "$ex/real_errors" "$out/grammar.tsv" > "$out/grammar.out" 2>/dev/null
[ -f "$out/oracle.tsv" ] || ORACLE=1 SHARD=$shard "$ex/real_errors" "$out/oracle.tsv" > "$out/oracle.out" 2>/dev/null
SHARD=$shard LOG="$out/re-model.log" EMDYSI_LM="$spec" "$ex/real_errors" "$out/re-model.tsv" \
  > "$out/re-model.out" 2> >(grep --line-buffered ' pairs$' > "$out/re-model.err")
tail -n 3 "$out"/grammar.out "$out"/oracle.out "$out"/re-model.out

echo "3. the versions questions again, by way of choosing and margin"
"$ex/versions" "$spec" "$out/re-model.log.versions.tsv" "$out/scores.tsv" > "$out/versions.out" 2>/dev/null
python3 scripts/model-eval/margins.py "$out/scores.tsv" 2 | tee "$out/margins.out"

echo "4. false flags on edited text (first development set, SHARD=$shard), without and with the model"
[ -f "$out/ff-grammar.tsv" ] || SHARD=$shard "$ex/false_flags" "$out/ff-grammar.tsv" > "$out/ff-grammar.out" 2>/dev/null
SHARD=$shard EMDYSI_LM="$spec" "$ex/false_flags" "$out/ff-model.tsv" > "$out/ff-model.out" 2>/dev/null
tail -n 1 "$out"/ff-grammar.out "$out"/ff-model.out
echo "done; to compare with another model, run again with its SPEC and another OUT_DIR"
