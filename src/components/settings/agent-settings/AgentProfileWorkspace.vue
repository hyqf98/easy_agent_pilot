<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { EaButton, EaIcon, EaStateBlock } from '@/components/common'
import type { AgentConfig } from '@/stores/agent'
import { useAgentProfileStore } from '@/stores/agentProfile'
import { useSkillConfigStore } from '@/stores/skillConfig'
import { useAgentTeamStore } from '@/stores/agentTeam'
import SkillConfigPage from '@/components/skill-config/SkillConfigPage.vue'

const props = defineProps<{
  agent: AgentConfig | null
}>()

const emit = defineEmits<{
  manageModels: [agent: AgentConfig]
}>()

const agentProfileStore = useAgentProfileStore()
const skillConfigStore = useSkillConfigStore()
const agentTeamStore = useAgentTeamStore()

const activeTab = ref<'profile' | 'resources' | 'teams'>('profile')
const isSaving = ref(false)

const form = ref({
  roleDefinition: '',
  systemPrompt: '',
  workingStyle: '',
  outputStyle: '',
  toolUsagePolicy: '',
  domainTagsText: '',
  capabilityTagsText: '',
  readonlyResearcher: false,
  planningCapability: false,
  executionCapability: true,
  defaultExecutionMode: ''
})

function parseTags(value: string): string[] {
  return value
    .split(/[\n,，]/)
    .map(item => item.trim())
    .filter(Boolean)
}

function fillForm() {
  if (!props.agent) {
    return
  }

  const profile = agentProfileStore.getProfile(props.agent.id)
  form.value = {
    roleDefinition: profile?.roleDefinition || '',
    systemPrompt: profile?.systemPrompt || '',
    workingStyle: profile?.workingStyle || '',
    outputStyle: profile?.outputStyle || '',
    toolUsagePolicy: profile?.toolUsagePolicy || '',
    domainTagsText: profile?.domainTags.join(', ') || '',
    capabilityTagsText: profile?.capabilityTags.join(', ') || '',
    readonlyResearcher: profile?.readonlyResearcher || false,
    planningCapability: profile?.planningCapability || false,
    executionCapability: profile?.executionCapability ?? true,
    defaultExecutionMode: profile?.defaultExecutionMode || ''
  }
}

watch(
  () => props.agent?.id,
  async (agentId) => {
    if (!agentId || !props.agent) {
      return
    }

    await Promise.all([
      agentProfileStore.loadProfile(agentId),
      skillConfigStore.selectAgent(props.agent),
      agentTeamStore.teams.length === 0 ? agentTeamStore.loadTeams() : Promise.resolve()
    ])

    fillForm()
  },
  { immediate: true }
)

watch(
  () => agentProfileStore.getProfile(props.agent?.id || null)?.updatedAt,
  () => fillForm()
)

const teamMemberships = computed(() => {
  if (!props.agent) {
    return []
  }

  return agentTeamStore.teams
    .filter(team => team.members.some(member => member.agentId === props.agent?.id))
    .map(team => ({
      team,
      membership: team.members.find(member => member.agentId === props.agent?.id) || null
    }))
})

const currentResourceAgentId = computed(() => skillConfigStore.selectedAgent?.id)
const mcpCount = computed(() =>
  props.agent && currentResourceAgentId.value === props.agent.id
    ? skillConfigStore.mcpConfigs.length
    : 0
)
const skillCount = computed(() =>
  props.agent && currentResourceAgentId.value === props.agent.id
    ? skillConfigStore.skillsConfigs.length
    : 0
)
const pluginCount = computed(() =>
  props.agent && currentResourceAgentId.value === props.agent.id
    ? skillConfigStore.pluginsConfigs.length
    : 0
)

const capabilityChips = computed(() => {
  const values: string[] = []

  if (form.value.readonlyResearcher) values.push('只读研究')
  if (form.value.planningCapability) values.push('规划协调')
  if (form.value.executionCapability) values.push('执行交付')
  values.push(...parseTags(form.value.capabilityTagsText))

  return Array.from(new Set(values))
})

async function saveProfile() {
  if (!props.agent) {
    return
  }

  isSaving.value = true
  try {
    await agentProfileStore.saveProfile(props.agent.id, {
      roleDefinition: form.value.roleDefinition.trim() || undefined,
      systemPrompt: form.value.systemPrompt.trim() || undefined,
      workingStyle: form.value.workingStyle.trim() || undefined,
      outputStyle: form.value.outputStyle.trim() || undefined,
      toolUsagePolicy: form.value.toolUsagePolicy.trim() || undefined,
      domainTags: parseTags(form.value.domainTagsText),
      capabilityTags: parseTags(form.value.capabilityTagsText),
      readonlyResearcher: form.value.readonlyResearcher,
      planningCapability: form.value.planningCapability,
      executionCapability: form.value.executionCapability,
      defaultExecutionMode: form.value.defaultExecutionMode || undefined
    })
  } finally {
    isSaving.value = false
  }
}
</script>

<template>
  <section class="agent-profile-workspace">
    <div class="agent-profile-workspace__header">
      <div>
        <h4 class="agent-profile-workspace__title">
          {{ props.agent ? `${props.agent.name} · Agent 工作区` : 'Agent 工作区' }}
        </h4>
        <p class="agent-profile-workspace__subtitle">
          在这里为单个 Agent 统一配置专业角色、资源能力与团队归属。
        </p>
      </div>
      <EaButton
        v-if="props.agent"
        size="small"
        type="secondary"
        @click="emit('manageModels', props.agent)"
      >
        <EaIcon name="cpu" />
        管理模型
      </EaButton>
    </div>

    <EaStateBlock
      v-if="!props.agent"
      icon="bot"
      title="选择一个 Agent"
      description="从上方列表选择 Agent 后，这里会显示它的专业化配置工作区。"
    />

    <template v-else>
      <div class="agent-profile-workspace__summary">
        <div class="summary-card">
          <span class="summary-card__label">类型</span>
          <strong>{{ props.agent.type.toUpperCase() }}</strong>
          <span class="summary-card__meta">{{ props.agent.provider || '未指定 provider' }}</span>
        </div>
        <div class="summary-card">
          <span class="summary-card__label">默认模型</span>
          <strong>{{ props.agent.modelId || '跟随默认模型' }}</strong>
          <span class="summary-card__meta">可在模型管理中修改</span>
        </div>
        <div class="summary-card">
          <span class="summary-card__label">资源概览</span>
          <strong>{{ mcpCount }} MCP / {{ skillCount }} Skills / {{ pluginCount }} Plugins</strong>
          <span class="summary-card__meta">按当前 Agent 实际配置统计</span>
        </div>
        <div class="summary-card">
          <span class="summary-card__label">团队归属</span>
          <strong>{{ teamMemberships.length }} 个 Team</strong>
          <span class="summary-card__meta">父子拓扑仍在下方统一管理</span>
        </div>
      </div>

      <div class="agent-profile-workspace__tabs">
        <button
          class="agent-profile-workspace__tab"
          :class="{ 'agent-profile-workspace__tab--active': activeTab === 'profile' }"
          @click="activeTab = 'profile'"
        >
          <EaIcon name="settings-2" />
          专业画像
        </button>
        <button
          class="agent-profile-workspace__tab"
          :class="{ 'agent-profile-workspace__tab--active': activeTab === 'resources' }"
          @click="activeTab = 'resources'"
        >
          <EaIcon name="boxes" />
          资源能力
        </button>
        <button
          class="agent-profile-workspace__tab"
          :class="{ 'agent-profile-workspace__tab--active': activeTab === 'teams' }"
          @click="activeTab = 'teams'"
        >
          <EaIcon name="git-branch" />
          Team 归属
        </button>
      </div>

      <div
        v-if="activeTab === 'profile'"
        class="agent-profile-workspace__panel"
      >
        <div class="profile-form">
          <label class="profile-form__field">
            <span>角色定位</span>
            <input
              v-model="form.roleDefinition"
              type="text"
              placeholder="例如：资深前端架构师 / 只读研究员 / 主控协调者"
            >
          </label>

          <label class="profile-form__field">
            <span>系统提示词</span>
            <textarea
              v-model="form.systemPrompt"
              rows="6"
              placeholder="定义这个 Agent 的长期角色、行为边界与专业风格。"
            />
          </label>

          <div class="profile-form__grid">
            <label class="profile-form__field">
              <span>工作方式</span>
              <textarea
                v-model="form.workingStyle"
                rows="4"
                placeholder="例如：先研究再落实现；偏保守；优先兼容现有架构。"
              />
            </label>
            <label class="profile-form__field">
              <span>输出偏好</span>
              <textarea
                v-model="form.outputStyle"
                rows="4"
                placeholder="例如：先结论后细节；给出风险和验证结果；中文输出。"
              />
            </label>
          </div>

          <label class="profile-form__field">
            <span>工具使用策略</span>
            <textarea
              v-model="form.toolUsagePolicy"
              rows="3"
              placeholder="例如：研究型 Agent 禁止写文件；执行型 Agent 必须先读上下文再动手。"
            />
          </label>

          <div class="profile-form__grid">
            <label class="profile-form__field">
              <span>领域标签</span>
              <input
                v-model="form.domainTagsText"
                type="text"
                placeholder="前端, Rust, 架构治理, 计划编排"
              >
            </label>
            <label class="profile-form__field">
              <span>能力标签</span>
              <input
                v-model="form.capabilityTagsText"
                type="text"
                placeholder="coordination, research, coding, testing"
              >
            </label>
          </div>

          <label class="profile-form__field">
            <span>默认执行模式</span>
            <select v-model="form.defaultExecutionMode">
              <option value="">
                跟随场景默认
              </option>
              <option value="chat">
                chat
              </option>
              <option value="task_split">
                task_split
              </option>
            </select>
          </label>

          <div class="profile-form__toggles">
            <label class="toggle-item">
              <input
                v-model="form.readonlyResearcher"
                type="checkbox"
              >
              <span>只读研究型 Agent</span>
            </label>
            <label class="toggle-item">
              <input
                v-model="form.planningCapability"
                type="checkbox"
              >
              <span>具备规划/协调能力</span>
            </label>
            <label class="toggle-item">
              <input
                v-model="form.executionCapability"
                type="checkbox"
              >
              <span>具备执行交付能力</span>
            </label>
          </div>

          <div
            v-if="capabilityChips.length > 0"
            class="profile-form__chips"
          >
            <span
              v-for="chip in capabilityChips"
              :key="chip"
              class="profile-chip"
            >
              {{ chip }}
            </span>
          </div>

          <div class="profile-form__actions">
            <EaButton
              type="primary"
              size="small"
              :loading="isSaving"
              @click="saveProfile"
            >
              保存 Agent Profile
            </EaButton>
          </div>
        </div>
      </div>

      <div
        v-else-if="activeTab === 'resources'"
        class="agent-profile-workspace__panel agent-profile-workspace__panel--resources"
      >
        <SkillConfigPage
          :embedded-agent="props.agent"
          hide-agent-selector
          compact
        />
      </div>

      <div
        v-else
        class="agent-profile-workspace__panel"
      >
        <div
          v-if="teamMemberships.length === 0"
          class="team-empty"
        >
          <EaIcon
            name="git-branch"
            :size="20"
          />
          <span>当前 Agent 还未加入任何 Team，可在下方 Agent Team 管理中配置。</span>
        </div>

        <div
          v-else
          class="team-list"
        >
          <article
            v-for="item in teamMemberships"
            :key="item.team.id"
            class="team-card"
          >
            <div class="team-card__header">
              <div>
                <h5>{{ item.team.name }}</h5>
                <p>{{ item.team.description || '未填写团队说明' }}</p>
              </div>
              <span class="team-card__badge">
                {{ item.membership?.role || 'member' }}
              </span>
            </div>
            <div class="team-card__meta">
              <span>能力预设：{{ item.membership?.capabilityPreset || '-' }}</span>
              <span v-if="item.membership?.modelId">Team 模型：{{ item.membership.modelId }}</span>
            </div>
          </article>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.agent-profile-workspace {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  padding: var(--spacing-4);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  background: var(--color-surface);
  min-height: 22rem;
  width: 100%;
  min-width: 0;
  overflow: hidden;
}

.agent-profile-workspace__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-3);
}

.agent-profile-workspace__title {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
}

.agent-profile-workspace__subtitle {
  margin: var(--spacing-1) 0 0;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.agent-profile-workspace__summary {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: var(--spacing-3);
}

.summary-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
  padding: var(--spacing-4);
  border-radius: var(--radius-lg);
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
}

.summary-card__label {
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.summary-card__meta {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.agent-profile-workspace__tabs {
  display: flex;
  gap: var(--spacing-2);
  flex-wrap: wrap;
}

.agent-profile-workspace__tab {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  color: var(--color-text-secondary);
}

.agent-profile-workspace__tab--active {
  color: var(--color-primary);
  border-color: var(--color-primary);
  background: var(--color-primary-light);
}

.agent-profile-workspace__panel {
  flex: 1;
  min-height: 0;
  min-width: 0;
}

.agent-profile-workspace__panel--resources {
  display: flex;
  min-height: 30rem;
  overflow: hidden;
}

.profile-form {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.profile-form__grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: var(--spacing-3);
}

.profile-form__field {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.profile-form__field span {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-secondary);
}

.profile-form__field input,
.profile-form__field textarea,
.profile-form__field select {
  width: 100%;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  color: var(--color-text-primary);
  padding: var(--spacing-3);
}

.profile-form__field textarea {
  resize: vertical;
}

.profile-form__toggles {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-4);
}

.toggle-item {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-2);
  color: var(--color-text-secondary);
}

.profile-form__chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-2);
}

.profile-chip {
  display: inline-flex;
  align-items: center;
  padding: var(--spacing-1) var(--spacing-2);
  border-radius: var(--radius-full);
  background: var(--color-bg-secondary);
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}

.profile-form__actions {
  display: flex;
  justify-content: flex-end;
}

@media (max-width: 960px) {
  .agent-profile-workspace__header {
    flex-direction: column;
    align-items: stretch;
  }

  .profile-form__actions {
    justify-content: stretch;
  }
}

.team-empty {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-4);
  border-radius: var(--radius-lg);
  background: var(--color-bg-secondary);
  color: var(--color-text-secondary);
}

.team-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.team-card {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: var(--spacing-4);
  background: var(--color-surface);
}

.team-card__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-3);
}

.team-card__header h5 {
  margin: 0;
  font-size: var(--font-size-base);
}

.team-card__header p {
  margin: var(--spacing-1) 0 0;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.team-card__badge {
  display: inline-flex;
  align-items: center;
  padding: var(--spacing-1) var(--spacing-2);
  border-radius: var(--radius-full);
  background: var(--color-primary-light);
  color: var(--color-primary);
  font-size: var(--font-size-xs);
}

.team-card__meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-3);
  margin-top: var(--spacing-3);
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}
</style>
