<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **easy-agent-pilot** (16347 symbols, 30791 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/easy-agent-pilot/context` | Codebase overview, check index freshness |
| `gitnexus://repo/easy-agent-pilot/clusters` | All functional areas |
| `gitnexus://repo/easy-agent-pilot/processes` | All execution flows |
| `gitnexus://repo/easy-agent-pilot/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |
| Work in the Stores area (993 symbols) | `.claude/skills/generated/stores/SKILL.md` |
| Work in the Commands area (872 symbols) | `.claude/skills/generated/commands/SKILL.md` |
| Work in the Strategies area (189 symbols) | `.claude/skills/generated/strategies/SKILL.md` |
| Work in the Composables area (149 symbols) | `.claude/skills/generated/composables/SKILL.md` |
| Work in the Conversation area (105 symbols) | `.claude/skills/generated/conversation/SKILL.md` |
| Work in the Unattended area (103 symbols) | `.claude/skills/generated/unattended/SKILL.md` |
| Work in the FileTree area (99 symbols) | `.claude/skills/generated/filetree/SKILL.md` |
| Work in the TaskSplitDialog area (70 symbols) | `.claude/skills/generated/tasksplitdialog/SKILL.md` |
| Work in the Plan area (58 symbols) | `.claude/skills/generated/plan/SKILL.md` |
| Work in the Tabs area (51 symbols) | `.claude/skills/generated/tabs/SKILL.md` |
| Work in the Memory area (47 symbols) | `.claude/skills/generated/memory/SKILL.md` |
| Work in the MessageBubble area (33 symbols) | `.claude/skills/generated/messagebubble/SKILL.md` |
| Work in the MemoryAuthoringDialog area (30 symbols) | `.claude/skills/generated/memoryauthoringdialog/SKILL.md` |
| Work in the Message area (30 symbols) | `.claude/skills/generated/message/SKILL.md` |
| Work in the RichMarkdownEditor area (29 symbols) | `.claude/skills/generated/richmarkdowneditor/SKILL.md` |
| Work in the TaskExecutionLog area (29 symbols) | `.claude/skills/generated/taskexecutionlog/SKILL.md` |
| Work in the Usage area (28 symbols) | `.claude/skills/generated/usage/SKILL.md` |
| Work in the MessageList area (27 symbols) | `.claude/skills/generated/messagelist/SKILL.md` |
| Work in the MonacoCodeEditor area (27 symbols) | `.claude/skills/generated/monacocodeeditor/SKILL.md` |
| Work in the Compression area (25 symbols) | `.claude/skills/generated/compression/SKILL.md` |

<!-- gitnexus:end -->
