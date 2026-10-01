# Release Notes

This release delivers a myriad of improvements. We have made the the installer faster, and we fixed a bug that caused crashes on startup.

The configuration file is now validated before it is loaded. Errors are reported with a line number. Due to the fact that some users relied on the old behavior, the check can be disabled.

There are three new commands: `sync`, `prune` and `doctor`. The `doctor` command checks your setup and suggests fixes.

The colour of the progress bar can now be changed. The default color is grey.
