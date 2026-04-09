<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaButton } from '@/components/common'
import { useMiniPanelShortcutState } from '@/composables/useMiniPanelShortcut'
import {
  buildShortcutFromKeyboardEvent,
  DEFAULT_MINI_PANEL_SHORTCUT,
  formatShortcutForDisplay,
  formatShortcutPreviewFromKeyboardEvent,
  IS_MAC,
  resolveMiniPanelShortcut,
  SUPPORTS_NATIVE_SHORTCUT_OVERRIDE,
  validateShortcutForCurrentPlatform
} from '@/utils/shortcut'

const props = defineProps<{
  modelValue: string
  disabled?: boolean
  windowsOverrideEnabled?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'update:windowsOverrideEnabled': [value: boolean]
}>()

const { t } = useI18n()
const { registrationState, registrationError, registrationMode } = useMiniPanelShortcutState()

const isRecording = ref(false)
const recorderRef = ref<HTMLButtonElement | null>(null)
const captureHint = ref('')
const recordingPreview = ref('')
const suppressNextToggleUntil = ref(0)
const nativeCaptureToken = ref(0)

const displayValue = computed(() => formatShortcutForDisplay(props.modelValue))
const recordingDisplayValue = computed(() => recordingPreview.value || t('settings.general.miniPanelShortcutRecording'))

const statusText = computed(() => {
  if (props.disabled) {
    return t('settings.general.miniPanelShortcutDisabled')
  }

  if (registrationState.value === 'registering') {
    return t('settings.general.miniPanelShortcutRegistering')
  }

  if (registrationState.value === 'error') {
    if (registrationError.value === 'GLOBAL_SHORTCUT_PERMISSION_REQUIRED') {
      return t('settings.general.miniPanelShortcutPermissionRequired')
    }

    if (registrationError.value === 'GLOBAL_SHORTCUT_RESERVED_WINDOWS_ALT_SPACE') {
      return t('settings.general.miniPanelShortcutReservedWindowsAltSpace')
    }

    if (registrationError.value === 'GLOBAL_SHORTCUT_CONFLICT') {
      return t('settings.general.miniPanelShortcutConflict')
    }

    if (registrationError.value === 'NATIVE_SHORTCUT_OVERRIDE_UNSUPPORTED') {
      return t('settings.general.miniPanelShortcutOverrideUnsupported')
    }

    if (registrationError.value === 'NATIVE_SHORTCUT_OVERRIDE_FAILED') {
      return t('settings.general.miniPanelShortcutOverrideFailed')
    }

    if (registrationError.value === 'NATIVE_SHORTCUT_OVERRIDE_PERMISSION_REQUIRED') {
      return t('settings.general.miniPanelShortcutOverridePermissionRequired')
    }

    return registrationError.value || t('settings.general.miniPanelShortcutConflict')
  }

  if (registrationState.value === 'registered') {
    if (registrationMode.value === 'windows-override') {
      return t('settings.general.miniPanelShortcutRegisteredOverride', {
        shortcut: formatShortcutForDisplay(resolveMiniPanelShortcut(props.modelValue))
      })
    }

    return t('settings.general.miniPanelShortcutRegistered', {
      shortcut: formatShortcutForDisplay(resolveMiniPanelShortcut(props.modelValue))
    })
  }

  return t('settings.general.miniPanelShortcutReady')
})

const statusClass = computed(() => ({
  'shortcut-status--error': registrationState.value === 'error' && !props.disabled,
  'shortcut-status--active': registrationState.value === 'registered' && !props.disabled,
  'shortcut-status--muted': props.disabled || registrationState.value === 'idle'
}))

const canEnableShortcutOverride = computed(() => (
  SUPPORTS_NATIVE_SHORTCUT_OVERRIDE
  && !props.disabled
  && !props.windowsOverrideEnabled
  && registrationState.value === 'error'
  && registrationError.value !== 'GLOBAL_SHORTCUT_PERMISSION_REQUIRED'
  && registrationError.value !== 'NATIVE_SHORTCUT_OVERRIDE_UNSUPPORTED'
  && registrationError.value !== 'NATIVE_SHORTCUT_OVERRIDE_FAILED'
  && registrationError.value !== 'NATIVE_SHORTCUT_OVERRIDE_PERMISSION_REQUIRED'
))

function stopRecording() {
  nativeCaptureToken.value += 1
  isRecording.value = false
  captureHint.value = ''
  recordingPreview.value = ''
}

function applyCapturedShortcut(shortcut: string) {
  const validationError = validateShortcutForCurrentPlatform(shortcut, {
    windowsOverrideEnabled: props.windowsOverrideEnabled
  })
  if (validationError === 'reserved-windows-alt-space') {
    captureHint.value = t('settings.general.miniPanelShortcutReservedWindowsAltSpace')
    return
  }

  emit('update:modelValue', shortcut)
  suppressNextToggleUntil.value = Date.now() + 160
  stopRecording()
}

async function startNativeCapture(captureToken: number) {
  try {
    const shortcut = await invoke<string>('capture_mini_panel_native_shortcut_once', {
      timeoutMs: 15000
    })

    if (!isRecording.value || captureToken !== nativeCaptureToken.value) {
      return
    }

    recordingPreview.value = formatShortcutForDisplay(shortcut)
    applyCapturedShortcut(shortcut)
  } catch (error) {
    if (!isRecording.value || captureToken !== nativeCaptureToken.value) {
      return
    }

    const message = error instanceof Error ? error.message : String(error)
    if (message.includes('MACOS_SHORTCUT_OVERRIDE_PERMISSION_REQUIRED')) {
      captureHint.value = t('settings.general.miniPanelShortcutOverridePermissionRequired')
      return
    }

    if (message.includes('MACOS_SHORTCUT_CAPTURE_CANCELLED')) {
      suppressNextToggleUntil.value = Date.now() + 160
      stopRecording()
      return
    }

    if (message.includes('MACOS_SHORTCUT_CAPTURE_TIMEOUT')) {
      captureHint.value = t('settings.general.miniPanelShortcutRecordingDesc')
      return
    }

    captureHint.value = t('settings.general.miniPanelShortcutUnsupported')
  }
}

async function startRecording() {
  if (props.disabled) {
    return
  }

  isRecording.value = true
  captureHint.value = t('settings.general.miniPanelShortcutRecordingDesc')
  recordingPreview.value = ''
  await nextTick()
  recorderRef.value?.focus()

  if (IS_MAC) {
    const captureToken = nativeCaptureToken.value + 1
    nativeCaptureToken.value = captureToken
    void startNativeCapture(captureToken)
  }
}

function toggleRecording() {
  if (Date.now() < suppressNextToggleUntil.value) {
    return
  }

  if (isRecording.value) {
    stopRecording()
    return
  }

  void startRecording()
}

function resetShortcut() {
  emit('update:modelValue', DEFAULT_MINI_PANEL_SHORTCUT)
  stopRecording()
}

function handleKeydown(event: KeyboardEvent) {
  if (!isRecording.value) {
    return
  }

  event.preventDefault()
  event.stopPropagation()
  event.stopImmediatePropagation()

  if (event.key === 'Escape') {
    suppressNextToggleUntil.value = Date.now() + 300
    stopRecording()
    return
  }

  recordingPreview.value = formatShortcutPreviewFromKeyboardEvent(event)

  const result = buildShortcutFromKeyboardEvent(event)
  if (result.accelerator) {
    suppressNextToggleUntil.value = Date.now() + 500
    applyCapturedShortcut(result.accelerator)
    return
  }

  if (result.error === 'modifier-only') {
    captureHint.value = t('settings.general.miniPanelShortcutModifierOnly')
    return
  }

  captureHint.value = t('settings.general.miniPanelShortcutUnsupported')
}

function handleKeyup(event: KeyboardEvent) {
  if (!isRecording.value || event.key === 'Escape') {
    return
  }

  event.preventDefault()
  event.stopPropagation()
  event.stopImmediatePropagation()
  recordingPreview.value = formatShortcutPreviewFromKeyboardEvent(event)
}

function handleButtonKeydown(event: KeyboardEvent) {
  if (!isRecording.value) {
    return
  }
  event.preventDefault()
  event.stopPropagation()
}

function handleButtonKeyup(event: KeyboardEvent) {
  if (!isRecording.value) {
    return
  }
  event.preventDefault()
  event.stopPropagation()
}

function handleButtonClick(event: MouseEvent) {
  if (isRecording.value) {
    event.preventDefault()
    return
  }
  toggleRecording()
}

function enableWindowsOverride() {
  emit('update:windowsOverrideEnabled', true)
}

watch(
  () => props.disabled,
  (disabled) => {
    if (disabled) {
      stopRecording()
    }
  }
)

onMounted(() => {
  window.addEventListener('blur', stopRecording)
  window.addEventListener('keydown', handleKeydown, true)
  window.addEventListener('keyup', handleKeyup, true)
})

onUnmounted(() => {
  window.removeEventListener('blur', stopRecording)
  window.removeEventListener('keydown', handleKeydown, true)
  window.removeEventListener('keyup', handleKeyup, true)
})
</script>

<template>
  <div class="shortcut-recorder">
    <div class="shortcut-recorder__controls">
      <button
        ref="recorderRef"
        type="button"
        class="shortcut-display"
        :class="{
          'shortcut-display--recording': isRecording,
          'shortcut-display--disabled': disabled
        }"
        :disabled="disabled"
        @click="handleButtonClick"
        @keydown="handleButtonKeydown"
        @keyup="handleButtonKeyup"
      >
        <span class="shortcut-display__value">
          {{ isRecording ? recordingDisplayValue : displayValue }}
        </span>
        <span
          v-if="isRecording"
          class="shortcut-display__pulse"
        />
      </button>

      <div class="shortcut-recorder__actions">
        <EaButton
          type="secondary"
          size="small"
          :disabled="disabled"
          @click="toggleRecording"
        >
          {{ isRecording ? t('settings.general.miniPanelShortcutCancel') : t('common.edit') }}
        </EaButton>
        <EaButton
          type="ghost"
          size="small"
          :disabled="disabled"
          @click="resetShortcut"
        >
          {{ t('settings.general.miniPanelShortcutReset') }}
        </EaButton>
      </div>
    </div>

    <p class="shortcut-recorder__hint">
      {{ isRecording ? captureHint : t('settings.general.miniPanelShortcutHint') }}
    </p>
    <p
      class="shortcut-status"
      :class="statusClass"
    >
      {{ statusText }}
    </p>
    <div
      v-if="canEnableShortcutOverride"
      class="shortcut-recorder__override"
    >
      <EaButton
        type="secondary"
        size="small"
        @click="enableWindowsOverride"
      >
        {{ t('settings.general.miniPanelShortcutEnableOverride') }}
      </EaButton>
      <p class="shortcut-recorder__override-text">
        {{ t('settings.general.miniPanelShortcutOverrideDesc') }}
      </p>
    </div>
  </div>
</template>

<style scoped>
.shortcut-recorder {
  display: flex;
  min-width: 320px;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--spacing-2);
}

.shortcut-recorder__controls {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
}

.shortcut-recorder__actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.shortcut-display {
  position: relative;
  display: inline-flex;
  min-width: 180px;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-4);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  color: var(--color-text-primary);
  transition: border-color var(--transition-fast) var(--easing-default),
              box-shadow var(--transition-fast) var(--easing-default),
              background-color var(--transition-fast) var(--easing-default);
}

.shortcut-display:hover:not(:disabled) {
  border-color: var(--color-border-dark);
  background: var(--color-surface-hover);
}

.shortcut-display--recording {
  border-color: var(--color-primary);
  background: var(--color-primary-light);
  box-shadow: 0 0 0 3px var(--color-primary-light);
}

.shortcut-display--disabled {
  opacity: 0.6;
}

.shortcut-display__value {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  letter-spacing: 0.02em;
}

.shortcut-display__pulse {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-primary);
  animation: shortcut-pulse 1s ease-in-out infinite;
}

.shortcut-recorder__hint,
.shortcut-status {
  max-width: 320px;
  font-size: var(--font-size-xs);
  line-height: 1.5;
  text-align: right;
}

.shortcut-recorder__hint {
  color: var(--color-text-tertiary);
}

.shortcut-status {
  color: var(--color-text-secondary);
}

.shortcut-status--active {
  color: var(--color-success, #15803d);
}

.shortcut-status--error {
  color: var(--color-error, #dc2626);
}

.shortcut-status--muted {
  color: var(--color-text-tertiary);
}

.shortcut-recorder__override {
  display: flex;
  max-width: 320px;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--spacing-2);
}

.shortcut-recorder__override-text {
  margin: 0;
  font-size: var(--font-size-xs);
  line-height: 1.5;
  text-align: right;
  color: var(--color-text-tertiary);
}

@keyframes shortcut-pulse {
  0%,
  100% {
    transform: scale(0.9);
    opacity: 0.6;
  }

  50% {
    transform: scale(1.15);
    opacity: 1;
  }
}
</style>
