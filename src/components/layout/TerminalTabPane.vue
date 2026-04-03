<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import { Terminal } from '@xterm/xterm'
import type { TerminalTab } from '@/stores/terminal'
import { useTerminalStore } from '@/stores/terminal'

interface TerminalDataEvent {
  session_id: string
  data: string
}

const props = defineProps<{
  tab: TerminalTab
  active: boolean
}>()

const { t } = useI18n()
const terminalStore = useTerminalStore()
const containerRef = ref<HTMLElement | null>(null)
const inputBuffer = ref('')
const canSuggest = ref(true)
const suggestion = ref<string | null>(null)
const ghostSuffix = computed(() => {
  if (!suggestion.value || !inputBuffer.value) {
    return ''
  }

  return suggestion.value.slice(inputBuffer.value.length)
})

let xterm: Terminal | null = null
let fitAddon: FitAddon | null = null
let outputUnlisten: UnlistenFn | null = null
let resizeObserver: ResizeObserver | null = null
let flushTimer: ReturnType<typeof setTimeout> | null = null
let queuedWrite = ''
let writeChain = Promise.resolve()

function isBackspaceInput(data: string) {
  return data === '\u007F' || data === '\u0008'
}

function isEnterInput(data: string) {
  return data === '\r' || data === '\n'
}

function isPrintableInput(data: string) {
  return Array.from(data).every((character) => {
    const code = character.charCodeAt(0)
    return code >= 32 && code !== 127
  })
}

function resetSuggestionState() {
  inputBuffer.value = ''
  canSuggest.value = true
  suggestion.value = null
}

function refreshSuggestion() {
  if (!canSuggest.value) {
    suggestion.value = null
    return
  }

  suggestion.value = terminalStore.getCommandSuggestion(props.tab.projectId, inputBuffer.value)
}

async function writeToSession(data: string) {
  writeChain = writeChain
    .then(async () => {
      await invoke('terminal_write', {
        sessionId: props.tab.sessionId,
        data
      })
    })
    .catch(console.error)

  await writeChain
}

function flushQueuedWrite() {
  if (!queuedWrite) {
    return Promise.resolve()
  }

  const nextChunk = queuedWrite
  queuedWrite = ''
  if (flushTimer) {
    clearTimeout(flushTimer)
    flushTimer = null
  }

  return writeToSession(nextChunk)
}

function queueWrite(data: string, immediate = false) {
  queuedWrite += data

  if (immediate) {
    return flushQueuedWrite()
  }

  if (flushTimer) {
    return Promise.resolve()
  }

  return new Promise<void>((resolve) => {
    flushTimer = setTimeout(() => {
      flushTimer = null
      void flushQueuedWrite().finally(() => resolve())
    }, 12)
  })
}

async function fitTerminal() {
  if (!props.active || !xterm || !fitAddon || !containerRef.value) {
    return
  }

  fitAddon.fit()
  await invoke('terminal_resize', {
    sessionId: props.tab.sessionId,
    cols: xterm.cols,
    rows: xterm.rows
  }).catch(console.error)
}

async function handleTerminalInput(data: string) {
  if (data === '\t') {
    if (suggestion.value) {
      const suffix = suggestion.value.slice(inputBuffer.value.length)
      if (suffix) {
        inputBuffer.value = suggestion.value
        refreshSuggestion()
        await queueWrite(suffix, true)
      }
      return
    }

    await queueWrite(data, true)
    return
  }

  if (isEnterInput(data)) {
    terminalStore.rememberCommand(props.tab.projectId, inputBuffer.value)
    resetSuggestionState()
    await queueWrite('\r', true)
    return
  }

  if (data === '\u0003') {
    resetSuggestionState()
    await queueWrite(data, true)
    return
  }

  if (isBackspaceInput(data)) {
    if (canSuggest.value && inputBuffer.value.length > 0) {
      inputBuffer.value = inputBuffer.value.slice(0, -1)
      refreshSuggestion()
    }
    await queueWrite('\u007F', true)
    return
  }

  if (data.startsWith('\u001b')) {
    canSuggest.value = false
    suggestion.value = null
    await queueWrite(data, true)
    return
  }

  if (!isPrintableInput(data) || data.includes('\n')) {
    canSuggest.value = false
    suggestion.value = null
    await queueWrite(data, true)
    return
  }

  inputBuffer.value += data
  refreshSuggestion()
  await queueWrite(data)
}

function createTerminalTheme() {
  const styles = getComputedStyle(document.documentElement)

  return {
    background: styles.getPropertyValue('--terminal-surface-bg').trim() || '#0f172a',
    foreground: styles.getPropertyValue('--terminal-surface-text').trim() || '#e2e8f0',
    cursor: styles.getPropertyValue('--color-primary').trim() || '#60a5fa',
    selectionBackground: 'rgba(96, 165, 250, 0.26)',
    black: '#0f172a',
    brightBlack: '#475569',
    red: '#f87171',
    brightRed: '#fca5a5',
    green: '#4ade80',
    brightGreen: '#86efac',
    yellow: '#fbbf24',
    brightYellow: '#fde68a',
    blue: '#60a5fa',
    brightBlue: '#93c5fd',
    magenta: '#c084fc',
    brightMagenta: '#d8b4fe',
    cyan: '#22d3ee',
    brightCyan: '#67e8f9',
    white: '#e2e8f0',
    brightWhite: '#f8fafc'
  }
}

onMounted(async () => {
  if (!containerRef.value) {
    return
  }

  xterm = new Terminal({
    cursorBlink: true,
    fontFamily: 'Cascadia Code, SFMono-Regular, Consolas, monospace',
    fontSize: 13,
    lineHeight: 1.25,
    scrollback: 5000,
    convertEol: false,
    theme: createTerminalTheme()
  })

  // 打包态 WebView 对 Backspace/Tab 的默认行为不稳定，这里显式拦截，统一交给 xterm 处理。
  xterm.attachCustomKeyEventHandler((event) => {
    if (event.type !== 'keydown') {
      return true
    }

    if (event.key === 'Backspace' || event.key === 'Tab') {
      event.preventDefault()
      event.stopPropagation()
    }

    return true
  })

  fitAddon = new FitAddon()
  xterm.loadAddon(fitAddon)
  xterm.loadAddon(new WebLinksAddon())
  xterm.open(containerRef.value)
  xterm.onData((data) => {
    void handleTerminalInput(data)
  })

  outputUnlisten = await listen<TerminalDataEvent>('terminal:data', (event) => {
    if (event.payload.session_id !== props.tab.sessionId || !xterm) {
      return
    }

    xterm.write(event.payload.data)
  })

  resizeObserver = new ResizeObserver(() => {
    void fitTerminal()
  })
  resizeObserver.observe(containerRef.value)

  await nextTick()
  await fitTerminal()

  if (props.active) {
    xterm.focus()
  }
})

watch(() => props.active, async (active) => {
  if (!active || !xterm) {
    return
  }

  await nextTick()
  await fitTerminal()
  xterm.focus()
})

watch(() => props.tab.projectId, () => {
  refreshSuggestion()
})

onBeforeUnmount(() => {
  if (flushTimer) {
    clearTimeout(flushTimer)
    flushTimer = null
  }
  void flushQueuedWrite()
  resizeObserver?.disconnect()
  outputUnlisten?.()
  xterm?.dispose()
})
</script>

<template>
  <div class="terminal-tab-pane">
    <div
      ref="containerRef"
      class="terminal-tab-pane__viewport"
    />

    <div
      v-if="inputBuffer && ghostSuffix"
      class="terminal-tab-pane__ghost"
    >
      <span class="terminal-tab-pane__ghost-prefix">{{ inputBuffer }}</span>
      <span class="terminal-tab-pane__ghost-suffix">{{ ghostSuffix }}</span>
      <span class="terminal-tab-pane__ghost-hint">{{ t('terminal.suggestionHint') }}</span>
    </div>

    <div
      v-if="tab.status === 'closed'"
      class="terminal-tab-pane__overlay"
    >
      <div class="terminal-tab-pane__overlay-title">
        {{ t('terminal.closedTitle') }}
      </div>
      <div class="terminal-tab-pane__overlay-text">
        {{ t('terminal.closedHint') }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.terminal-tab-pane {
  position: relative;
  flex: 1;
  min-height: 0;
  border-radius: 16px;
  overflow: hidden;
  background: var(--terminal-surface-bg);
  border: 1px solid color-mix(in srgb, var(--terminal-surface-border) 88%, transparent);
  box-shadow: 0 20px 34px rgba(15, 23, 42, 0.12);
}

.terminal-tab-pane__viewport {
  width: 100%;
  height: 100%;
  padding: 10px 12px;
  box-sizing: border-box;
}

.terminal-tab-pane__ghost {
  position: absolute;
  left: 14px;
  right: 14px;
  bottom: 12px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border-radius: 999px;
  background: rgba(15, 23, 42, 0.72);
  backdrop-filter: blur(10px);
  color: rgba(226, 232, 240, 0.92);
  font-family: var(--font-family-mono);
  font-size: 11px;
  pointer-events: none;
}

.terminal-tab-pane__ghost-prefix {
  color: rgba(226, 232, 240, 0.96);
}

.terminal-tab-pane__ghost-suffix {
  color: rgba(148, 163, 184, 0.9);
}

.terminal-tab-pane__ghost-hint {
  margin-left: auto;
  color: rgba(148, 163, 184, 0.88);
  font-family: var(--font-family-sans);
}

.terminal-tab-pane__overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background: rgba(15, 23, 42, 0.68);
  color: #e2e8f0;
  text-align: center;
  backdrop-filter: blur(10px);
}

.terminal-tab-pane__overlay-title {
  font-size: 14px;
  font-weight: 700;
}

.terminal-tab-pane__overlay-text {
  max-width: 28rem;
  font-size: 12px;
  color: #cbd5e1;
}
</style>
