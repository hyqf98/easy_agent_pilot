<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import MarkdownRenderer from './MarkdownRenderer.vue'
import type { RuntimeNotice } from '@/utils/runtimeNotice'
import { getUsageNoticeSummary, summarizeRuntimeNotice } from '@/utils/runtimeNotice'
import { resolveRecordedModelId } from '@/services/usage/agentCliUsageRecorder'

interface UsageFallback {
  model?: string
  inputTokens?: number
  outputTokens?: number
}

const props = withDefaults(defineProps<{
  notices: RuntimeNotice[]
  defaultExpanded?: boolean
  fallbackUsage?: UsageFallback | null
  compactContextSummary?: boolean
}>(), {
  defaultExpanded: false,
  fallbackUsage: null,
  compactContextSummary: false
})
const { t } = useI18n()

const expandedIds = ref<Set<string>>(new Set(
  props.defaultExpanded ? props.notices.map(notice => notice.id) : []
))

const usageNotices = computed(() => props.notices.filter(isUsageNotice))
const regularNotices = computed(() => props.notices.filter(notice => !isUsageNotice(notice)))
const primaryRegularNotice = computed(() => regularNotices.value[0] ?? null)
const extraRegularNotices = computed(() => regularNotices.value.slice(1))
const primaryUsageNotice = computed(() => usageNotices.value[0] ?? null)
const extraUsageNotices = computed(() => usageNotices.value.slice(1))
const shouldUseCombinedSummary = computed(() => Boolean(primaryRegularNotice.value && primaryUsageNotice.value))
const requestedModelFallback = computed(() => {
  const fromRegularNotice = regularNotices.value
    .map(notice => extractModelFromNotice(notice))
    .find(Boolean)

  return fromRegularNotice || props.fallbackUsage?.model?.trim() || null
})

function toggleNotice(id: string) {
  const next = new Set(expandedIds.value)
  if (next.has(id)) {
    next.delete(id)
  } else {
    next.add(id)
  }
  expandedIds.value = next
}

function isExpanded(id: string) {
  return expandedIds.value.has(id)
}

function isUsageNotice(notice: RuntimeNotice) {
  return notice.id === 'usage' || notice.title.toLowerCase().includes('model')
}

function isEnvironmentNotice(notice: RuntimeNotice) {
  return notice.id === 'environment'
}

function isCompactContextNotice(notice: RuntimeNotice) {
  return props.compactContextSummary && notice.id === 'context'
}

function noticeChips(notice: RuntimeNotice) {
  return summarizeRuntimeNotice(notice).map(chip => formatChipLabel(notice, chip))
}

function extractNoticeFieldValue(notice: RuntimeNotice, labels: string[]) {
  const lines = notice.content
    .split('\n')
    .map(line => line.trim())
    .filter(Boolean)

  for (const line of lines) {
    const normalizedLine = line.replace(/^-\s*/, '')
    const separatorIndex = normalizedLine.indexOf(':')
    if (separatorIndex < 0) {
      continue
    }

    const label = normalizedLine.slice(0, separatorIndex).trim().toLowerCase()
    const value = normalizedLine.slice(separatorIndex + 1).trim()
    if (!value) {
      continue
    }

    if (labels.includes(label)) {
      return value
    }
  }

  return null
}

function compactContextNoticeChips(notice: RuntimeNotice) {
  const chips = [
    extractNoticeFieldValue(notice, ['模型', 'model']),
    extractNoticeFieldValue(notice, ['专家', 'expert'])
  ].filter((value): value is string => Boolean(value))

  return chips.length > 0 ? chips : noticeChips(notice).slice(0, 2)
}

function usageSummary(notice: RuntimeNotice) {
  const summary = getUsageNoticeSummary(notice)
  const fallback = props.fallbackUsage
  const requestedModel = requestedModelFallback.value
  const hasUsageValue = (value: unknown) => value !== null && value !== undefined && value !== ''

  if (!summary) {
    if (!fallback) {
      return null
    }

    const hasFallbackInput = typeof fallback.inputTokens === 'number'
    const hasFallbackOutput = typeof fallback.outputTokens === 'number'

    return {
      model: resolveRecordedModelId({
        reportedModelId: fallback.model,
        requestedModelId: requestedModel
      }) || requestedModel || fallback.model || null,
      input: hasFallbackInput ? String(fallback.inputTokens) : null,
      output: hasFallbackOutput ? String(fallback.outputTokens) : null
    }
  }

  const hasRealInput = hasUsageValue(summary.input)
  const hasRealOutput = hasUsageValue(summary.output)
  const fallbackInput = typeof fallback?.inputTokens === 'number' ? String(fallback.inputTokens) : null
  const fallbackOutput = typeof fallback?.outputTokens === 'number' ? String(fallback.outputTokens) : null

  return {
    model: resolveRecordedModelId({
      reportedModelId: summary.model || fallback?.model,
      requestedModelId: requestedModel
    }) || requestedModel || summary.model || fallback?.model || null,
    input: hasRealInput ? summary.input : (fallbackInput || null),
    output: hasRealOutput ? summary.output : (fallbackOutput || null)
  }
}

function extractModelFromNotice(notice: RuntimeNotice): string | null {
  const match = notice.content.match(/(?:^|\n)-?\s*(?:模型|model)\s*:\s*(.+)$/im)
  return match?.[1]?.trim() || null
}

function usageModelLabel(notice: RuntimeNotice) {
  return usageSummary(notice)?.model || requestedModelFallback.value || '—'
}

function formatChipLabel(notice: RuntimeNotice, chip: string) {
  if (!isEnvironmentNotice(notice)) {
    return chip
  }

  return chip
    .replace(/^Skills\s+/i, 'Sk ')
    .replace(/^Plugins\s+/i, 'Pl ')
    .replace(/^Agents\s+/i, 'Ag ')
    .replace(/^Commands\s+/i, 'Cmd ')
  }
</script>

<template>
  <div class="runtime-notice-list">
    <article
      v-if="shouldUseCombinedSummary && primaryRegularNotice && primaryUsageNotice"
      class="runtime-notice runtime-notice--summary"
      :class="`runtime-notice--${primaryRegularNotice.tone || 'info'}`"
    >
      <div class="runtime-notice__summary">
        <button
          type="button"
          class="runtime-notice__summary-runtime"
          @click="toggleNotice(primaryRegularNotice.id)"
        >
          <div class="runtime-notice__summary-runtime-main">
            <div class="runtime-notice__header-main">
              <span class="runtime-notice__eyebrow">{{ t('message.runtimeNotice.runtime') }}</span>
              <span class="runtime-notice__title">{{ primaryRegularNotice.title }}</span>
            </div>
            <div
              v-if="noticeChips(primaryRegularNotice).length > 0"
              class="runtime-notice__chips runtime-notice__chips--leading"
            >
              <span
                v-for="chip in noticeChips(primaryRegularNotice)"
                :key="chip"
                class="runtime-notice__chip"
              >
                {{ chip }}
              </span>
            </div>
          </div>
          <span
            class="runtime-notice__chevron"
            :class="{ 'runtime-notice__chevron--expanded': isExpanded(primaryRegularNotice.id) }"
          >▼</span>
        </button>

        <div class="runtime-notice__summary-usage">
          <div class="runtime-notice__usage-main">
            <span class="runtime-notice__usage-label">{{ t('message.runtimeNotice.model') }}</span>
            <span class="runtime-notice__usage-model">
              {{ usageModelLabel(primaryUsageNotice) }}
            </span>
          </div>
          <div class="runtime-notice__usage-stats">
            <span class="runtime-notice__usage-chip runtime-notice__usage-chip--input">
              {{ t('message.runtimeNotice.input') }} {{ usageSummary(primaryUsageNotice)?.input || '—' }}
            </span>
            <span class="runtime-notice__usage-chip runtime-notice__usage-chip--output">
              {{ t('message.runtimeNotice.output') }} {{ usageSummary(primaryUsageNotice)?.output || '—' }}
            </span>
          </div>
        </div>
      </div>

      <div
        v-show="isExpanded(primaryRegularNotice.id)"
        class="runtime-notice__content"
      >
        <MarkdownRenderer :content="primaryRegularNotice.content" />
      </div>
    </article>

    <template v-if="shouldUseCombinedSummary">
      <article
        v-for="notice in extraRegularNotices"
        :key="notice.id"
        class="runtime-notice"
        :class="`runtime-notice--${notice.tone || 'info'}`"
      >
        <button
          type="button"
          class="runtime-notice__header"
          @click="toggleNotice(notice.id)"
        >
          <div class="runtime-notice__header-main">
            <span class="runtime-notice__eyebrow">{{ t('message.runtimeNotice.runtime') }}</span>
            <span class="runtime-notice__title">{{ notice.title }}</span>
          </div>
          <div class="runtime-notice__header-side">
            <div
              v-if="noticeChips(notice).length > 0"
              class="runtime-notice__chips"
            >
              <span
                v-for="chip in noticeChips(notice)"
                :key="chip"
                class="runtime-notice__chip"
              >
                {{ chip }}
              </span>
            </div>
            <span
              class="runtime-notice__chevron"
              :class="{ 'runtime-notice__chevron--expanded': isExpanded(notice.id) }"
            >▼</span>
          </div>
        </button>

        <div
          v-show="isExpanded(notice.id)"
          class="runtime-notice__content"
        >
          <MarkdownRenderer :content="notice.content" />
        </div>
      </article>

      <article
        v-for="notice in extraUsageNotices"
        :key="notice.id"
        class="runtime-notice runtime-notice--usage"
        :class="`runtime-notice--${notice.tone || 'info'}`"
      >
        <div class="runtime-notice__usage">
          <div class="runtime-notice__usage-main">
            <span class="runtime-notice__usage-label">{{ t('message.runtimeNotice.model') }}</span>
            <span class="runtime-notice__usage-model">
              {{ usageModelLabel(notice) }}
            </span>
          </div>
          <div class="runtime-notice__usage-stats">
            <span class="runtime-notice__usage-chip runtime-notice__usage-chip--input">
              {{ t('message.runtimeNotice.input') }} {{ usageSummary(notice)?.input || '—' }}
            </span>
            <span class="runtime-notice__usage-chip runtime-notice__usage-chip--output">
              {{ t('message.runtimeNotice.output') }} {{ usageSummary(notice)?.output || '—' }}
            </span>
          </div>
        </div>
      </article>
    </template>

    <template v-else>
      <article
        v-for="notice in notices"
        :key="notice.id"
        class="runtime-notice"
        :class="[
          `runtime-notice--${notice.tone || 'info'}`,
          { 'runtime-notice--usage': isUsageNotice(notice) }
        ]"
      >
        <div
          v-if="isUsageNotice(notice)"
          class="runtime-notice__usage"
        >
          <div class="runtime-notice__usage-main">
            <span class="runtime-notice__usage-label">{{ t('message.runtimeNotice.model') }}</span>
            <span class="runtime-notice__usage-model">
              {{ usageModelLabel(notice) }}
            </span>
          </div>
          <div class="runtime-notice__usage-stats">
            <span class="runtime-notice__usage-chip runtime-notice__usage-chip--input">
              {{ t('message.runtimeNotice.input') }} {{ usageSummary(notice)?.input || '—' }}
            </span>
            <span class="runtime-notice__usage-chip runtime-notice__usage-chip--output">
              {{ t('message.runtimeNotice.output') }} {{ usageSummary(notice)?.output || '—' }}
            </span>
          </div>
        </div>

        <template v-else>
          <div
            v-if="isCompactContextNotice(notice)"
            class="runtime-notice__header runtime-notice__header--static"
          >
            <div class="runtime-notice__header-main">
              <span class="runtime-notice__eyebrow">{{ t('message.runtimeNotice.runtime') }}</span>
              <span class="runtime-notice__title">{{ notice.title }}</span>
            </div>
            <div class="runtime-notice__header-side">
              <div
                v-if="compactContextNoticeChips(notice).length > 0"
                class="runtime-notice__chips"
              >
                <span
                  v-for="chip in compactContextNoticeChips(notice)"
                  :key="chip"
                  class="runtime-notice__chip"
                >
                  {{ chip }}
                </span>
              </div>
            </div>
          </div>

          <button
            v-else
            type="button"
            class="runtime-notice__header"
            @click="toggleNotice(notice.id)"
          >
            <div class="runtime-notice__header-main">
              <span class="runtime-notice__eyebrow">{{ t('message.runtimeNotice.runtime') }}</span>
              <span class="runtime-notice__title">{{ notice.title }}</span>
            </div>
            <div class="runtime-notice__header-side">
              <div
                v-if="noticeChips(notice).length > 0"
                class="runtime-notice__chips"
              >
                <span
                  v-for="chip in noticeChips(notice)"
                  :key="chip"
                  class="runtime-notice__chip"
                >
                  {{ chip }}
                </span>
              </div>
              <span
                class="runtime-notice__chevron"
                :class="{ 'runtime-notice__chevron--expanded': isExpanded(notice.id) }"
              >▼</span>
            </div>
          </button>

          <div
            v-if="!isCompactContextNotice(notice)"
            v-show="isExpanded(notice.id)"
            class="runtime-notice__content"
          >
            <MarkdownRenderer :content="notice.content" />
          </div>
        </template>
      </article>
    </template>
  </div>
</template>

<style scoped>
.runtime-notice-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  width: 100%;
}

.runtime-notice {
  border-radius: 0.85rem;
  border: 1px solid var(--runtime-notice-border);
  background: var(--runtime-notice-bg);
  box-shadow: 0 10px 22px rgba(15, 23, 42, 0.04);
  overflow: hidden;
}

.runtime-notice--summary {
  overflow: visible;
}

.runtime-notice--usage {
  border-color: var(--runtime-notice-usage-border);
  background: var(--runtime-notice-usage-bg);
}

.runtime-notice--info {
  border-color: rgba(14, 165, 233, 0.12);
}

.runtime-notice--success {
  border-color: rgba(34, 197, 94, 0.14);
}

.runtime-notice--warning {
  border-color: rgba(245, 158, 11, 0.16);
}

.runtime-notice__summary {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 0.55rem;
  padding: 0.55rem 0.7rem;
}

.runtime-notice__summary-runtime,
.runtime-notice__header {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.65rem;
  padding: 0.55rem 0.7rem;
  border: 0;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.runtime-notice__summary-runtime {
  padding: 0;
  min-width: 0;
  align-items: flex-start;
}

.runtime-notice__summary-runtime:hover,
.runtime-notice__header:hover {
  background: var(--runtime-notice-hover);
}

.runtime-notice__header--static {
  cursor: default;
}

.runtime-notice__header-main {
  min-width: 0;
  display: flex;
  align-items: baseline;
  gap: 0.45rem;
  flex-wrap: wrap;
}

.runtime-notice__summary-runtime-main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.runtime-notice__eyebrow,
.runtime-notice__usage-label {
  font-size: 0.58rem;
  line-height: 1;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.runtime-notice__title {
  min-width: 0;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.runtime-notice__header-side {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.45rem;
  min-width: 0;
  flex: 1;
}

.runtime-notice__chips {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.28rem;
  min-width: 0;
}

.runtime-notice__chips--leading {
  justify-content: flex-start;
}

.runtime-notice__chip,
.runtime-notice__usage-chip {
  max-width: 100%;
  padding: 0.18rem 0.42rem;
  border-radius: 999px;
  background: var(--runtime-notice-chip-bg);
  color: var(--runtime-notice-chip-text);
  font-size: 0.64rem;
  line-height: 1.1;
  white-space: nowrap;
}

.runtime-notice__chevron {
  flex-shrink: 0;
  font-size: 0.66rem;
  color: var(--color-text-tertiary);
  transition: transform 0.18s ease;
}

.runtime-notice__chevron--expanded {
  transform: rotate(180deg);
}

.runtime-notice__content {
  padding: 0 0.9rem 0.85rem;
  border-top: 1px solid var(--runtime-notice-content-border);
}

.runtime-notice__usage,
.runtime-notice__summary-usage {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.55rem;
  min-width: 0;
  flex-wrap: wrap;
}

.runtime-notice__usage {
  padding: 0.58rem 0.72rem;
}

.runtime-notice__summary-usage {
  padding-top: 0.18rem;
  border-top: 1px solid var(--runtime-notice-content-border);
}

.runtime-notice__usage-main {
  min-width: 0;
  display: flex;
  align-items: baseline;
  gap: 0.42rem;
}

.runtime-notice__usage-model {
  min-width: 0;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--runtime-notice-usage-model);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.runtime-notice__usage-stats {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.28rem;
  min-width: 0;
  flex-wrap: wrap;
}

.runtime-notice__usage-chip {
  background: rgba(37, 99, 235, 0.08);
}

.runtime-notice__usage-chip--output {
  background: rgba(14, 165, 233, 0.1);
}

@media (max-width: 768px) {
  .runtime-notice__summary-usage,
  .runtime-notice__usage,
  .runtime-notice__header,
  .runtime-notice__summary-runtime {
    flex-wrap: wrap;
  }

  .runtime-notice__header-side,
  .runtime-notice__usage-stats {
    width: 100%;
    justify-content: flex-start;
  }
}
</style>
