<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaButton } from '@/components/common'

interface Props {
  visible: boolean
  name?: string
}

const props = defineProps<Props>()

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
      class="modal-overlay"
      @click.self="dialogVisible = false"
    >
      <div class="modal modal--small">
        <div class="modal__header">
          <h3 class="modal__title">
            {{ t('settings.cli.confirmDelete') }}
          </h3>
        </div>

        <div class="modal__body">
          <p>{{ t('settings.cli.confirmDeleteMessage', { name }) }}</p>
        </div>

        <div class="modal__footer">
          <EaButton
            type="secondary"
            @click="dialogVisible = false"
          >
            {{ t('common.cancel') }}
          </EaButton>
          <EaButton
            type="primary"
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
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 1060;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(0, 0, 0, 0.5);
}

.modal {
  width: 100%;
  max-width: 480px;
  margin: var(--spacing-4);
  border-radius: var(--radius-lg);
  background-color: var(--color-bg-primary);
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
}

.modal--small {
  max-width: 360px;
}

.modal__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4) var(--spacing-5);
  border-bottom: 1px solid var(--color-border);
}

.modal__title {
  margin: 0;
  color: var(--color-text-primary);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
}

.modal__body {
  padding: var(--spacing-5);
  color: var(--color-text-secondary);
}

.modal__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3);
  padding: var(--spacing-4) var(--spacing-5);
  border-top: 1px solid var(--color-border);
}
</style>
