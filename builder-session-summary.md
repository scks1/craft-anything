# Builder session summary (R5.7 website run, 2026-11-06)

Source: the session file Craft Anything kept for the run's builder (`agent-session.jsonl`, 269 lines, 1,586,077 bytes). The file itself isn't published. The counts below come from its `assistant` records, with each tool call counted once by id.

| Tool | Calls |
| --- | --- |
| Bash | 13 |
| Read | 4 |
| Write | 4 |
| Edit | 2 |
| Craft Anything MCP tools | 12 in all: `rebuild_check` 2, `core_plan_update` 3, and 1 each of `core_investigation_show`, `core_person_ask`, `core_session_show`, `core_unknown_add`, `core_unknown_edit`, `enable_toolset`, `list_toolsets` |
| WebFetch, WebSearch | 0 |

- Tool calls whose input contains the site's address: 0. The address appears in the request Craft Anything sent the builder and in tool results that read the capture.
- Bash calls naming `curl`, `wget`, `urllib`, `requests`, `http.client`, `nc` or `ssh`: 0. One Bash call ran `npm install` in the build's own `app/` folder, which reaches the npm registry, not the site.
- Files the builder read, through Read or `cat`: the capture (README, `capture.json`, the three screenshots, both `states.json`), the project's `craft.json`, `feed.json` and inbox file, and its own build output and comparison images.
- In 0.5.0 the builder's `--tools` list is Bash, PowerShell, Read, Write, Edit, MultiEdit, Glob, Grep, TodoWrite and NotebookEdit, which leaves out WebFetch and WebSearch (`crates/ai/src/cli_agents/adapters.rs`, line 395 at v0.5.0). Bash can still reach the network, which is why the Bash count above matters.

These counts show which tools the builder called and with what input. They don't show what a command did beyond its text.
