# English "Golden Rules" for sentence segmentation

- Upstream: https://github.com/nipunsadvilkar/pySBD, `tests/lang/test_english.py`
  (`GOLDEN_EN_RULES_TEST_CASES`), commit `5905f13be4fc95f407b98392e0ec303617a33d86`.
  The rules originate in the Pragmatic Segmenter
  (https://github.com/diasks2/pragmatic_segmenter), also MIT-licensed.
- License: MIT, Copyright (c) 2019 Nipun Sadvilkar (copied to `LICENSE`).
- Contents: 47 short constructed test cases (input text and expected
  sentences). One case wrapped in `pytest.param` in upstream was left out.
- Format: `en.tsv`, one case per line: the text, then each expected sentence,
  separated by tabs; `\n`, `\t` and `\\` are escaped.
