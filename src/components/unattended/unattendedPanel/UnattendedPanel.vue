<script setup lang="ts">
import EaSelect from '@/components/common/EaSelect.vue'
import { useUnattendedPanel } from './useUnattendedPanel'

const {
  agentOptions,
  canStartRuntime,
  canStopRuntime,
  formatDateTime,
  getModelOptions,
  handleAgentChange,
  handleDeleteChannel,
  handleModelChange,
  handleProjectChange,
  handleRefreshLogin,
  logBubbles,
  logViewportRef,
  projectOptions,
  resolveAgentName,
  resolveChannelStatusLabel,
  resolveChannelStatusTone,
  resolveProjectName,
  resolveSelectedAgentId,
  resolveThreadAgentName,
  selectedChannel,
  selectedChannelId,
  selectedThread,
  shouldShowQrCode,
  systemRootPathHint,
  t,
  totalErrorAccounts,
  totalListeningAccounts,
  unattendedStore,
  weixinChannels
} = useUnattendedPanel()
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

<style scoped src="./styles.css"></style>
