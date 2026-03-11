<script setup lang="ts">
import { EaButton } from '@/components/common'
import SettingsSectionCard from '@/components/settings/common/SettingsSectionCard.vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
</script>

<template>
  <SettingsSectionCard :title="t('settings.data.dataPath')">
    <div class="settings-item">
      <div class="settings-item__info">
        <span class="settings-item__label">{{ t('settings.data.dataLocation') }}</span>
        <span class="settings-item__value">~/.easy-agent</span>
      </div>
      <EaButton
        type="ghost"
        size="small"
      >
        {{ t('settings.data.change') }}
      </EaButton>
    </div>
  </SettingsSectionCard>
</template>

<style scoped>
.settings-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.settings-item__info {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
}

.settings-item__label {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.settings-item__value {
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  font-family: var(--font-family-mono);
}
</style>
