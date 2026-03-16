<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAgentStore } from '@/stores/agent'
import { useMarketplaceStore, type McpInstallInput } from '@/stores/marketplace'
import { EaButton, EaIcon, EaInput, EaModal, EaSelect } from '@/components/common'
import type { McpMarketItem } from '@/types/marketplace'

interface Props {
  mcpItem: McpMarketItem
}

const props = defineProps<Props>()

const emit = defineEmits<{
  close: []
  complete: []
}>()

const { t } = useI18n()
const agentStore = useAgentStore()
const marketplaceStore = useMarketplaceStore()

const selectedAgentId = ref('')
const customCommand = ref('')
const customArgs = ref('')
const envKey = ref('')
const envValue = ref('')
const customEnv = ref<Record<string, string>>({})
const isInstalling = ref(false)
const installError = ref<string | null>(null)
const installSuccess = ref(false)

const cliAgents = computed(() =>
  agentStore.agents.filter(
    agent => agent.type === 'cli' && (agent.provider === 'claude' || agent.provider === 'codex')
  )
)

const selectedAgent = computed(() =>
  cliAgents.value.find(agent => agent.id === selectedAgentId.value)
)

const selectedDetail = computed(() => {
  const detail = marketplaceStore.selectedMcpDetail
  return detail?.slug === props.mcpItem.slug ? detail : null
})

const defaultCommand = computed(() =>
  selectedDetail.value?.install_command || props.mcpItem.install_command || 'npx'
)

const defaultArgs = computed(() =>
  (selectedDetail.value?.install_args || props.mcpItem.install_args || []).join(' ')
)

const canInstall = computed(() => Boolean(selectedAgentId.value) && !isInstalling.value)

function splitArgs(value: string): string[] {
  const matches = value.match(/"[^"]*"|'[^']*'|\S+/g) || []
  return matches
    .map(item => item.trim().replace(/^['"]|['"]$/g, ''))
    .filter(Boolean)
}

function addEnv() {
  if (!envKey.value || !envValue.value) {
    return
  }

  customEnv.value = {
    ...customEnv.value,
    [envKey.value.trim()]: envValue.value.trim()
  }
  envKey.value = ''
  envValue.value = ''
}

function removeEnv(key: string) {
  const nextEnv = { ...customEnv.value }
  delete nextEnv[key]
  customEnv.value = nextEnv
}

async function handleInstall() {
  if (!selectedAgent.value || !canInstall.value) {
    return
  }

  isInstalling.value = true
  installError.value = null
  installSuccess.value = false

  try {
    const input: McpInstallInput = {
      mcp_id: props.mcpItem.slug,
      mcp_name: props.mcpItem.slug,
      cli_path: selectedAgent.value.cliPath || selectedAgent.value.provider || 'claude',
      command: customCommand.value || defaultCommand.value,
      args: splitArgs(customArgs.value || defaultArgs.value),
      env: { ...customEnv.value },
      scope: 'global',
      project_path: null
    }

    const result = await marketplaceStore.installMcp(input)
    if (result.success) {
      installSuccess.value = true
      setTimeout(() => emit('complete'), 1200)
      return
    }

    installError.value = result.message
  } catch (error) {
    installError.value = error instanceof Error ? error.message : t('marketplace.installFailed')
  } finally {
    isInstalling.value = false
  }
}

function handleClose() {
  if (!isInstalling.value) {
    emit('close')
  }
}

onMounted(async () => {
  if (cliAgents.value.length > 0) {
    selectedAgentId.value = cliAgents.value[0].id
  }

  await marketplaceStore.fetchMcpDetail(props.mcpItem.slug, marketplaceStore.activeMarketSource)
  customCommand.value = defaultCommand.value
  customArgs.value = defaultArgs.value
})
</script>

<template>
  <EaModal
    :visible="true"
    :title="t('marketplace.installMcp')"
    size="md"
    @close="handleClose"
  >
    <div class="mcp-install-modal">
      <div
        v-if="installSuccess"
        class="mcp-install-modal__success"
      >
        <EaIcon
          name="check-circle"
          :size="48"
          class="mcp-install-modal__success-icon"
        />
        <p>{{ t('marketplace.installSuccess') }}</p>
      </div>

      <template v-else>
        <div class="mcp-install-modal__info">
          <h4>{{ mcpItem.name }}</h4>
          <p>{{ mcpItem.description }}</p>
          <div class="mcp-install-modal__meta">
            <span v-if="mcpItem.category">{{ mcpItem.category }}</span>
            <span v-if="mcpItem.stars">{{ mcpItem.stars.toLocaleString() }} ★</span>
          </div>
        </div>

        <div class="mcp-install-modal__field">
          <label>{{ t('marketplace.selectAgent') }}</label>
          <EaSelect
            v-model="selectedAgentId"
            :options="cliAgents.map(agent => ({
              value: agent.id,
              label: `${agent.name} · ${(agent.provider || 'claude').toUpperCase()}`
            }))"
            :placeholder="t('marketplace.selectAgentPlaceholder')"
          />
          <p
            v-if="cliAgents.length === 0"
            class="mcp-install-modal__hint"
          >
            {{ t('marketplace.noCliAgent') }}
          </p>
          <p
            v-else
            class="mcp-install-modal__hint"
          >
            {{ t('marketplace.globalOnly') }}
          </p>
        </div>

        <div class="mcp-install-modal__field">
          <label>{{ t('marketplace.command') }}</label>
          <EaInput
            v-model="customCommand"
            :placeholder="defaultCommand"
          />
        </div>

        <div class="mcp-install-modal__field">
          <label>{{ t('marketplace.args') }}</label>
          <EaInput
            v-model="customArgs"
            :placeholder="defaultArgs"
          />
        </div>

        <div class="mcp-install-modal__field">
          <label>{{ t('marketplace.envVars') }}</label>
          <div
            v-if="Object.keys(customEnv).length > 0"
            class="mcp-install-modal__env-list"
          >
            <div
              v-for="(value, key) in customEnv"
              :key="key"
              class="mcp-install-modal__env-item"
            >
              <span class="mcp-install-modal__env-key">{{ key }}</span>
              <span class="mcp-install-modal__env-value">{{ value }}</span>
              <button
                class="mcp-install-modal__env-remove"
                @click="removeEnv(key)"
              >
                <EaIcon name="x" :size="14" />
              </button>
            </div>
          </div>

          <div class="mcp-install-modal__env-add">
            <EaInput
              v-model="envKey"
              :placeholder="t('marketplace.envKey')"
            />
            <EaInput
              v-model="envValue"
              :placeholder="t('marketplace.envValue')"
            />
            <EaButton
              type="ghost"
              size="small"
              :disabled="!envKey || !envValue"
              @click="addEnv"
            >
              <EaIcon name="plus" :size="16" />
            </EaButton>
          </div>
        </div>

        <div
          v-if="installError"
          class="mcp-install-modal__error"
        >
          <EaIcon name="alert-circle" :size="16" />
          <span>{{ installError }}</span>
        </div>
      </template>
    </div>

    <template #footer>
      <div class="mcp-install-modal__footer">
        <EaButton
          variant="ghost"
          @click="handleClose"
        >
          {{ installSuccess ? t('common.close') : t('common.cancel') }}
        </EaButton>
        <EaButton
          v-if="!installSuccess"
          variant="primary"
          :disabled="!canInstall"
          :loading="isInstalling"
          @click="handleInstall"
        >
          {{ t('marketplace.install') }}
        </EaButton>
      </div>
    </template>
  </EaModal>
</template>

<style scoped>
.mcp-install-modal {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.mcp-install-modal__success {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-8);
  text-align: center;
}

.mcp-install-modal__success-icon {
  margin-bottom: var(--spacing-4);
  color: var(--color-success);
}

.mcp-install-modal__info {
  padding: var(--spacing-3);
  background: var(--color-bg-secondary);
  border-radius: var(--radius-md);
}

.mcp-install-modal__info h4 {
  margin: 0 0 var(--spacing-1);
  font-size: var(--font-size-base);
}

.mcp-install-modal__info p {
  margin: 0;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.mcp-install-modal__meta {
  display: flex;
  gap: var(--spacing-3);
  margin-top: var(--spacing-2);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.mcp-install-modal__field {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.mcp-install-modal__field label {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
}

.mcp-install-modal__hint {
  margin: 0;
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.mcp-install-modal__env-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.mcp-install-modal__env-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto;
  gap: var(--spacing-2);
  align-items: center;
  padding: var(--spacing-2) var(--spacing-3);
  background: var(--color-bg-secondary);
  border-radius: var(--radius-md);
}

.mcp-install-modal__env-key,
.mcp-install-modal__env-value {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-size-xs);
}

.mcp-install-modal__env-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  cursor: pointer;
}

.mcp-install-modal__env-add {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto;
  gap: var(--spacing-2);
}

.mcp-install-modal__error {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-3);
  background: var(--color-danger-light);
  border-radius: var(--radius-md);
  color: var(--color-danger);
  font-size: var(--font-size-sm);
}

.mcp-install-modal__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-2);
}
</style>
