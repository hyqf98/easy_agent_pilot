<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon, EaStateBlock } from '@/components/common'
import { useNotificationStore } from '@/stores/notification'
import { getErrorMessage } from '@/utils/api'
import { useFileEditorStore } from '@/modules/fileEditor'
import SettingsSectionCard from '@/components/settings/common/SettingsSectionCard.vue'
import {
  downloadLspServer,
  getLspStorageDir,
  listLspServers,
  removeLspServer,
  type LspServerInfo
} from '@/modules/fileEditor/services/lspService'

const { t } = useI18n()
const notificationStore = useNotificationStore()
const fileEditorStore = useFileEditorStore()

const servers = ref<LspServerInfo[]>([])
const storagePath = ref('')
const isLoading = ref(false)
const actionLoading = reactive<Record<string, boolean>>({})

const formatTime = (iso: string | null): string => {
  if (!iso) return '-'
  return new Date(iso).toLocaleString()
}

const updateServer = (next: LspServerInfo): void => {
  const index = servers.value.findIndex(item => item.id === next.id)
  if (index >= 0) {
    servers.value[index] = next
  }
}

const loadData = async (): Promise<void> => {
  isLoading.value = true
  try {
    const [path, list] = await Promise.all([getLspStorageDir(), listLspServers()])
    storagePath.value = path
    servers.value = list
  } catch (error) {
    notificationStore.error(t('settings.lsp.loadFailed'), getErrorMessage(error))
  } finally {
    isLoading.value = false
  }
}

const handleDownload = async (server: LspServerInfo): Promise<void> => {
  actionLoading[server.id] = true
  try {
    const next = await downloadLspServer(server.id)
    updateServer(next)
    await fileEditorStore.refreshActiveFileLanguage()
    notificationStore.success(t('settings.lsp.downloadSuccess'), server.name)
  } catch (error) {
    notificationStore.error(t('settings.lsp.downloadFailed'), getErrorMessage(error))
  } finally {
    actionLoading[server.id] = false
  }
}

const handleRemove = async (server: LspServerInfo): Promise<void> => {
  actionLoading[server.id] = true
  try {
    const next = await removeLspServer(server.id)
    updateServer(next)
    await fileEditorStore.refreshActiveFileLanguage()
    notificationStore.success(t('settings.lsp.removeSuccess'), server.name)
  } catch (error) {
    notificationStore.error(t('settings.lsp.removeFailed'), getErrorMessage(error))
  } finally {
    actionLoading[server.id] = false
  }
}

onMounted(() => {
  loadData()
})
</script>

<template>
  <div class="settings-page">
    <h3 class="settings-page__title">
      {{ t('settings.lsp.title') }}
    </h3>

    <SettingsSectionCard
      :title="t('settings.lsp.storageTitle')"
      :description="t('settings.lsp.storageDesc')"
    >
      <code class="storage-path">{{ storagePath || '~/.easy-agent/tools/lsp' }}</code>
    </SettingsSectionCard>

    <SettingsSectionCard :title="t('settings.lsp.serverListTitle')">
      <template #actions>
        <EaButton
          type="secondary"
          size="small"
          :loading="isLoading"
          @click="loadData"
        >
          <EaIcon
            name="refresh-cw"
            :size="14"
          />
          {{ t('common.refresh') }}
        </EaButton>
      </template>
      <template #description>
        {{ t('settings.lsp.manualOnly') }}
      </template>

      <div
        v-if="isLoading"
        class="lsp-state"
      >
        <EaStateBlock
          variant="loading"
          :description="t('common.loading')"
        />
      </div>

      <div
        v-else-if="servers.length > 0"
        class="lsp-list"
      >
        <div
          v-for="item in servers"
          :key="item.id"
          class="lsp-item"
        >
          <div class="lsp-item__meta">
            <div class="lsp-item__head">
              <span class="lsp-item__name">{{ item.name }}</span>
              <span
                class="lsp-item__status"
                :class="{ 'lsp-item__status--installed': item.installed }"
              >
                {{ item.installed ? t('settings.lsp.installed') : t('settings.lsp.notInstalled') }}
              </span>
            </div>
            <div class="lsp-item__desc">
              {{ item.description }}
            </div>
            <div class="lsp-item__extensions">
              <span
                v-for="ext in item.fileExtensions"
                :key="ext"
                class="lsp-item__ext"
              >
                .{{ ext }}
              </span>
            </div>
            <div class="lsp-item__time">
              {{ t('settings.lsp.installedAt') }}: {{ formatTime(item.installedAt) }}
            </div>
          </div>
          <div class="lsp-item__actions">
            <EaButton
              v-if="!item.installed"
              type="primary"
              size="small"
              :loading="Boolean(actionLoading[item.id])"
              @click="handleDownload(item)"
            >
              <EaIcon
                name="download"
                :size="14"
              />
              {{ t('settings.lsp.download') }}
            </EaButton>
            <EaButton
              v-else
              type="danger"
              size="small"
              :loading="Boolean(actionLoading[item.id])"
              @click="handleRemove(item)"
            >
              <EaIcon
                name="trash-2"
                :size="14"
              />
              {{ t('settings.lsp.remove') }}
            </EaButton>
          </div>
        </div>
      </div>
      <div
        v-else
        class="lsp-state"
      >
        <EaStateBlock
          variant="empty"
          :description="t('settings.lsp.manualOnly')"
        />
      </div>
    </SettingsSectionCard>
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-6);
}

.settings-page__title {
  margin: 0;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.storage-path {
  display: block;
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  color: var(--color-text-primary);
  word-break: break-all;
}

.lsp-state {
  display: flex;
}

.lsp-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.lsp-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-4);
  padding: var(--spacing-3) var(--spacing-4);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}

.lsp-item__meta {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.lsp-item__head {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.lsp-item__name {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.lsp-item__status {
  font-size: var(--font-size-xs);
  color: var(--color-warning);
  background: var(--color-warning-light);
  border-radius: var(--radius-full);
  padding: 2px 8px;
}

.lsp-item__status--installed {
  color: var(--color-success);
  background: var(--color-success-light);
}

.lsp-item__desc {
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.lsp-item__extensions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-2);
}

.lsp-item__ext {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  padding: 1px 6px;
  border-radius: var(--radius-sm);
}

.lsp-item__time {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.lsp-item__actions {
  flex-shrink: 0;
}

@media (max-width: 960px) {
  .lsp-item {
    flex-direction: column;
    align-items: stretch;
  }

  .lsp-item__actions {
    align-self: flex-start;
  }
}
</style>
