<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { StructuredExecutionResult } from '@/utils/structuredContent'

const props = defineProps<{
  result: StructuredExecutionResult
  title?: string
}>()

const { t } = useI18n()

const fileGroups = computed(() => [
  {
    key: 'generated',
    label: t('message.structured.generatedFiles'),
    files: props.result.generatedFiles
  },
  {
    key: 'modified',
    label: t('message.structured.modifiedFiles'),
    files: props.result.modifiedFiles
  },
  {
    key: 'changed',
    label: t('message.structured.changedFiles'),
    files: props.result.changedFiles
  },
  {
    key: 'deleted',
    label: t('message.structured.deletedFiles'),
    files: props.result.deletedFiles
  }
].filter(group => group.files.length > 0))
</script>

<template>
  <section class="structured-result-card">
    <header
      v-if="title"
      class="structured-result-card__header"
    >
      <span class="structured-result-card__title">{{ title }}</span>
    </header>

    <div
      v-if="result.summary"
      class="structured-result-card__section"
    >
      <div class="structured-result-card__label">
        {{ t('message.structured.summary') }}
      </div>
      <p class="structured-result-card__summary">
        {{ result.summary }}
      </p>
    </div>

    <div
      v-for="group in fileGroups"
      :key="group.key"
      class="structured-result-card__section"
    >
      <div class="structured-result-card__label">
        {{ group.label }}
      </div>
      <ul class="structured-result-card__files">
        <li
          v-for="file in group.files"
          :key="`${group.key}-${file}`"
        >
          {{ file }}
        </li>
      </ul>
    </div>
  </section>
</template>

<style scoped>
.structured-result-card {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
  border-radius: 1rem;
  border: 1px solid rgba(59, 130, 246, 0.14);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.97), rgba(248, 250, 252, 0.95));
  padding: 0.95rem 1rem;
  box-shadow: 0 12px 30px rgba(15, 23, 42, 0.05);
}

.structured-result-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.structured-result-card__title {
  font-size: 0.82rem;
  font-weight: 700;
  color: var(--color-text-primary);
}

.structured-result-card__section {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.structured-result-card__label {
  font-size: 0.75rem;
  font-weight: 700;
  letter-spacing: 0.02em;
  color: color-mix(in srgb, var(--color-primary) 82%, #0f172a);
  text-transform: uppercase;
}

.structured-result-card__summary {
  margin: 0;
  color: var(--color-text-primary);
  line-height: 1.65;
  white-space: pre-wrap;
  word-break: break-word;
}

.structured-result-card__files {
  display: grid;
  gap: 0.35rem;
  margin: 0;
  padding-left: 1rem;
  color: var(--color-text-secondary);
}

:global(.dark) .structured-result-card {
  border-color: rgba(96, 165, 250, 0.2);
  background:
    linear-gradient(180deg, rgba(15, 23, 42, 0.96), rgba(17, 24, 39, 0.94));
}

:global(.dark) .structured-result-card__label {
  color: color-mix(in srgb, var(--color-primary) 76%, #e2e8f0);
}
</style>
