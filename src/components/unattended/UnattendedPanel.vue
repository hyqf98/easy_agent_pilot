<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import EaSelect, { type SelectOption } from '@/components/common/EaSelect.vue'
import { useConfirmDialog } from '@/composables'
import { inferAgentProvider, useAgentStore } from '@/stores/agent'
import { useAgentConfigStore } from '@/stores/agentConfig'
import type { UnattendedChannel, UnattendedEventRecord } from '@/services/unattended/types'
import { useProjectStore } from '@/stores/project'
import { useUnattendedStore } from '@/stores/unattended'

interface LogBubble {
  id: string
  side: 'left' | 'right'
  title: string
  text: string
  meta: string
  tone: 'inbound' | 'outbound'
}

const unattendedStore = useUnattendedStore()
const projectStore = useProjectStore()
const agentStore = useAgentStore()
const agentConfigStore = useAgentConfigStore()
const confirmDialog = useConfirmDialog()
const { t } = useI18n()
const modelOptionsByAgentId = ref<Record<string, SelectOption[]>>({})
const logViewportRef = ref<HTMLElement | null>(null)
const selectedChannelId = ref('')
const syncingChannelAgentIds = ref<Set<string>>(new Set())

const weixinChannels = computed(() =>
  unattendedStore.channels.filter(channel => channel.channelType === 'weixin')
)

const totalListeningAccounts = computed(() =>
  unattendedStore.accounts.filter(account => account.runtimeStatus === 'listening').length
)

const totalErrorAccounts = computed(() =>
  unattendedStore.accounts.filter(account => account.runtimeStatus === 'error').length
)

const systemRootPathHint = computed(() =>
  navigator.userAgent.includes('Windows') ? 'C:\\' : '/'
)

const projectOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('settings.unattended.projectRootOption', { path: systemRootPathHint.value }) },
  ...projectStore.projects.map(project => ({
    value: project.id,
    label: `${project.name} · ${project.path}`
  }))
])

const agentOptions = computed<SelectOption[]>(() =>
  agentStore.agents.map(agent => ({
    value: agent.id,
    label: `${agent.name} (${agent.type.toUpperCase()}${agent.provider ? ` / ${agent.provider}` : ''})`
  }))
)

const selectedChannel = computed(() =>
  weixinChannels.value.find(channel => channel.id === selectedChannelId.value) || weixinChannels.value[0]
)

const selectedChannelAccountIds = computed(() =>
  selectedChannel.value
    ? getChannelAccounts(selectedChannel.value.id).map(account => account.id)
    : []
)

const threadsSorted = computed(() =>
  [...unattendedStore.threads].sort((left, right) => {
    const leftTime = left.lastMessageAt || left.updatedAt
    const rightTime = right.lastMessageAt || right.updatedAt
    return new Date(rightTime).getTime() - new Date(leftTime).getTime()
  })
)

const selectedChannelThreads = computed(() =>
  selectedChannelAccountIds.value.length === 0
    ? threadsSorted.value
    : threadsSorted.value.filter(thread =>
      selectedChannelAccountIds.value.includes(thread.channelAccountId)
    )
)

const selectedThread = computed(() =>
  selectedChannelThreads.value[0]
)

const visibleEvents = computed(() => {
  const filtered = unattendedStore.events.filter(event => {
    if (event.direction !== 'inbound' && event.direction !== 'outbound') {
      return false
    }

    if (selectedChannelAccountIds.value.length === 0) {
      return true
    }

    const channelAccountId = event.channelAccountId
    return typeof channelAccountId === 'string'
      && selectedChannelAccountIds.value.includes(channelAccountId)
  })

  return [...filtered]
    .sort((left, right) => new Date(right.createdAt).getTime() - new Date(left.createdAt).getTime())
    .slice(0, 60)
})

const logBubbles = computed<LogBubble[]>(() =>
  visibleEvents.value.map(event => createLogBubble(event))
)

function parseEventPayload(event: UnattendedEventRecord): Record<string, unknown> | null {
  if (!event.payloadJson) {
    return null
  }

  try {
    return JSON.parse(event.payloadJson) as Record<string, unknown>
  } catch {
    return null
  }
}

function formatDateTime(value?: string): string {
  if (!value) {
    return '--'
  }

  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  return date.toLocaleString(undefined, {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

function normalizeBubbleText(value: string, fallback: string): string {
  const normalized = value.replace(/\r\n/g, '\n').trim()
  if (!normalized) {
    return fallback
  }
  return normalized
}

function createLogBubble(event: UnattendedEventRecord): LogBubble {
  const payload = parseEventPayload(event)
  const payloadText = typeof payload?.text === 'string' ? payload.text : ''
  const text = normalizeBubbleText(
    payloadText || event.summary || `${event.eventType} · ${event.status}`,
    t('settings.unattended.emptyLogText')
  )
  const time = formatDateTime(event.createdAt)

  if (event.direction === 'inbound') {
    return {
      id: event.id,
      side: 'left',
      title: t('settings.unattended.logInboundTitle'),
      text,
      meta: `${time} · ${resolveEventTypeLabel(event.eventType)}`,
      tone: 'inbound'
    }
  }

  if (event.direction === 'outbound') {
    return {
      id: event.id,
      side: 'right',
      title: t('settings.unattended.logOutboundTitle'),
      text,
      meta: `${time} · ${resolveEventTypeLabel(event.eventType)}`,
      tone: 'outbound'
    }
  }

  return {
    id: event.id,
    side: 'right',
    title: t('settings.unattended.logOutboundTitle'),
    text,
    meta: `${time} · ${resolveEventTypeLabel(event.eventType)}`,
    tone: 'outbound'
  }
}

function resolveEventTypeLabel(eventType: string): string {
  switch (eventType) {
    case 'inbound_message':
      return t('settings.unattended.eventInbound')
    case 'outbound_message':
      return t('settings.unattended.eventOutbound')
    default:
      return eventType
  }
}

function resolveProjectName(projectId?: string): string {
  if (!projectId) {
    return t('settings.unattended.systemRootPath', { path: systemRootPathHint.value })
  }
  return projectStore.projects.find(item => item.id === projectId)?.name || projectId
}

function resolveAgentName(agentId?: string): string {
  if (!agentId) {
    return t('settings.unattended.unbound')
  }
  return agentStore.agents.find(item => item.id === agentId)?.name || agentId
}

function resolveThreadAgentName(agentId?: string): string {
  if (!agentId) {
    return t('settings.unattended.followChannelAgent')
  }
  return resolveAgentName(agentId)
}

function getChannelAccounts(channelId: string) {
  return unattendedStore.accounts.filter(account => account.channelId === channelId)
}

function hasExpiredChannelToken(channelId: string): boolean {
  return getChannelAccounts(channelId).some(account => {
    const errorMessage = account.lastError || ''
    return account.runtimeStatus === 'error'
      && (
        errorMessage.includes('token')
        || errorMessage.includes('扫码')
        || errorMessage.includes('失效')
      )
  })
}

function resolveLoginState(channelId: string): string {
  const loginStatus = unattendedStore.loginSessions[channelId]?.status
  if (loginStatus) {
    return loginStatus
  }
  if (hasExpiredChannelToken(channelId)) {
    return 'expired'
  }
  return getChannelAccounts(channelId).length > 0 ? 'confirmed' : 'idle'
}

function resolveLoginStateLabel(channelId: string): string {
  switch (resolveLoginState(channelId)) {
    case 'waiting':
      return t('settings.unattended.loginWaiting')
    case 'scanned':
      return t('settings.unattended.loginScanned')
    case 'confirmed':
      return t('settings.unattended.loginConfirmed')
    case 'expired':
      return t('settings.unattended.loginExpired')
    case 'cancelled':
      return t('settings.unattended.loginCancelled')
    case 'error':
      return t('settings.unattended.loginError')
    default:
      return t('settings.unattended.loginIdle')
  }
}

function resolveRuntimeTone(channelId: string): 'listening' | 'warning' | 'idle' | 'error' {
  const channelAccounts = unattendedStore.accounts.filter(account => account.channelId === channelId)
  if (channelAccounts.some(account => account.runtimeStatus === 'error')) {
    return 'error'
  }
  if (channelAccounts.some(account => account.runtimeStatus === 'listening')) {
    return 'listening'
  }
  if (channelAccounts.length > 0) {
    return 'warning'
  }
  return 'idle'
}

function resolveRuntimeLabel(channelId: string): string {
  const channelAccounts = unattendedStore.accounts.filter(account => account.channelId === channelId)
  if (channelAccounts.some(account => account.runtimeStatus === 'error')) {
    return t('settings.unattended.runtimeError')
  }
  if (channelAccounts.some(account => account.runtimeStatus === 'listening')) {
    return t('settings.unattended.runtimeListening')
  }
  if (channelAccounts.length > 0) {
    return t('settings.unattended.runtimeReady')
  }
  return t('settings.unattended.runtimeIdle')
}

function hasActiveLoginSession(channelId: string): boolean {
  return Boolean(unattendedStore.loginSessions[channelId]?.qrcode)
}

function hasConnectedAccount(channelId: string): boolean {
  return getChannelAccounts(channelId).length > 0
}

function canStartRuntime(channelId: string): boolean {
  return hasConnectedAccount(channelId) && resolveRuntimeTone(channelId) !== 'listening'
}

function canStopRuntime(channelId: string): boolean {
  return hasActiveLoginSession(channelId) || resolveRuntimeTone(channelId) === 'listening'
}

function shouldShowQrCode(channelId: string): boolean {
  const loginSession = unattendedStore.loginSessions[channelId]
  if (!loginSession?.qrcodeImg) {
    return false
  }

  return loginSession.status === 'waiting' || loginSession.status === 'scanned'
}

function resolveChannelStatusTone(channelId: string): 'listening' | 'warning' | 'idle' | 'error' {
  const loginState = resolveLoginState(channelId)
  if (hasConnectedAccount(channelId) && !hasActiveLoginSession(channelId)) {
    return resolveRuntimeTone(channelId)
  }

  if (['expired', 'cancelled', 'error'].includes(loginState)) {
    return 'error'
  }
  if (['waiting', 'scanned'].includes(loginState)) {
    return 'warning'
  }
  if (resolveRuntimeTone(channelId) === 'error') {
    return 'error'
  }
  if (resolveRuntimeTone(channelId) === 'listening') {
    return 'listening'
  }
  if (loginState === 'confirmed') {
    return 'listening'
  }
  return 'idle'
}

function resolveChannelStatusLabel(channelId: string): string {
  const loginState = resolveLoginState(channelId)
  if (hasConnectedAccount(channelId) && !hasActiveLoginSession(channelId)) {
    return resolveRuntimeLabel(channelId)
  }

  if (['waiting', 'scanned', 'expired', 'cancelled', 'error'].includes(loginState)) {
    return resolveLoginStateLabel(channelId)
  }

  if (resolveRuntimeTone(channelId) === 'error') {
    return resolveRuntimeLabel(channelId)
  }

  return resolveRuntimeLabel(channelId)
}

function getModelOptions(agentId?: string): SelectOption[] {
  if (!agentId) {
    return [{ value: '', label: t('settings.unattended.followAgentModel') }]
  }
  return [
    { value: '', label: t('settings.unattended.followAgentModel') },
    ...(modelOptionsByAgentId.value[agentId] || [])
  ]
}

function resolvePreferredAgentId(agentId?: string): string {
  if (agentId && agentStore.agents.some(item => item.id === agentId)) {
    return agentId
  }
  return agentStore.agents[0]?.id || ''
}

function resolveSelectedAgentId(channel: UnattendedChannel): string {
  return resolvePreferredAgentId(channel.defaultAgentId)
}

async function resolvePreferredModelId(agentId?: string): Promise<string> {
  if (!agentId) {
    return ''
  }

  await ensureAgentModelOptions(agentId)
  const configuredModels = agentConfigStore.getModelsConfigs(agentId).filter(model => model.enabled)
  const preferredModel = configuredModels.find(model => model.isDefault) || configuredModels[0]
  return preferredModel?.modelId || ''
}

async function syncChannelDefaultAgents(): Promise<void> {
  const preferredAgentId = agentStore.agents[0]?.id
  if (!preferredAgentId) {
    return
  }

  const channelsNeedingSync = weixinChannels.value.filter(channel =>
    !channel.defaultAgentId
    || !agentStore.agents.some(item => item.id === channel.defaultAgentId)
  )

  for (const channel of channelsNeedingSync) {
    if (syncingChannelAgentIds.value.has(channel.id)) {
      continue
    }

    syncingChannelAgentIds.value = new Set(syncingChannelAgentIds.value).add(channel.id)
    try {
      const preferredModelId = await resolvePreferredModelId(preferredAgentId)
      await unattendedStore.updateChannel(channel.id, {
        defaultAgentId: preferredAgentId,
        defaultModelId: preferredModelId
      })
    } finally {
      const nextSyncingIds = new Set(syncingChannelAgentIds.value)
      nextSyncingIds.delete(channel.id)
      syncingChannelAgentIds.value = nextSyncingIds
    }
  }
}

async function ensureAgentModelOptions(agentId?: string): Promise<void> {
  if (!agentId || modelOptionsByAgentId.value[agentId]) {
    return
  }

  const agent = agentStore.agents.find(item => item.id === agentId)
  if (!agent) {
    return
  }

  const models = await agentConfigStore.ensureModelsConfigs(agentId, inferAgentProvider(agent))
  modelOptionsByAgentId.value = {
    ...modelOptionsByAgentId.value,
    [agentId]: models
      .filter(model => model.enabled)
      .map(model => ({
        value: model.modelId,
        label: `${model.displayName}${model.isDefault ? ` · ${t('settings.unattended.defaultBadge')}` : ''}`
      }))
  }
}

async function preloadChannelModels(): Promise<void> {
  await Promise.all(
    weixinChannels.value
      .map(channel => channel.defaultAgentId)
      .filter((agentId): agentId is string => Boolean(agentId))
      .map(agentId => ensureAgentModelOptions(agentId))
  )
}

/** 渠道配置需要先本地同步，避免监听线程在保存瞬间读到旧配置。 */
async function handleProjectChange(channelId: string, nextProjectId: string | number): Promise<void> {
  await unattendedStore.updateChannel(channelId, {
    defaultProjectId: String(nextProjectId)
  })
}

async function handleDeleteChannel(channel: UnattendedChannel): Promise<void> {
  const confirmed = await confirmDialog.show({
    type: 'warning',
    title: t('settings.unattended.deleteTitle'),
    message: t('settings.unattended.deleteMessage', { name: channel.name }),
    confirmLabel: t('common.delete'),
    cancelLabel: t('common.cancel'),
    confirmButtonType: 'danger'
  })

  if (!confirmed) {
    return
  }

  await unattendedStore.deleteChannel(channel.id)
}

async function handleAgentChange(channel: UnattendedChannel, nextAgentId: string | number): Promise<void> {
  const agentId = String(nextAgentId)
  const nextModelId = await resolvePreferredModelId(agentId)
  await unattendedStore.updateChannel(channel.id, {
    defaultAgentId: agentId,
    defaultModelId: nextModelId
  })
}

async function handleModelChange(channelId: string, nextModelId: string | number): Promise<void> {
  await unattendedStore.updateChannel(channelId, {
    defaultModelId: String(nextModelId)
  })
}

async function handleRefreshLogin(channelId: string): Promise<void> {
  if (!unattendedStore.loginSessions[channelId]?.qrcode) {
    await unattendedStore.startWeixinLogin(channelId)
    return
  }
  await unattendedStore.pollWeixinLogin(channelId)
}

async function scrollLogsToLatest(): Promise<void> {
  await nextTick()
  const viewport = logViewportRef.value
  if (!viewport) {
    return
  }
  viewport.scrollTop = 0
}

onMounted(async () => {
  if (projectStore.projects.length === 0) {
    await projectStore.loadProjects()
  }
  if (agentStore.agents.length === 0) {
    await agentStore.loadAgents()
  }
  await unattendedStore.initialize()
  await preloadChannelModels()
  await syncChannelDefaultAgents()
  await scrollLogsToLatest()
})

watch(
  () => weixinChannels.value.map(channel => channel.id).join('|'),
  () => {
    if (selectedChannel.value) {
      selectedChannelId.value = selectedChannel.value.id
      return
    }
    selectedChannelId.value = weixinChannels.value[0]?.id || ''
  },
  { immediate: true }
)

watch(
  () => weixinChannels.value.map(channel => `${channel.id}:${channel.defaultAgentId || ''}`),
  () => {
    void preloadChannelModels()
    void syncChannelDefaultAgents()
  }
)

watch(
  () => agentStore.agents.map(agent => agent.id).join('|'),
  () => {
    void syncChannelDefaultAgents()
  }
)

watch(
  () => logBubbles.value.map(bubble => bubble.id).join('|'),
  () => {
    void scrollLogsToLatest()
  },
  { flush: 'post', immediate: true }
)
</script>

<template>
  <div class="unattended-panel">
    <section class="hero">
      <div class="hero__copy">
        <p class="hero__eyebrow">
          {{ t('settings.unattended.heroEyebrow') }}
        </p>
        <h2 class="hero__title">
          {{ t('settings.unattended.heroTitle') }}
        </h2>
        <p class="hero__subtitle">
          {{ t('settings.unattended.heroSubtitle') }}
        </p>
      </div>

      <div class="hero__metrics">
        <div class="metric-card metric-card--signal">
          <span class="metric-card__label">{{ t('settings.unattended.metricChannels') }}</span>
          <strong class="metric-card__value">{{ weixinChannels.length }}</strong>
          <span class="metric-card__hint">{{ t('settings.unattended.metricChannelsHint') }}</span>
        </div>
        <div class="metric-card metric-card--active">
          <span class="metric-card__label">{{ t('settings.unattended.metricListening') }}</span>
          <strong class="metric-card__value">{{ totalListeningAccounts }}</strong>
          <span class="metric-card__hint">{{ t('settings.unattended.metricListeningHint') }}</span>
        </div>
        <div class="metric-card metric-card--risk">
          <span class="metric-card__label">{{ t('settings.unattended.metricErrors') }}</span>
          <strong class="metric-card__value">{{ totalErrorAccounts }}</strong>
          <span class="metric-card__hint">{{ t('settings.unattended.metricErrorsHint') }}</span>
        </div>
      </div>

      <button
        class="hero__action"
        @click="unattendedStore.createWeixinChannel()"
      >
        {{ t('settings.unattended.createChannel') }}
      </button>
    </section>

    <section class="channels-section">
      <div class="section-heading">
        <div>
          <p class="section-heading__eyebrow">
            {{ t('settings.unattended.channelEyebrow') }}
          </p>
          <h3>{{ t('settings.unattended.channelsTitle') }}</h3>
        </div>
        <span class="section-heading__meta">{{ t('settings.unattended.channelCount', { count: weixinChannels.length }) }}</span>
      </div>

      <div
        v-if="weixinChannels.length === 0"
        class="empty-state"
      >
        {{ t('settings.unattended.emptyChannels') }}
      </div>

      <div class="channel-grid">
        <article
          v-for="channel in weixinChannels"
          :key="channel.id"
          class="channel-card"
        >
          <div class="channel-card__head">
            <div>
              <div class="channel-card__title-row">
                <h4>{{ channel.name }}</h4>
                <span
                  class="status-pill"
                  :class="`status-pill--${resolveChannelStatusTone(channel.id)}`"
                >
                  {{ resolveChannelStatusLabel(channel.id) }}
                </span>
              </div>
              <p class="channel-card__summary">
                {{ t('settings.unattended.channelSummary', {
                  project: resolveProjectName(channel.defaultProjectId),
                  agent: resolveAgentName(resolveSelectedAgentId(channel) || undefined)
                }) }}
              </p>
            </div>

            <div class="channel-card__actions">
              <button
                class="action-btn action-btn--primary"
                @click="unattendedStore.startWeixinLogin(channel.id)"
              >
                {{ t('settings.unattended.actionScan') }}
              </button>
              <button
                class="action-btn action-btn--ghost"
                @click="handleRefreshLogin(channel.id)"
              >
                {{ t('settings.unattended.actionRefresh') }}
              </button>
              <button
                class="action-btn action-btn--soft"
                :disabled="!canStartRuntime(channel.id)"
                @click="unattendedStore.startRuntime(channel.id)"
              >
                {{ t('settings.unattended.actionStart') }}
              </button>
              <button
                class="action-btn action-btn--warning"
                :disabled="!canStopRuntime(channel.id)"
                @click="unattendedStore.stopRuntime(channel.id)"
              >
                {{ t('settings.unattended.actionStop') }}
              </button>
              <button
                class="action-btn action-btn--danger"
                @click="handleDeleteChannel(channel)"
              >
                {{ t('common.delete') }}
              </button>
            </div>
          </div>

          <div class="channel-card__body">
            <div class="channel-form">
              <label class="control-card">
                <span class="control-card__label">{{ t('settings.unattended.defaultProject') }}</span>
                <EaSelect
                  :model-value="channel.defaultProjectId || ''"
                  :options="projectOptions"
                  @update:model-value="handleProjectChange(channel.id, $event)"
                />
                <small class="control-card__hint">
                  {{ t('settings.unattended.projectHint', { path: systemRootPathHint }) }}
                </small>
              </label>

              <label class="control-card">
                <span class="control-card__label">{{ t('settings.unattended.defaultAgent') }}</span>
                <EaSelect
                  :model-value="resolveSelectedAgentId(channel)"
                  :options="agentOptions"
                  :disabled="agentOptions.length === 0"
                  @update:model-value="handleAgentChange(channel, $event)"
                />
                <small class="control-card__hint">
                  {{ t('settings.unattended.agentHint') }}
                </small>
              </label>

              <label class="control-card">
                <span class="control-card__label">{{ t('settings.unattended.defaultModel') }}</span>
                <EaSelect
                  :model-value="channel.defaultModelId || ''"
                  :options="getModelOptions(resolveSelectedAgentId(channel))"
                  :disabled="!resolveSelectedAgentId(channel)"
                  @update:model-value="handleModelChange(channel.id, $event)"
                />
                <small class="control-card__hint">
                  {{ t('settings.unattended.modelHint') }}
                </small>
              </label>
            </div>
          </div>

          <div
            v-if="shouldShowQrCode(channel.id)"
            class="channel-card__qr"
          >
            <img
              :src="unattendedStore.loginSessions[channel.id].qrcodeImg"
              :alt="t('settings.unattended.qrCodeAlt')"
            >
            <div class="channel-card__qr-copy">
              <span class="status-pill status-pill--idle">{{ t('settings.unattended.qrHintBadge') }}</span>
              <p>{{ t('settings.unattended.qrHintText') }}</p>
            </div>
          </div>
        </article>
      </div>
    </section>

    <section class="log-panel">
      <div class="section-heading section-heading--compact">
        <div>
          <p class="section-heading__eyebrow">
            {{ t('settings.unattended.logEyebrow') }}
          </p>
          <h3>{{ t('settings.unattended.logsTitle') }}</h3>
        </div>
        <span class="section-heading__meta">
          {{ selectedThread ? t('settings.unattended.lastActive', { time: formatDateTime(selectedThread.lastMessageAt || selectedThread.updatedAt) }) : t('settings.unattended.allThreads') }}
        </span>
      </div>

      <div
        v-if="weixinChannels.length > 1"
        class="log-channel-switcher"
      >
        <span class="log-channel-switcher__label">{{ t('settings.unattended.logSwitchLabel') }}</span>
        <div class="log-channel-switcher__list">
          <button
            v-for="channel in weixinChannels"
            :key="channel.id"
            type="button"
            class="log-channel-chip"
            :class="{ 'log-channel-chip--active': selectedChannel?.id === channel.id }"
            @click="selectedChannelId = channel.id"
          >
            {{ channel.name }}
          </button>
        </div>
      </div>

      <div
        v-if="selectedThread"
        class="log-panel__thread-summary"
      >
        <div>
          <strong>{{ selectedThread.peerNameSnapshot || selectedThread.peerId }}</strong>
          <p>{{ t('settings.unattended.currentAgent', { agent: resolveThreadAgentName(selectedThread.activeAgentId) }) }}</p>
        </div>
        <div>
          <strong>{{ selectedThread.activeModelId || t('settings.unattended.defaultModelShort') }}</strong>
          <p>{{ t('settings.unattended.lastActive', { time: formatDateTime(selectedThread.lastMessageAt || selectedThread.updatedAt) }) }}</p>
        </div>
      </div>

      <div
        v-if="logBubbles.length === 0"
        class="empty-state empty-state--chat"
      >
        {{ t('settings.unattended.emptyLogs') }}
      </div>

      <div
        v-else
        ref="logViewportRef"
        class="chat-log"
      >
        <div
          v-for="bubble in logBubbles"
          :key="bubble.id"
          class="chat-bubble"
          :class="[
            `chat-bubble--${bubble.side}`,
            `chat-bubble--${bubble.tone}`
          ]"
        >
          <span class="chat-bubble__title">{{ bubble.title }}</span>
          <p class="chat-bubble__text">
            {{ bubble.text }}
          </p>
          <span class="chat-bubble__meta">{{ bubble.meta }}</span>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.unattended-panel {
  --panel-bg: var(--color-surface);
  --panel-strong: var(--color-surface-elevated);
  --panel-card: rgba(255, 255, 255, 0.92);
  --panel-muted: rgba(255, 255, 255, 0.68);
  --panel-line: rgba(148, 163, 184, 0.24);
  --panel-shadow: 0 24px 60px rgba(18, 33, 58, 0.14);
  --panel-glow: rgba(42, 127, 255, 0.18);
  --tone-inbound: linear-gradient(180deg, rgba(236, 247, 255, 0.98), rgba(219, 239, 255, 0.92));
  --tone-outbound: linear-gradient(135deg, #113d3a, #0e5a76);
  --tone-outbound-text: #effffc;
  --tone-system: linear-gradient(180deg, rgba(245, 240, 231, 0.98), rgba(252, 247, 238, 0.94));
  --tone-error: linear-gradient(180deg, rgba(255, 240, 237, 0.98), rgba(255, 229, 223, 0.94));
  --scrollbar-thumb: rgba(60, 96, 168, 0.28);
  display: flex;
  flex-direction: column;
  gap: 18px;
  width: 100%;
  min-height: max-content;
  padding: 18px 20px 32px;
  box-sizing: border-box;
  background:
    radial-gradient(circle at top left, rgba(33, 150, 243, 0.08), transparent 28%),
    radial-gradient(circle at right center, rgba(16, 185, 129, 0.06), transparent 24%),
    linear-gradient(180deg, rgba(255, 246, 228, 0.18), transparent 38%),
    var(--panel-bg);
}

:global([data-theme='dark']) .unattended-panel,
:global(.dark) .unattended-panel {
  --panel-bg: #09111f;
  --panel-strong: rgba(12, 19, 35, 0.9);
  --panel-card: rgba(15, 23, 42, 0.76);
  --panel-muted: rgba(15, 23, 42, 0.58);
  --panel-line: rgba(148, 163, 184, 0.2);
  --panel-shadow: 0 24px 60px rgba(2, 6, 23, 0.4);
  --panel-glow: rgba(56, 189, 248, 0.16);
  --tone-inbound: linear-gradient(180deg, rgba(18, 40, 63, 0.94), rgba(15, 23, 42, 0.94));
  --tone-outbound: linear-gradient(135deg, rgba(14, 116, 144, 0.92), rgba(8, 78, 112, 0.94));
  --tone-outbound-text: #ecfeff;
  --tone-system: linear-gradient(180deg, rgba(51, 65, 85, 0.84), rgba(30, 41, 59, 0.9));
  --tone-error: linear-gradient(180deg, rgba(84, 28, 28, 0.88), rgba(69, 10, 10, 0.92));
  --scrollbar-thumb: rgba(125, 211, 252, 0.28);
  background:
    radial-gradient(circle at top left, rgba(14, 165, 233, 0.14), transparent 30%),
    radial-gradient(circle at right center, rgba(16, 185, 129, 0.1), transparent 24%),
    linear-gradient(180deg, rgba(15, 23, 42, 0.6), transparent 34%),
    var(--panel-bg);
}

.hero,
.channels-section,
.log-panel {
  position: relative;
  flex-shrink: 0;
  border: 1px solid var(--panel-line);
  border-radius: 26px;
  background: var(--panel-strong);
  box-shadow: var(--panel-shadow);
}

.log-panel {
  overflow: hidden;
}

.hero::before,
.channels-section::before,
.log-panel::before {
  content: '';
  position: absolute;
  inset: 0 auto auto 0;
  width: 180px;
  height: 180px;
  background: radial-gradient(circle, var(--panel-glow), transparent 72%);
  pointer-events: none;
}

.hero {
  display: grid;
  grid-template-columns: minmax(0, 1.15fr) minmax(320px, 0.85fr) auto;
  gap: 18px;
  align-items: end;
  padding: 22px;
  min-height: 156px;
}

.hero__eyebrow,
.section-heading__eyebrow {
  margin: 0 0 8px;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.22em;
  text-transform: uppercase;
  color: color-mix(in srgb, var(--color-text-secondary) 72%, #315cbb 28%);
}

.hero__title {
  margin: 0;
  max-width: 14ch;
  font-size: clamp(24px, 3.1vw, 36px);
  line-height: 1.08;
  letter-spacing: -0.04em;
  color: var(--color-text-primary);
}

.hero__subtitle {
  margin: 10px 0 0;
  max-width: 72ch;
  font-size: 12px;
  line-height: 1.55;
  color: var(--color-text-secondary);
}

.hero__metrics {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.metric-card {
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 12px;
  min-height: 104px;
  border: 1px solid var(--panel-line);
  border-radius: 20px;
  background: var(--color-surface);
}

.metric-card--signal {
  background: linear-gradient(180deg, rgba(232, 244, 255, 0.92), rgba(255, 255, 255, 0.46));
}

.metric-card--active {
  background: linear-gradient(180deg, rgba(227, 255, 246, 0.94), rgba(255, 255, 255, 0.5));
}

.metric-card--risk {
  background: linear-gradient(180deg, rgba(255, 239, 232, 0.94), rgba(255, 255, 255, 0.5));
}

.metric-card__label {
  font-size: 11px;
  color: var(--color-text-secondary);
}

.metric-card__value {
  font-size: 26px;
  line-height: 1;
  letter-spacing: -0.04em;
  color: var(--color-text-primary);
}

.metric-card__hint {
  font-size: 11px;
  line-height: 1.45;
  color: var(--color-text-tertiary);
}

.hero__action,
.action-btn {
  transition: transform 180ms ease, box-shadow 180ms ease, border-color 180ms ease, background 180ms ease;
}

.hero__action {
  align-self: start;
  justify-self: end;
  border: none;
  border-radius: 999px;
  padding: 12px 16px;
  background: linear-gradient(135deg, #1345ff, #0b8d7f);
  color: #fff;
  font-size: 12px;
  font-weight: 700;
  box-shadow: 0 18px 38px rgba(19, 69, 255, 0.22);
}

.hero__action:hover,
.action-btn:hover {
  transform: translateY(-1px);
}

.channels-section {
  padding: 18px;
  height: auto;
  min-height: 308px;
}

.section-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 14px;
}

.section-heading--compact {
  margin-bottom: 16px;
}

.section-heading h3 {
  margin: 0;
  font-size: 20px;
  letter-spacing: -0.03em;
}

.section-heading__meta {
  display: inline-flex;
  align-items: center;
  padding: 7px 11px;
  border: 1px solid var(--panel-line);
  border-radius: 999px;
  font-size: 11px;
  color: var(--color-text-secondary);
  background: var(--panel-muted);
}

.channel-grid {
  display: grid;
  gap: 14px;
  align-content: start;
}

.channel-card {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px;
  border: 1px solid var(--panel-line);
  border-radius: 20px;
  background: linear-gradient(140deg, var(--panel-card), rgba(248, 244, 235, 0.86));
}

.channel-card__head,
.channel-card__body,
.channel-card__foot,
.channel-card__qr {
  display: flex;
  justify-content: space-between;
  gap: 14px;
}

.channel-card__title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.channel-card h4 {
  margin: 0;
  font-size: 18px;
  letter-spacing: -0.03em;
}

.channel-card__summary {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--color-text-secondary);
}

.channel-card__actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  align-content: flex-start;
  gap: 8px;
}

.action-btn {
  border: 1px solid transparent;
  border-radius: 999px;
  padding: 8px 10px;
  font-size: 10.5px;
  font-weight: 700;
}

.action-btn:disabled {
  cursor: not-allowed;
  opacity: 0.52;
  transform: none;
  box-shadow: none;
}

.action-btn--primary {
  background: linear-gradient(135deg, #18a249, #0a6d88);
  color: #fff;
}

.action-btn--ghost {
  background: var(--panel-muted);
  border-color: var(--panel-line);
  color: var(--color-text-primary);
}

.action-btn--soft {
  background: rgba(19, 69, 255, 0.1);
  color: #2046ba;
}

.action-btn--warning {
  background: rgba(255, 173, 51, 0.14);
  color: #9f5a00;
}

.action-btn--danger {
  background: rgba(214, 63, 33, 0.12);
  color: #b43d20;
}

.channel-card__body {
  display: block;
}

.channel-form {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
  min-width: 0;
}

.log-channel-switcher {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.log-channel-switcher__label {
  flex: 0 0 auto;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-text-tertiary);
}

.log-channel-switcher__list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.log-channel-chip {
  border: 1px solid var(--panel-line);
  border-radius: 999px;
  padding: 7px 11px;
  background: var(--panel-card);
  color: var(--color-text-secondary);
  font-size: 11px;
  font-weight: 600;
  transition: all 180ms ease;
}

.log-channel-chip:hover {
  border-color: color-mix(in srgb, var(--panel-line) 40%, #2563eb 60%);
  color: var(--color-text-primary);
}

.log-channel-chip--active {
  border-color: transparent;
  background: linear-gradient(135deg, #1345ff, #0b8d7f);
  color: #fff;
  box-shadow: 0 12px 24px rgba(19, 69, 255, 0.18);
}

.control-card,
.state-card {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
  padding: 12px;
  border: 1px solid var(--panel-line);
  border-radius: 18px;
  background: var(--panel-card);
  overflow: hidden;
}

.control-card__label,
.state-card__label {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-text-tertiary);
}

.control-card__hint {
  font-size: 10.5px;
  line-height: 1.4;
  color: var(--color-text-tertiary);
}

.channel-side {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
  min-width: 0;
  width: min(100%, 300px);
  justify-self: end;
}

.control-card :deep(.ea-select),
.control-card :deep(.ea-select--medium),
.control-card :deep(.ea-select--small) {
  min-width: 0;
  width: 100%;
  max-width: 100%;
}

.control-card :deep(.ea-select__trigger),
.control-card :deep(.ea-select__label) {
  min-width: 0;
}

.state-card strong {
  font-size: 15px;
  letter-spacing: -0.02em;
}

.channel-card__qr {
  align-items: center;
  justify-content: flex-start;
  padding: 12px;
  border-radius: 20px;
  border: 1px dashed var(--panel-line);
  background: var(--panel-card);
}

.channel-card__qr img {
  width: 148px;
  height: 148px;
  border-radius: 18px;
  background: #fff;
  padding: 8px;
  box-shadow: 0 18px 36px rgba(18, 34, 62, 0.12);
}

.channel-card__qr-copy {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-width: 280px;
}

.channel-card__qr-copy p {
  margin: 0;
  font-size: 12px;
  line-height: 1.55;
  color: var(--color-text-secondary);
}

.channel-card__foot {
  flex-wrap: wrap;
  align-items: center;
}

.account-chip {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid var(--panel-line);
  border-radius: 999px;
  background: var(--panel-card);
}

.account-chip__id {
  max-width: 340px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
  color: var(--color-text-secondary);
}

.channel-card__foot-empty {
  font-size: 13px;
  color: var(--color-text-tertiary);
}

.status-pill {
  display: inline-flex;
  align-items: center;
  padding: 5px 9px;
  border-radius: 999px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
}

.status-pill--listening {
  background: rgba(24, 162, 73, 0.14);
  color: #14753b;
}

.status-pill--warning {
  background: rgba(255, 186, 81, 0.2);
  color: #9d6100;
}

.status-pill--idle {
  background: rgba(19, 69, 255, 0.1);
  color: #234ac4;
}

.status-pill--error {
  background: rgba(214, 63, 33, 0.13);
  color: #af3318;
}

.log-panel {
  padding: 18px;
}

.log-panel__thread-summary {
  display: flex;
  justify-content: space-between;
  gap: 12px;
}

.log-panel {
  display: block;
}

.log-panel__thread-summary {
  align-items: center;
  padding: 10px 12px;
  margin-bottom: 12px;
  border: 1px solid var(--panel-line);
  border-radius: 18px;
  background: var(--panel-card);
}

.log-panel__thread-summary strong {
  display: block;
  margin-bottom: 3px;
  font-size: 13px;
}

.log-panel__thread-summary p {
  margin: 0;
  font-size: 10px;
  color: var(--color-text-secondary);
}

.chat-log {
  display: flex;
  flex-direction: column;
  gap: 8px;
  height: clamp(620px, 70vh, 920px);
  padding: 2px 6px 2px 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  scroll-behavior: smooth;
}

.chat-log::-webkit-scrollbar {
  width: 10px;
}

.chat-log::-webkit-scrollbar-thumb {
  border: 2px solid transparent;
  border-radius: 999px;
  background: var(--scrollbar-thumb);
  background-clip: padding-box;
}

.chat-bubble {
  display: flex;
  flex-direction: column;
  gap: 5px;
  width: min(66%, 640px);
  max-width: 100%;
  padding: 10px 12px;
  border-radius: 16px;
  border: 1px solid var(--panel-line);
  box-shadow: 0 14px 30px rgba(20, 26, 38, 0.08);
  box-sizing: border-box;
}

.chat-bubble--left {
  align-self: flex-start;
  background: var(--tone-inbound);
  border-top-left-radius: 8px;
}

.chat-bubble--right {
  align-self: flex-end;
  color: var(--tone-outbound-text);
  background: var(--tone-outbound);
  border-top-right-radius: 8px;
}

.chat-bubble--center {
  align-self: center;
  max-width: 560px;
  background: var(--tone-system);
  text-align: center;
}

.chat-bubble--error {
  background: var(--tone-error);
}

.chat-bubble__title {
  font-size: 9.5px;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  opacity: 0.78;
}

.chat-bubble__text {
  margin: 0;
  font-size: 11px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}

.chat-bubble__meta {
  font-size: 9.5px;
  opacity: 0.72;
}

.empty-state {
  padding: 24px;
  border: 1px dashed var(--panel-line);
  border-radius: 22px;
  text-align: center;
  line-height: 1.7;
  color: var(--color-text-secondary);
  background: var(--panel-card);
}

.empty-state--compact {
  margin-top: 12px;
}

.empty-state--chat {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 440px;
}

:global([data-theme='dark']) .metric-card,
:global(.dark) .metric-card,
:global([data-theme='dark']) .channel-card,
:global(.dark) .channel-card,
:global([data-theme='dark']) .control-card,
:global(.dark) .control-card,
:global([data-theme='dark']) .state-card,
:global(.dark) .state-card,
:global([data-theme='dark']) .log-panel__thread-summary,
:global(.dark) .log-panel__thread-summary,
:global([data-theme='dark']) .empty-state,
:global(.dark) .empty-state,
:global([data-theme='dark']) .account-chip,
:global(.dark) .account-chip,
:global([data-theme='dark']) .channel-card__qr,
:global(.dark) .channel-card__qr {
  color: var(--color-text-primary);
}

:global([data-theme='dark']) .metric-card--signal,
:global(.dark) .metric-card--signal {
  background: linear-gradient(180deg, rgba(8, 47, 73, 0.9), rgba(15, 23, 42, 0.88));
}

:global([data-theme='dark']) .metric-card--active,
:global(.dark) .metric-card--active {
  background: linear-gradient(180deg, rgba(6, 78, 59, 0.88), rgba(15, 23, 42, 0.88));
}

:global([data-theme='dark']) .metric-card--risk,
:global(.dark) .metric-card--risk {
  background: linear-gradient(180deg, rgba(104, 48, 8, 0.88), rgba(15, 23, 42, 0.88));
}

:global([data-theme='dark']) .channel-card,
:global(.dark) .channel-card {
  background: linear-gradient(140deg, rgba(15, 23, 42, 0.88), rgba(30, 41, 59, 0.82));
}

:global([data-theme='dark']) .action-btn--soft,
:global(.dark) .action-btn--soft {
  background: rgba(59, 130, 246, 0.18);
  color: #bfdbfe;
}

:global([data-theme='dark']) .action-btn--warning,
:global(.dark) .action-btn--warning {
  background: rgba(251, 191, 36, 0.14);
  color: #fde68a;
}

:global([data-theme='dark']) .action-btn--danger,
:global(.dark) .action-btn--danger {
  background: rgba(248, 113, 113, 0.16);
  color: #fecaca;
}

:global([data-theme='dark']) .status-pill--listening,
:global(.dark) .status-pill--listening {
  background: rgba(16, 185, 129, 0.16);
  color: #a7f3d0;
}

:global([data-theme='dark']) .status-pill--warning,
:global(.dark) .status-pill--warning {
  background: rgba(245, 158, 11, 0.18);
  color: #fde68a;
}

:global([data-theme='dark']) .status-pill--idle,
:global(.dark) .status-pill--idle {
  background: rgba(96, 165, 250, 0.16);
  color: #bfdbfe;
}

:global([data-theme='dark']) .status-pill--error,
:global(.dark) .status-pill--error {
  background: rgba(248, 113, 113, 0.18);
  color: #fecaca;
}

:global([data-theme='dark']) .log-channel-chip,
:global(.dark) .log-channel-chip {
  color: var(--color-text-secondary);
}

:global([data-theme='dark']) .log-channel-chip--active,
:global(.dark) .log-channel-chip--active {
  color: #eff6ff;
}

:global([data-theme='dark']) .chat-bubble--center,
:global(.dark) .chat-bubble--center,
:global([data-theme='dark']) .chat-bubble--error,
:global(.dark) .chat-bubble--error {
  color: #f8fafc;
}

@media (max-width: 1480px) {
  .hero {
    grid-template-columns: 1fr;
    min-height: 0;
  }

  .hero__action {
    justify-self: start;
  }

  .channel-card__body {
    display: block;
  }

  .channel-side {
    width: 100%;
    justify-self: stretch;
  }
}

@media (max-width: 1100px) {
  .channel-form {
    grid-template-columns: 1fr;
  }

  .channel-side {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .chat-bubble {
    max-width: 100%;
  }
}

@media (max-width: 760px) {
  .unattended-panel {
    padding: 14px;
  }

  .hero,
  .channels-section,
  .log-panel {
    border-radius: 20px;
  }

  .hero,
  .channels-section,
  .log-panel,
  .channel-card {
    padding: 16px;
  }

  .hero__metrics,
  .channel-side {
    grid-template-columns: 1fr;
  }

  .channel-card__head,
  .channel-card__qr,
  .log-panel__thread-summary,
  .log-channel-switcher {
    flex-direction: column;
  }

  .channel-card__actions {
    justify-content: flex-start;
  }
}
</style>
