<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAgentStore } from '@/stores/agent'
import { useMarketplaceStore, type SkillInstallInput } from '@/stores/marketplace'
import { EaButton, EaIcon, EaModal, EaSelect } from '@/components/common'
import type { SkillMarketItem } from '@/types/marketplace'

interface Props {
  skillItem: SkillMarketItem
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
const isInstalling = ref(false)
const installError = ref<string | null>(null)
const installSuccess = ref(false)

const cliAgents = computed(() =>
  agentStore.agents.filter(
    agent => agent.type === 'cli' && (agent.provider === 'claude' || agent.provider === 'codex' || agent.provider === 'opencode')
  )
)

const selectedAgent = computed(() =>
  cliAgents.value.find(agent => agent.id === selectedAgentId.value)
)

const canInstall = computed(() => Boolean(selectedAgentId.value) && !isInstalling.value)

async function handleInstall() {
  if (!selectedAgent.value || !canInstall.value) {
    return
  }

  isInstalling.value = true
  installError.value = null
  installSuccess.value = false

  try {
    const input: SkillInstallInput = {
      skill_id: props.skillItem.slug,
      skill_name: props.skillItem.name,
      cli_type: (selectedAgent.value.provider || 'claude') as 'claude' | 'codex' | 'opencode',
      scope: 'global',
      project_path: null,
      source_market: marketplaceStore.activeMarketSource
    }

    const result = await marketplaceStore.installSkill(input)
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
  await agentStore.loadAgents()
  if (cliAgents.value.length > 0) {
    selectedAgentId.value = cliAgents.value[0].id
  }
})
</script>

<template>
  <EaModal
    :visible="true"
    :title="t('marketplace.installSkill')"
    size="md"
    @close="handleClose"
  >
    <div class="skill-install-modal">
      <div
        v-if="installSuccess"
        class="skill-install-modal__success"
      >
        <EaIcon
          name="check-circle"
          :size="48"
          class="skill-install-modal__success-icon"
        />
        <p>{{ t('marketplace.installSuccess') }}</p>
      </div>

      <template v-else>
        <div class="skill-install-modal__info">
          <h4>{{ skillItem.name }}</h4>
          <p>{{ skillItem.description }}</p>
          <div class="skill-install-modal__meta">
            <span v-if="skillItem.author">{{ skillItem.author }}</span>
            <span v-if="skillItem.stars">{{ skillItem.stars.toLocaleString() }} ★</span>
          </div>
        </div>

        <div class="skill-install-modal__field">
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
            class="skill-install-modal__hint"
          >
            {{ t('marketplace.noCliAgent') }}
          </p>
          <p
            v-else
            class="skill-install-modal__hint"
          >
            {{ t('marketplace.globalSkillInstall') }}
          </p>
        </div>

        <div
          v-if="installError"
          class="skill-install-modal__error"
        >
          <EaIcon
            name="alert-circle"
            :size="16"
          />
          <span>{{ installError }}</span>
        </div>
      </template>
    </div>

    <template #footer>
      <div class="skill-install-modal__footer">
        <EaButton
          type="ghost"
          @click="handleClose"
        >
          {{ installSuccess ? t('common.close') : t('common.cancel') }}
        </EaButton>
        <EaButton
          v-if="!installSuccess"
          type="primary"
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
.skill-install-modal {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.skill-install-modal__success {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-8);
  text-align: center;
}

.skill-install-modal__success-icon {
  margin-bottom: var(--spacing-4);
  color: var(--color-success);
}

.skill-install-modal__info {
  padding: var(--spacing-3);
  background: var(--color-bg-secondary);
  border-radius: var(--radius-md);
}

.skill-install-modal__info h4 {
  margin: 0 0 var(--spacing-1);
  font-size: var(--font-size-base);
}

.skill-install-modal__info p {
  margin: 0;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.skill-install-modal__meta {
  display: flex;
  gap: var(--spacing-3);
  margin-top: var(--spacing-2);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.skill-install-modal__field {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.skill-install-modal__field label {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
}

.skill-install-modal__hint {
  margin: 0;
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.skill-install-modal__error {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-3);
  background: var(--color-danger-light);
  border-radius: var(--radius-md);
  color: var(--color-danger);
  font-size: var(--font-size-sm);
}

.skill-install-modal__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-2);
}
</style>
