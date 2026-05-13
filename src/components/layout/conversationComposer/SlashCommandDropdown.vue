<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import type { SlashCommandDescriptor, SlashCommandPanelType } from '@/services/slashCommands'

const props = defineProps<{
  visible: boolean
  position: { x: number; y: number; width: number; height: number }
  query: string
  commands: SlashCommandDescriptor[]
  panelType: SlashCommandPanelType
}>()

const emit = defineEmits<{
  select: [command: SlashCommandDescriptor]
  close: []
}>()

const { t } = useI18n()
const dropdownRef = ref<HTMLElement | null>(null)
const selectedIndex = ref(0)
const tipVisible = ref(false)
const tipTop = ref(0)
const tipLeft = ref(0)
let tipTimer: ReturnType<typeof setTimeout> | null = null

const selectedCommand = computed(() => props.commands[selectedIndex.value])

function showTip() {
  if (tipTimer) clearTimeout(tipTimer)
  tipVisible.value = true
  tipTimer = setTimeout(() => {
    tipVisible.value = false
  }, 2500)
}

function hideTip() {
  if (tipTimer) clearTimeout(tipTimer)
  tipTimer = null
  tipVisible.value = false
}

function updateTipPosition() {
  nextTick(() => {
    const selectedEl = dropdownRef.value?.querySelector('.slash-command__item--selected') as HTMLElement | null
    if (!selectedEl) {
      hideTip()
      return
    }
    const rect = selectedEl.getBoundingClientRect()
    tipTop.value = rect.top + rect.height / 2
    tipLeft.value = rect.right + 8
  })
}

function onSelectionChange() {
  showTip()
  updateTipPosition()
}

interface DisplayItem {
  type: 'command'
  command: SlashCommandDescriptor
  globalIndex: number
}

interface DisplayGroup {
  type: 'group'
  label: string
}

type DisplayEntry = DisplayItem | DisplayGroup

const displayEntries = computed(() => {
  const entries: DisplayEntry[] = []
  let globalIdx = 0

  const builtinCmds = props.commands.filter(c => c.source !== 'plugin')
  const pluginCmds = props.commands.filter(c => c.source === 'plugin')

  if (builtinCmds.length > 0) {
    entries.push({ type: 'group', label: t('message.slash.builtinGroup') })
    for (const cmd of builtinCmds) {
      entries.push({ type: 'command', command: cmd, globalIndex: globalIdx++ })
    }
  }

  if (pluginCmds.length > 0) {
    entries.push({ type: 'group', label: t('message.slash.pluginGroup') })
    for (const cmd of pluginCmds) {
      entries.push({ type: 'command', command: cmd, globalIndex: globalIdx++ })
    }
  }

  return entries
})

const dropdownStyle = computed(() => {
  if (!props.position.x || !props.position.y) return {}

  const dropdownHeight = 280
  const showAbove = window.innerHeight - props.position.y < dropdownHeight

  if (showAbove) {
    return {
      left: `${props.position.x}px`,
      bottom: `${window.innerHeight - props.position.y + 20}px`
    }
  }

  return {
    left: `${props.position.x}px`,
    top: `${props.position.y + 4}px`
  }
})

const emptyLabel = computed(() => {
  if (props.query.trim()) {
    return t('message.slash.noMatch')
  }

  return t('message.slash.hint')
})

function close() {
  emit('close')
}

function select(command: SlashCommandDescriptor) {
  emit('select', command)
}

function scrollToSelected() {
  nextTick(() => {
    const selectedEl = dropdownRef.value?.querySelector('.slash-command__item--selected')
    selectedEl?.scrollIntoView({ block: 'nearest' })
  })
}

function handleKeyDown(event: KeyboardEvent) {
  if (!props.visible) return

  switch (event.key) {
    case 'ArrowUp':
      event.preventDefault()
      event.stopPropagation()
      if (props.commands.length === 0) return
      selectedIndex.value = selectedIndex.value > 0 ? selectedIndex.value - 1 : props.commands.length - 1
      scrollToSelected()
      onSelectionChange()
      break
    case 'ArrowDown':
      event.preventDefault()
      event.stopPropagation()
      if (props.commands.length === 0) return
      selectedIndex.value = selectedIndex.value < props.commands.length - 1 ? selectedIndex.value + 1 : 0
      scrollToSelected()
      onSelectionChange()
      break
    case 'Enter': {
      const cmd = props.commands[selectedIndex.value]
      if (!cmd) return
      event.preventDefault()
      event.stopPropagation()
      select(cmd)
      break
    }
    case 'Escape':
      event.preventDefault()
      event.stopPropagation()
      close()
      break
  }
}

watch(() => props.commands, () => {
  selectedIndex.value = 0
  scrollToSelected()
}, { deep: true })

watch(() => props.visible, (v) => {
  if (v) {
    onSelectionChange()
  } else {
    hideTip()
  }
})

onMounted(() => {
  document.addEventListener('keydown', handleKeyDown, true)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeyDown, true)
  if (tipTimer) clearTimeout(tipTimer)
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      ref="dropdownRef"
      class="slash-command-dropdown"
      :style="dropdownStyle"
    >
      <div class="slash-command__header">
        <div class="slash-command__title">
          <EaIcon
            name="terminal-square"
            :size="14"
          />
          <span>{{ t('message.slash.title') }}</span>
        </div>
        <span class="slash-command__query">
          {{ query ? `/${query}` : '/' }}
        </span>
      </div>

      <div
        v-if="commands.length === 0"
        class="slash-command__empty"
      >
        <EaIcon
          name="search"
          :size="22"
        />
        <span>{{ emptyLabel }}</span>
      </div>

      <div
        v-else
        class="slash-command__list"
      >
        <template
          v-for="entry in displayEntries"
          :key="entry.type === 'group' ? entry.label : entry.command.name"
        >
          <div
            v-if="entry.type === 'group'"
            class="slash-command__group-label"
          >
            {{ entry.label }}
          </div>
          <button
            v-else
            class="slash-command__item"
            :class="{ 'slash-command__item--selected': entry.globalIndex === selectedIndex }"
            @mousemove="selectedIndex = entry.globalIndex; onSelectionChange()"
            @click="select(entry.command)"
          >
            <span class="slash-command__item-name">/{{ entry.command.name }}</span>
            <span
              v-if="entry.command.source === 'plugin' && entry.command.pluginName"
              class="slash-command__item-badge"
            >{{ entry.command.pluginName }}</span>
            <span class="slash-command__item-desc">{{ t(entry.command.descriptionKey) }}</span>
          </button>
        </template>
      </div>

      <Teleport to="body">
        <Transition name="slash-tip">
          <div
            v-if="tipVisible && selectedCommand"
            class="slash-command__tip"
            :style="{ top: tipTop + 'px', left: tipLeft + 'px' }"
          >
            <div class="slash-command__tip-label">
              {{ t(selectedCommand.descriptionKey) }}
            </div>
            <div class="slash-command__tip-usage">
              {{ t(selectedCommand.usageKey) }}
            </div>
          </div>
        </Transition>
      </Teleport>
    </div>
  </Teleport>
</template>

<style scoped>
.slash-command-dropdown {
  position: fixed;
  z-index: calc(var(--z-dropdown) + 2);
  width: min(380px, calc(100vw - 24px));
  border: 1px solid color-mix(in srgb, var(--color-border) 78%, rgba(15, 23, 42, 0.08));
  border-radius: 16px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.97), rgba(248, 250, 252, 0.95));
  box-shadow: 0 18px 34px rgba(15, 23, 42, 0.14);
  backdrop-filter: blur(16px);
  overflow: hidden;
}

.slash-command__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
  padding: 12px 14px 8px;
  border-bottom: 1px solid color-mix(in srgb, var(--color-border) 78%, transparent);
}

.slash-command__title {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.slash-command__query {
  padding: 2px 8px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-warning) 12%, transparent);
  color: color-mix(in srgb, var(--color-warning) 78%, var(--color-text-primary));
  font-family: var(--font-family-mono);
  font-size: 11px;
  font-weight: var(--font-weight-semibold);
}

.slash-command__list {
  display: flex;
  flex-direction: column;
  max-height: 236px;
  overflow-y: auto;
  padding: 6px;
}

.slash-command__group-label {
  padding: 8px 12px 4px;
  font-size: 10px;
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  user-select: none;
}

.slash-command__item {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 8px 10px;
  border-radius: 10px;
  text-align: left;
  border: 1px solid transparent;
  transition:
    background-color var(--transition-fast) var(--easing-default),
    border-color var(--transition-fast) var(--easing-default);
}

.slash-command__item--selected {
  background: color-mix(in srgb, var(--color-warning) 10%, transparent);
  border-color: color-mix(in srgb, var(--color-warning) 24%, transparent);
}

.slash-command__item-name {
  display: inline-flex;
  align-items: center;
  flex-shrink: 0;
  padding: 1px 8px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-warning) 12%, transparent);
  color: color-mix(in srgb, var(--color-warning) 76%, var(--color-text-primary));
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 11px;
  font-weight: 700;
  white-space: nowrap;
}

.slash-command__item-badge {
  flex-shrink: 0;
  padding: 1px 6px;
  border-radius: 999px;
  background: color-mix(in srgb, #10B981 14%, transparent);
  color: #10B981;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.04em;
  white-space: nowrap;
}

.slash-command__item-desc {
  flex: 1;
  min-width: 0;
  font-size: 11px;
  color: var(--color-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.slash-command__empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 104px;
  padding: 12px;
  font-size: 11px;
  color: var(--color-text-secondary);
}

:global([data-theme='dark']) .slash-command-dropdown,
:global(.dark) .slash-command-dropdown {
  background: linear-gradient(180deg, rgba(15, 23, 42, 0.98), rgba(30, 41, 59, 0.98));
  border-color: rgba(148, 163, 184, 0.22);
  box-shadow: 0 18px 36px rgba(2, 6, 23, 0.42);
}

:global([data-theme='dark']) .slash-command__title,
:global(.dark) .slash-command__title {
  color: #f8fafc;
}

:global([data-theme='dark']) .slash-command__query,
:global(.dark) .slash-command__query,
:global([data-theme='dark']) .slash-command__item-name,
:global(.dark) .slash-command__item-name {
  background: rgba(245, 158, 11, 0.16);
  color: #fbbf24;
}

:global([data-theme='dark']) .slash-command__item-badge,
:global(.dark) .slash-command__item-badge {
  background: rgba(16, 185, 129, 0.18);
  color: #34d399;
}

:global([data-theme='dark']) .slash-command__item--selected,
:global(.dark) .slash-command__item--selected {
  background: rgba(245, 158, 11, 0.09);
  border-color: rgba(245, 158, 11, 0.22);
}

.slash-command__tip {
  position: fixed;
  z-index: calc(var(--z-dropdown) + 3);
  max-width: 280px;
  padding: 8px 12px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-bg-elevated) 98%, rgba(15, 23, 42, 0.04));
  border: 1px solid color-mix(in srgb, var(--color-border) 60%, transparent);
  box-shadow: 0 8px 20px rgba(15, 23, 42, 0.12);
  backdrop-filter: blur(12px);
  transform: translateY(-50%);
  pointer-events: none;
}

.slash-command__tip-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 11px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin-bottom: 3px;
}

.slash-command__tip-usage {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 11px;
  line-height: 1.5;
  color: var(--color-text-secondary);
}

.slash-tip-enter-active {
  transition: opacity 0.15s var(--easing-default);
}

.slash-tip-leave-active {
  transition: opacity 0.2s var(--easing-default);
}

.slash-tip-enter-from,
.slash-tip-leave-to {
  opacity: 0;
}

:global([data-theme='dark']) .slash-command__tip,
:global(.dark) .slash-command__tip {
  background: rgba(30, 41, 59, 0.98);
  border-color: rgba(148, 163, 184, 0.2);
  box-shadow: 0 8px 20px rgba(2, 6, 23, 0.35);
}
</style>
