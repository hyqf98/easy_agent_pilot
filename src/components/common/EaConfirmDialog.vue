<script setup lang="ts">
/**
 * EaConfirmDialog - 确认对话框组件
 * 用于危险操作的警告提示，需要用户确认
 */
import { ref, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import EaButton from './EaButton.vue'
import EaIcon from './EaIcon.vue'

export type ConfirmDialogType = 'warning' | 'danger' | 'info'

export interface EaConfirmDialogProps {
  visible?: boolean
  type?: ConfirmDialogType
  title?: string
  message?: string
  confirmLabel?: string
  cancelLabel?: string
  confirmButtonType?: 'primary' | 'danger'
}

const props = withDefaults(defineProps<EaConfirmDialogProps>(), {
  visible: false,
  type: 'warning',
  confirmButtonType: 'danger'
})

const emit = defineEmits<{
  'update:visible': [value: boolean]
  confirm: []
  cancel: []
}>()

const { t } = useI18n()

const iconMap: Record<ConfirmDialogType, string> = {
  warning: 'alert-triangle',
  danger: 'circle-alert',
  info: 'info'
}

const dialogRef = ref<HTMLElement | null>(null)
const cancelButtonRef = ref<InstanceType<typeof EaButton> | null>(null)

// Generate a unique ID for accessibility
const dialogId = `dialog-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`

const handleConfirm = () => {
  emit('confirm')
}

const handleCancel = () => {
  emit('cancel')
  emit('update:visible', false)
}

const handleOverlayClick = (e: MouseEvent) => {
  if (e.target === e.currentTarget) {
    handleCancel()
  }
}

const handleKeydown = (e: KeyboardEvent) => {
  if (!props.visible) return

  if (e.key === 'Escape') {
    handleCancel()
  } else if (e.key === 'Enter') {
    const activeElement = document.activeElement
    if (activeElement?.tagName !== 'BUTTON') {
      handleConfirm()
    }
  }
}

// Focus management - focus cancel button when dialog opens
watch(() => props.visible, async (visible) => {
  document.body.style.overflow = visible ? 'hidden' : ''

  if (visible) {
    await nextTick()
    // Focus the cancel button for safety (less destructive action)
    if (cancelButtonRef.value?.$el) {
      cancelButtonRef.value.$el.focus()
    }
  }
})

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="confirm-dialog-overlay"
      @click="handleOverlayClick"
    >
      <div
        ref="dialogRef"
        class="confirm-dialog"
        :class="`confirm-dialog--${type}`"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="`${dialogId}-title`"
      >
        <div class="confirm-dialog__header">
          <div class="confirm-dialog__icon-wrapper">
            <EaIcon
              :name="iconMap[type]"
              :size="24"
              class="confirm-dialog__icon"
              :class="`confirm-dialog__icon--${type}`"
            />
          </div>
          <h3
            :id="`${dialogId}-title`"
            class="confirm-dialog__title"
          >
            {{ title || t('common.confirm') }}
          </h3>
        </div>

        <div
          v-if="message"
          class="confirm-dialog__body"
        >
          <p class="confirm-dialog__message">
            {{ message }}
          </p>
        </div>

        <div class="confirm-dialog__actions">
          <EaButton
            ref="cancelButtonRef"
            type="secondary"
            @click="handleCancel"
          >
            {{ cancelLabel || t('common.cancel') }}
          </EaButton>
          <EaButton
            :type="confirmButtonType"
            @click="handleConfirm"
          >
            {{ confirmLabel || t('common.confirm') }}
          </EaButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.confirm-dialog-overlay {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--color-bg-overlay);
  backdrop-filter: blur(4px);
}

.confirm-dialog {
  display: flex;
  flex-direction: column;
  width: 90%;
  max-width: 420px;
  background-color: var(--color-surface);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-2xl);
  overflow: hidden;
}

.confirm-dialog--warning {
  border-top: 4px solid var(--color-warning);
}

.confirm-dialog--danger {
  border-top: 4px solid var(--color-error);
}

.confirm-dialog--info {
  border-top: 4px solid var(--color-info);
}

.confirm-dialog__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-5) var(--spacing-5) var(--spacing-3);
}

.confirm-dialog__icon-wrapper {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.confirm-dialog__icon {
  flex-shrink: 0;
}

.confirm-dialog__icon--warning {
  color: var(--color-warning);
}

.confirm-dialog__icon--danger {
  color: var(--color-error);
}

.confirm-dialog__icon--info {
  color: var(--color-info);
}

.confirm-dialog__title {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
  line-height: var(--line-height-tight);
}

.confirm-dialog__body {
  padding: 0 var(--spacing-5) var(--spacing-4);
}

.confirm-dialog__message {
  margin: 0;
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  line-height: var(--line-height-normal);
  white-space: pre-line;
}

.confirm-dialog__actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3);
  padding: var(--spacing-4) var(--spacing-5) var(--spacing-5);
  border-top: 1px solid var(--color-border-light);
}

/* Animations */
.confirm-dialog-overlay {
  animation: confirm-dialog-fade-in 160ms ease-out;
}

.confirm-dialog {
  animation: confirm-dialog-scale-in 180ms ease-out;
}

@keyframes confirm-dialog-fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes confirm-dialog-scale-in {
  from {
    opacity: 0;
    transform: scale(0.96);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
</style>
