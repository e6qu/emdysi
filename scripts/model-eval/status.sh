#!/usr/bin/env bash
# Where a running `real_errors` stands: pairs done, real errors caught so
# far by kind, flags on the corrected text, questions asked, and an
# estimate of the time left.
#
#   scripts/model-eval/status.sh OUT.tsv [ERR] [LOG]
#
# OUT.tsv is the output `real_errors` was given (its journal is read),
# ERR its standard error (for the "N of M pairs" lines), LOG its `LOG`
# file. The estimate is from the pairs done since the journal was created
# (a resumed run's estimate counts the earlier pairs too). With a LOG, how
# the model answers by kind of question; SCREEN (default 3) is the surprise
# a sentence is screened in at. BASE compares with runs without a model
# and with a perfect one (see below).
set -euo pipefail
out=${1:?usage: status.sh OUT.tsv [ERR] [LOG]}
err=${2:-}
log=${3:-}
journal="$out.journal"
[ -f "$journal" ] || { echo "no journal yet: $journal"; exit 0; }
done=$(wc -l < "$journal")
total=""
if [ -n "$err" ] && [ -f "$err" ]; then
  total=$(grep -o 'of [0-9]* pairs' "$err" | tail -1 | awk '{print $2}' || true)
fi
# When the journal was created (0 where the file system does not say).
start=$(stat -c %W "$journal" 2>/dev/null || echo 0)
[[ "$start" =~ ^[0-9]+$ ]] || start=0
mins=0
[ "$start" -gt 0 ] && mins=$(( ($(date +%s) - start) / 60 ))
echo "$(date -u +%H:%M) $done${total:+ of $total} pairs${start:+ in $mins min}"
# The journal's lines: index, kind, caught, flagged after the fix, ...
awk -F'\t' '{
  pairs[$2]++; if ($3 == "true") caught[$2]++; if ($4 == "true") after[$2]++
  all++; if ($3 == "true") c++; if ($4 == "true") a++
} END {
  for (k in pairs) printf "  %-22s %4d of %4d caught, %d flagged after the fix\n", k, caught[k], pairs[k], after[k]
  printf "  %-22s %4d of %4d caught (%.0f%%), %d flagged after the fix\n", "all", c, all, 100 * c / all, a
}' "$journal" | sort
# BASE: a directory with the same pairs checked without a model
# (grammar.tsv) and with a perfect one (oracle.tsv), as `real_errors`
# writes them; the pairs done so far are compared, by kind, text before
# and text after.
if [ -n "${BASE:-}" ] && [ -f "$BASE/grammar.tsv" ]; then
  awk -F'\t' '
    /^#/ { next }
    FILENAME ~ /grammar.tsv$/ { g[$1 FS $4 FS $5] = $2; next }
    FILENAME ~ /oracle.tsv$/ { o[$1 FS $4 FS $5] = $2; next }
    {
      k = $2 FS $5 FS $6; n++
      m = ($3 == "true"); gg = (g[k] == "true"); oo = (o[k] == "true")
      cm += m; cg += gg; co += oo; if (m && !gg) gain++; if (gg && !m) loss++
    }
    END {
      printf "  on these %d pairs: grammar alone %d, the model %d (%d gained, %d lost), a perfect model %d\n", n, cg, cm, gain, loss, co
    }' "$BASE/grammar.tsv" "$BASE/oracle.tsv" "$journal"
fi
# The log's lines: pair, kind of error, side, kind of question, options,
# the perfect answer, the model's, p(right), p(original) (for surprise:
# the highest surprise), ...
if [ -n "$log" ] && [ -f "$log" ]; then
  awk -F'\t' -v screen="${SCREEN:-3}" '!/^#/ {
    q = $4; n[q]++; if ($6 == $7) ok[q]++
    if (q ~ /^surprise/ && $9 >= screen) pass[q]++
  } END {
    for (q in n) {
      if (q == "surprise-wrong")
        printf "  %-15s %5d sentences, most surprising word right %.0f%%, screened in %.0f%%\n", q, n[q], 100 * ok[q] / n[q], 100 * pass[q] / n[q]
      else if (q == "surprise-right")
        printf "  %-15s %5d sentences, screened in %.0f%%\n", q, n[q], 100 * pass[q] / n[q]
      else
        printf "  %-15s %5d asked, %.0f%% chosen right\n", q, n[q], 100 * ok[q] / n[q]
    }
  }' "$log" | sort
fi
if [ -n "$total" ] && [ "$done" -gt 0 ] && [ "$mins" -gt 0 ]; then
  left=$(( (total - done) * mins / done ))
  echo "  about $left min left"
fi
