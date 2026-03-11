<script setup lang="ts">
import { computed } from 'vue'
import { EaButton } from '@/components/common'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  visible: boolean
  profileName?: string
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  confirm: []
}>()

const { t } = useI18n()

const dialogVisible = computed({
  get: () => props.visible,
  set: (value: boolean) => emit('update:visible', value)
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="dialogVisible"
      class="confirm-overlay"
      @click.self="dialogVisible = false"
    >
      <div class="confirm-dialog">
        <h3>{{ t('settings.providerSwitch.confirmDelete') }}</h3>
        <p>{{ t('settings.providerSwitch.confirmDeleteMessage', { name: profileName }) }}</p>
        <div class="confirm-actions">
          <EaButton @click="dialogVisible = false">
            {{ t('common.cancel') }}
          </EaButton>
          <EaButton
            type="danger"
            @click="emit('confirm')"
          >
            {{ t('common.delete') }}
          </EaButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.confirm-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.58);
  backdrop-filter: blur(4px);
}

.confirm-dialog {
  width: min(400px, 90%);
  padding: 24px;
  border-radius: 16px;
  background: var(--color-bg-primary, #ffffff);
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.2);
}

:global(.dark) .confirm-dialog {
  background: var(--color-bg-primary, #111827);
}

.confirm-dialog h3 {
  margin: 0 0 12px;
  color: var(--color-text-primary, #1a1a1a);
}

.confirm-dialog p {
  margin: 0 0 20px;
  color: var(--color-text-secondary, #666);
}

:global(.dark) .confirm-dialog h3 {
  color: var(--color-text-primary, #ffffff);
}

:global(.dark) .confirm-dialog p {
  color: var(--color-text-secondary, #cbd5e1);
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}
</style>
