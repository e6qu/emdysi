Great work on this PR! 🎉 Overall, the changes look solid and the code is clean and easy to follow. I really appreciate the thorough tests and the clear description.

I have a few suggestions that might help make this even better:

1. **Error handling:** In `fetch_user`, it might be worth considering what happens if the request times out. Adding a retry with backoff could help improve resilience.

2. **Naming:** The variable `data` is a bit generic. Consider renaming it to something more descriptive, like `user_profile`, to improve readability.

3. **Performance:** The loop in `merge_records` iterates over the list twice. It may be possible to combine these into a single pass for better efficiency.

4. **Documentation:** It would be great to add a short doc comment explaining the purpose of the new `sync_mode` flag, as it may not be immediately obvious to future readers.

None of these are blockers — just ideas to consider. Feel free to push back if you disagree with any of them!

Thanks again for the great contribution. Looking forward to seeing this merged! 🚀
