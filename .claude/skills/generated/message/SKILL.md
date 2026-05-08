---
name: message
description: "Skill for the Message area of easy-agent-pilot. 30 symbols across 6 files."
---

# Message

30 symbols | 6 files | Cohesion: 82%

## When to Use

- Working with code in `src/`
- Understanding how isEnvironmentNotice, noticeChips, extractNoticeFieldValue work
- Modifying message-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/components/message/RuntimeNoticeList.vue` | isEnvironmentNotice, noticeChips, extractNoticeFieldValue, compactContextNoticeChips, formatChipLabel (+5) |
| `src/components/message/MarkdownRenderer.vue` | normalizePath, trimTrailingSlash, normalizeComparablePath, joinProjectPath, resolveProjectFileTarget (+5) |
| `src/components/message/ExecutionTimeline.vue` | sortToolEntries, statusWeight, isToolGroupExpanded, toggleToolGroup |
| `src/components/message/StructuredContentRenderer.vue` | getResolvedFormValues, isFormResolved, isFormDisabled |
| `src/utils/fileIcon.ts` | match, test |
| `src/components/memory/MemoryMarkdownEditor.vue` | parseShortcut |

## Entry Points

Start here when exploring this area:

- **`isEnvironmentNotice`** (Function) — `src/components/message/RuntimeNoticeList.vue:65`
- **`noticeChips`** (Function) — `src/components/message/RuntimeNoticeList.vue:73`
- **`extractNoticeFieldValue`** (Function) — `src/components/message/RuntimeNoticeList.vue:77`
- **`compactContextNoticeChips`** (Function) — `src/components/message/RuntimeNoticeList.vue:104`
- **`formatChipLabel`** (Function) — `src/components/message/RuntimeNoticeList.vue:185`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `isEnvironmentNotice` | Function | `src/components/message/RuntimeNoticeList.vue` | 65 |
| `noticeChips` | Function | `src/components/message/RuntimeNoticeList.vue` | 73 |
| `extractNoticeFieldValue` | Function | `src/components/message/RuntimeNoticeList.vue` | 77 |
| `compactContextNoticeChips` | Function | `src/components/message/RuntimeNoticeList.vue` | 104 |
| `formatChipLabel` | Function | `src/components/message/RuntimeNoticeList.vue` | 185 |
| `usageSummary` | Function | `src/components/message/RuntimeNoticeList.vue` | 113 |
| `hasUsageValue` | Function | `src/components/message/RuntimeNoticeList.vue` | 117 |
| `toUsageNumber` | Function | `src/components/message/RuntimeNoticeList.vue` | 167 |
| `extractUsageNoticeContextOccupancy` | Function | `src/components/message/RuntimeNoticeList.vue` | 176 |
| `usageModelLabel` | Function | `src/components/message/RuntimeNoticeList.vue` | 181 |
| `normalizePath` | Function | `src/components/message/MarkdownRenderer.vue` | 55 |
| `trimTrailingSlash` | Function | `src/components/message/MarkdownRenderer.vue` | 59 |
| `normalizeComparablePath` | Function | `src/components/message/MarkdownRenderer.vue` | 63 |
| `joinProjectPath` | Function | `src/components/message/MarkdownRenderer.vue` | 115 |
| `resolveProjectFileTarget` | Function | `src/components/message/MarkdownRenderer.vue` | 119 |
| `isLikelyLocalFileHref` | Function | `src/components/message/MarkdownRenderer.vue` | 96 |
| `parseShortcut` | Function | `src/components/memory/MemoryMarkdownEditor.vue` | 318 |
| `safeDecodeUriComponent` | Function | `src/components/message/MarkdownRenderer.vue` | 47 |
| `stripFileProtocol` | Function | `src/components/message/MarkdownRenderer.vue` | 67 |
| `stripLineSuffix` | Function | `src/components/message/MarkdownRenderer.vue` | 85 |

## Connected Areas

| Area | Connections |
|------|-------------|
| MessageBubble | 2 calls |
| Usage | 1 calls |

## How to Explore

1. `gitnexus_context({name: "isEnvironmentNotice"})` — see callers and callees
2. `gitnexus_query({query: "message"})` — find related execution flows
3. Read key files listed above for implementation details
