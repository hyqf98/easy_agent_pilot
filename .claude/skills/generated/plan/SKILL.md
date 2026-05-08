---
name: plan
description: "Skill for the Plan area of easy-agent-pilot. 58 symbols across 19 files."
---

# Plan

58 symbols | 19 files | Cohesion: 81%

## When to Use

- Working with code in `src/`
- Understanding how setCurrentViewingTask, setCurrentTask, setCurrentPlan work
- Modifying plan-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/services/plan/prompts.ts` | buildCodexPlanSplitFieldSchema, buildCodexPlanSplitFormSchema, buildCodexPlanSplitTaskSchema, buildCodexPlanSplitJsonSchema, buildPlanSplitJsonSchema (+14) |
| `src/services/plan/ProgressManager.ts` | getProgressFilePath, createEmptyProgress, readProgress, writeProgress, updateSummary (+5) |
| `src/services/plan/FormEngine.ts` | extractFormRequest, parseXmlFormRequest, parseFormRequest, validateSchema, mergeDefaultValues (+2) |
| `src/components/plan/PlanModePanel.vue` | handlePlanClick, handleTaskClick, handlePlanTaskSelect |
| `src/stores/taskExecution.ts` | setCurrentViewingTask, parseFormRequest |
| `src/utils/structuredContent.ts` | extractFirstFormRequest, extractFirstFormRequestFromContents |
| `src/components/plan/TaskResplitModal.vue` | close, handleConfirm |
| `src/components/plan/TaskListOptimizeModal.vue` | close, handleConfirm |
| `src/stores/task.ts` | setCurrentTask |
| `src/stores/plan.ts` | setCurrentPlan |

## Entry Points

Start here when exploring this area:

- **`setCurrentViewingTask`** (Function) — `src/stores/taskExecution.ts:2027`
- **`setCurrentTask`** (Function) — `src/stores/task.ts:519`
- **`setCurrentPlan`** (Function) — `src/stores/plan.ts:342`
- **`goToDependency`** (Function) — `src/components/plan/TaskDetail.vue:136`
- **`handlePlanClick`** (Function) — `src/components/plan/PlanModePanel.vue:68`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `setCurrentViewingTask` | Function | `src/stores/taskExecution.ts` | 2027 |
| `setCurrentTask` | Function | `src/stores/task.ts` | 519 |
| `setCurrentPlan` | Function | `src/stores/plan.ts` | 342 |
| `goToDependency` | Function | `src/components/plan/TaskDetail.vue` | 136 |
| `handlePlanClick` | Function | `src/components/plan/PlanModePanel.vue` | 68 |
| `handleTaskClick` | Function | `src/components/plan/PlanModePanel.vue` | 76 |
| `handlePlanTaskSelect` | Function | `src/components/plan/PlanModePanel.vue` | 95 |
| `extractFirstFormRequest` | Function | `src/utils/structuredContent.ts` | 841 |
| `extractFirstFormRequestFromContents` | Function | `src/utils/structuredContent.ts` | 977 |
| `activeFormQuestion` | Function | `src/components/plan/taskSplitDialog/useTaskSplitDialog.ts` | 838 |
| `buildPlanSplitJsonSchema` | Function | `src/services/plan/prompts.ts` | 649 |
| `buildPlanSplitSystemPrompt` | Function | `src/services/plan/prompts.ts` | 92 |
| `buildTaskListOptimizeKickoffPrompt` | Function | `src/services/plan/prompts.ts` | 171 |
| `buildOutputCorrectionPrompt` | Function | `src/services/plan/prompts.ts` | 293 |
| `appendPlanSplitInstructionGuard` | Function | `src/services/plan/prompts.ts` | 46 |
| `buildPlanSplitKickoffPrompt` | Function | `src/services/plan/prompts.ts` | 96 |
| `buildTaskResplitKickoffPrompt` | Function | `src/services/plan/prompts.ts` | 109 |
| `buildFormResponsePrompt` | Function | `src/services/plan/prompts.ts` | 281 |
| `resolvePlanTaskAgentSelection` | Function | `src/utils/planExecutionProgress.ts` | 35 |
| `getExpertById` | Function | `src/stores/agentTeams.ts` | 232 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `SplitChatMessages → ExtractBalancedJsonRanges` | cross_community | 6 |
| `SplitChatMessages → PushMarkdownBlock` | cross_community | 6 |
| `SplitChatMessages → SetStructuredContentCache` | cross_community | 5 |
| `StartSubSplit → IsChineseLocale` | cross_community | 4 |
| `StartListOptimize → IsChineseLocale` | cross_community | 4 |
| `StartListOptimize → T` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Stores | 1 calls |
| MessageBubble | 1 calls |
| Conversation | 1 calls |

## How to Explore

1. `gitnexus_context({name: "setCurrentViewingTask"})` — see callers and callees
2. `gitnexus_query({query: "plan"})` — find related execution flows
3. Read key files listed above for implementation details
