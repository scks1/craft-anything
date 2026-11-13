# Evidence for the R5.7 live Clean room website build (2026-11-06)

These files back the run recorded in `docs/STATUS.md` by PR #1048 (head 4dd5802a). The run used the published 0.5.0 Linux CLI with Claude Code, and the project folder stayed on the machine that ran it.

| File | What it is |
| --- | --- |
| `check-result.json` | The project's `craft.json` fields `version`, `request`, `target.kind`, `stack`, `status` and `checks`, copied unchanged. The check record holds round 1's three widths, the similarity notes and the four claims. |
| `usage.json` | The project's `usage.json`, byte for byte. |
| `artifact-sha256.txt` | SHA-256 of the capture's and round 1's screenshots, both `capture.json` files and `usage.json`. Each round-1 screenshot has the same hash as the capture screenshot at its width. |
| `builder-session-summary.md` | Counts taken from the builder's kept Claude Code session: which tools it called and whether any call used the site's address or the network. |

Left out:

- The Claude Code session file, because it holds local paths and the full prompts.
- The build activity log, because it holds local paths and the machine's user name.
- `last_reply` and `agent_session` from `craft.json`.

The screenshots and `capture.json` files are also left out unless a maintainer asks for them. The hashes above identify the exact files.

Not shown by these files: what the page's button does. Clean room's state capture doesn't click ordinary buttons, so identical first screenshots say nothing about it. An app, a larger site, another agent, Windows and macOS weren't run either.
