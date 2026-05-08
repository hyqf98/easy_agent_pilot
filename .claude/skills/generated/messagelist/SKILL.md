---
name: messagelist
description: "Skill for the MessageList area of easy-agent-pilot. 27 symbols across 1 files."
---

# MessageList

27 symbols | 1 files | Cohesion: 87%

## When to Use

- Working with code in `src/`
- Understanding how useMessageList, getSessionMessages, buildMessageLayout work
- Modifying messagelist-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/components/message/messageList/useMessageList.ts` | detectWindowsRuntime, useMessageList, getSessionMessages, buildMessageLayout, persistMessageHeights (+22) |

## Entry Points

Start here when exploring this area:

- **`useMessageList`** (Function) — `src/components/message/messageList/useMessageList.ts:70`
- **`getSessionMessages`** (Function) — `src/components/message/messageList/useMessageList.ts:240`
- **`buildMessageLayout`** (Function) — `src/components/message/messageList/useMessageList.ts:252`
- **`persistMessageHeights`** (Function) — `src/components/message/messageList/useMessageList.ts:263`
- **`restoreMessageHeights`** (Function) — `src/components/message/messageList/useMessageList.ts:275`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `useMessageList` | Function | `src/components/message/messageList/useMessageList.ts` | 70 |
| `getSessionMessages` | Function | `src/components/message/messageList/useMessageList.ts` | 240 |
| `buildMessageLayout` | Function | `src/components/message/messageList/useMessageList.ts` | 252 |
| `persistMessageHeights` | Function | `src/components/message/messageList/useMessageList.ts` | 263 |
| `restoreMessageHeights` | Function | `src/components/message/messageList/useMessageList.ts` | 275 |
| `getMessageOffsetTop` | Function | `src/components/message/messageList/useMessageList.ts` | 284 |
| `resolveAnchorAtScrollTop` | Function | `src/components/message/messageList/useMessageList.ts` | 299 |
| `resolveAnchorScrollTop` | Function | `src/components/message/messageList/useMessageList.ts` | 330 |
| `isRestoringScroll` | Function | `src/components/message/messageList/useMessageList.ts` | 348 |
| `clearRestoreGuard` | Function | `src/components/message/messageList/useMessageList.ts` | 352 |
| `finishRestoreGuard` | Function | `src/components/message/messageList/useMessageList.ts` | 360 |
| `saveScrollSnapshot` | Function | `src/components/message/messageList/useMessageList.ts` | 381 |
| `applyScrollSnapshot` | Function | `src/components/message/messageList/useMessageList.ts` | 398 |
| `restoreScrollSnapshot` | Function | `src/components/message/messageList/useMessageList.ts` | 418 |
| `getBottomScrollTop` | Function | `src/components/message/messageList/useMessageList.ts` | 492 |
| `clearForceBottomAlignTask` | Function | `src/components/message/messageList/useMessageList.ts` | 514 |
| `scrollToBottom` | Function | `src/components/message/messageList/useMessageList.ts` | 521 |
| `queueBottomAlignment` | Function | `src/components/message/messageList/useMessageList.ts` | 529 |
| `forceBottomAlignment` | Function | `src/components/message/messageList/useMessageList.ts` | 570 |
| `updateViewportMetrics` | Function | `src/components/message/messageList/useMessageList.ts` | 591 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `HandleViewportResize → GetSessionMessages` | intra_community | 5 |
| `HandleViewportResize → GetMessageOffsetTop` | intra_community | 5 |
| `HandleViewportResize → BuildMessageLayout` | intra_community | 5 |
| `UseMessageList → SaveState` | cross_community | 4 |
| `HandleViewportResize → GetBottomScrollTop` | intra_community | 4 |
| `HandleViewportResize → IsRestoringScroll` | intra_community | 4 |
| `HandleScroll → GetSessionMessages` | cross_community | 4 |
| `HandleScroll → GetMessageOffsetTop` | cross_community | 4 |
| `HandleScroll → BuildMessageLayout` | cross_community | 4 |
| `UseMessageList → SaveOpenSessions` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Stores | 4 calls |
| TaskSplitDialog | 1 calls |

## How to Explore

1. `gitnexus_context({name: "useMessageList"})` — see callers and callees
2. `gitnexus_query({query: "messagelist"})` — find related execution flows
3. Read key files listed above for implementation details
