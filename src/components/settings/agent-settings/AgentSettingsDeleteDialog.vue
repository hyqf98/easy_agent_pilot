<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon } from '@/components/common'

interface Props {
  visible: boolean
  agentName: string
}

const props = defineProps<Props>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  confirm: []
  cancel: []
}>()

const { t } = useI18n()

const dialogVisible = computed({
  get: () => props.visible,
  set: (value: boolean) => emit('update:visible', value)
})

function handleClose() {
  emit('update:visible', false)
  emit('cancel')
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="dialogVisible"
      class="modal-overlay"
      @click="handleClose"
    >
      <div
        class="confirm-dialog"
        @click.stop
      >
        <div class="confirm-dialog__content">
          <EaIcon
            name="alert-triangle"
            :size="24"
            class="confirm-dialog__icon"
          />
          <h4 class="confirm-dialog__title">
            {{ t('common.confirmDelete') }}
          </h4>
          <p class="confirm-dialog__message">
            {{ t('settings.agentList.confirmDeleteMessage', { name: agentName }) }}
          </p>
        </div>
        <div class="confirm-dialog__actions">
          <EaButton
            type="secondary"
            @click="handleClose"
          >
            {{ t('common.cancel') }}
          </EaButton>
          <EaButton
            type="danger"
            @click="emit('confirm')"
          >
            {{ t('common.confirmDelete') }}
          </EaButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
}

.confirm-dialog {
  width: 400px;
  max-width: 90vw;
  border-radius: var(--radius-xl);
  background-color: var(--color-surface);
  box-shadow: var(--shadow-2xl);
}

.confirm-dialog__content {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--spacing-6);
  text-align: center;
}

.confirm-dialog__icon {
  margin-bottom: var(--spacing-4);
  color: var(--color-warning);
}

.confirm-dialog__title {
  margin: 0 0 var(--spacing-2);
  color: var(--color-text-primary);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
}

.confirm-dialog__message {
  margin: 0;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.confirm-dialog__actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3);
  padding: var(--spacing-4) var(--spacing-6);
  border-top: 1px solid var(--color-border);
}
</style>
