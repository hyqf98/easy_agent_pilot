<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import CliInstallerSection from '@/components/settings/cli/CliInstallerSection.vue'
import { useCliInstallerStore, type CliName } from '@/stores/cliInstaller'

withDefaults(defineProps<{
  embedded?: boolean
}>(), {
  embedded: false
})

const { t } = useI18n()
const cliInstallerStore = useCliInstallerStore()

async function handleCheckUpdate(cliName: string) {
  await cliInstallerStore.checkUpdate(cliName as CliName)
}

async function handleInstall(cliName: string, method: string) {
  await cliInstallerStore.installCli(cliName as CliName, method)
}

async function handleUpgrade(cliName: string) {
  await cliInstallerStore.upgradeCli(cliName as CliName)
}

onMounted(async () => {
  await cliInstallerStore.ensureReady()
})
</script>

<template>
  <div
    class="settings-page"
    :class="{ 'settings-page--embedded': embedded }"
  >
    <div class="settings-page__header">
      <h3
        class="settings-page__title"
        :class="{ 'settings-page__title--embedded': embedded }"
      >
        {{ t('settings.cli.title') }}
      </h3>
    </div>

    <div class="settings-page__hint">
      <EaIcon
        name="info"
        :size="16"
      />
      <span>{{ t('settings.agentList.detectedTools') }}，可直接添加为 Agent。</span>
    </div>

    <CliInstallerSection
      :cards="cliInstallerStore.installerCards"
      :install-logs="cliInstallerStore.installLogs"
      :active-operations="cliInstallerStore.activeOperations"
      :checking-updates="cliInstallerStore.checkingUpdates"
      :visible-logs="cliInstallerStore.visibleLogsMap"
      :loading="!cliInstallerStore.isReady"
      @install="handleInstall"
      @upgrade="handleUpgrade"
      @check-update="handleCheckUpdate"
      @clear-logs="cliInstallerStore.clearLogs"
      @clear-logs-for-cli="(cliName) => cliInstallerStore.clearLogsForCli(cliName)"
    />
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-6);
}

.settings-page--embedded {
  padding-top: var(--spacing-2);
  border-top: 1px solid var(--color-border);
}

.settings-page__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.settings-page__title {
  margin: 0;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.settings-page__title--embedded {
  font-size: var(--font-size-lg);
}

.settings-page__hint {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-4);
  background-color: var(--color-primary-light);
  border-radius: var(--radius-lg);
}

.settings-page__hint :deep(svg),
.settings-page__hint .ea-icon {
  color: var(--color-primary);
}

.settings-page__hint span {
  font-size: var(--font-size-sm);
  color: var(--color-primary);
}
</style>
