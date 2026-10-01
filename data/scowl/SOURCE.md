# English word list (ESDB / SCOWL), size 60, American and British spellings

- Upstream: https://github.com/en-wl/wordlist (the English Speller Database,
  formerly SCOWL), commit `1e5b7d3a72f47a71da5d28686c1dd4b397178485`
  (2026-06-24).
- Generated with the upstream tools: `make`, then
  `./scowl --db scowl.db word-list 60 A,B,Z 1`: American (`A`), British
  "-ise" (`B`) and British "-ize"/Oxford (`Z`) spellings, 112,031 words
  including inflected and possessive forms. Australian spellings (`D`) are
  left out because they carry an additional notice.
  Each word is paired with the smallest ESDB size it appears at (35, 40, 50
  or 60; lower means more common), taken from the same database.
- License: Kevin Atkinson's permissive notice (copied to `Copyright` here).
  Per that file, for a generated word list of size 80 or below that is not
  Australian English "no additional copyright applies and including the
  notice before the === is sufficient". The full file is kept here anyway.
- Use: spelling (known words and suggestion ranking) in `emdysi-check`.
- `variants.tsv`: American and British spelling pairs (and British -ise
  and -ize pairs), generated from the same database by
  [`gen-variants.py`](gen-variants.py):
  `python3 gen-variants.py scowl.db en-60.tsv > variants.tsv`. Only words
  in `en-60.tsv` are kept. Use: the `consistency` rule kind.
