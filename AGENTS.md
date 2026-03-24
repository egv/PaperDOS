# AGENTS

- `br` refers to the Beads issue tracker CLI, not a shorthand for bring-up or roadmap status.
- When the user asks about epics, ready work, blocked work, or issue status, query `br` directly instead of inferring from git history or design docs.
- Useful defaults: `br epic status`, `br ready`, `br blocked`, `br show <issue-id>`, and `br list`.
- If code and tracker status disagree, report the `br` result clearly and call out that the Beads tracker may need updating.
- Use the `pulp-os` physical button names in discussion to avoid ambiguity: `VolUp`, `VolDown`, `Confirm`, `Back`, `Left`, `Right`, and `Power`.
- Current ABI mapping: `PD_BTN_UP` = `VolUp`, `PD_BTN_DOWN` = `VolDown`, and `PD_BTN_OK` = `Confirm`.
