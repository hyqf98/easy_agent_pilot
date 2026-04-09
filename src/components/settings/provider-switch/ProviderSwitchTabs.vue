<script setup lang="ts">
import type { CliType } from '@/stores/providerProfile'
import { EaIcon } from '@/components/common'
import { useI18n } from 'vue-i18n'

defineProps<{
  currentCliType: CliType
}>()

const emit = defineEmits<{
  change: [type: CliType]
}>()

const { t } = useI18n()
</script>

<template>
  <div class="cli-type-tabs">
    <div class="tabs-wrapper">
      <button
        :class="['tab-btn', { active: currentCliType === 'claude' }]"
        @click="emit('change', 'claude')"
      >
        <EaIcon
          name="terminal"
          :size="16"
        />
        <span>{{ t('settings.providerSwitch.cliType.claude') }}</span>
      </button>
      <button
        :class="['tab-btn', { active: currentCliType === 'codex' }]"
        @click="emit('change', 'codex')"
      >
        <EaIcon
          name="code"
          :size="16"
        />
        <span>{{ t('settings.providerSwitch.cliType.codex') }}</span>
      </button>
      <button
        :class="['tab-btn', { active: currentCliType === 'opencode' }]"
        @click="emit('change', 'opencode')"
      >
        <EaIcon
          name="terminal"
          :size="16"
        />
        <span>{{ t('settings.providerSwitch.cliType.opencode') }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.cli-type-tabs {
  margin-bottom: 24px;
}

.tabs-wrapper {
  display: inline-flex;
  gap: 4px;
  padding: 4px;
  border: 1px solid var(--color-border, #d7e1ec);
  border-radius: 10px;
  background: var(--color-bg-secondary, #f4f7fb);
}

.tab-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--color-text-secondary, #667085);
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: all 0.2s ease;
}

.tab-btn:hover {
  background: var(--color-surface-hover, rgba(15, 23, 42, 0.05));
  color: var(--color-text-primary, #0f172a);
}

.tab-btn.active {
  background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
  color: #ffffff;
  box-shadow: 0 8px 18px rgba(37, 99, 235, 0.22);
}

:global(.dark) .tabs-wrapper {
  border-color: var(--color-border, #334155);
  background: var(--color-bg-secondary, #1f2937);
}

:global(.dark) .tab-btn {
  color: var(--color-text-secondary, #cbd5e1);
}

:global(.dark) .tab-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--color-text-primary, #ffffff);
}

:global(.dark) .tab-btn.active {
  background: linear-gradient(135deg, #60a5fa 0%, #3b82f6 100%);
  box-shadow: 0 8px 18px rgba(59, 130, 246, 0.28);
}
</style>
