# American English word list (ESDB / SCOWL), size 60

- Upstream: https://github.com/en-wl/wordlist (the English Speller Database,
  formerly SCOWL), commit `1e5b7d3a72f47a71da5d28686c1dd4b397178485`
  (2026-06-24).
- Generated with the upstream tools: `make`, then
  `./scowl --db scowl.db word-list 60 A 1` (the default American English
  speller list, 109,143 words including inflected and possessive forms).
  Each word is paired with the smallest ESDB size it appears at (35, 40, 50
  or 60; lower means more common), taken from the same database.
- License: Kevin Atkinson's permissive notice (copied to `Copyright` here).
  Per that file, for a generated word list of size 80 or below that is not
  Australian English "no additional copyright applies and including the
  notice before the === is sufficient". The full file is kept here anyway.
- Use: spelling (known words and suggestion ranking) in `emdysi-check`.
