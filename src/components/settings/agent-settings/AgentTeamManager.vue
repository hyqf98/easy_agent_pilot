<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { EaButton, EaIcon, EaStateBlock } from '@/components/common'
import { useAgentStudioStore } from '@/stores/agentStudio'
import {
  useAgentTeamStore,
  type AgentTeam,
  type AgentTeamDraft,
  type AgentTeamDraftMember,
  type AgentTeamDraftRelation
} from '@/stores/agentTeam'

const agentStudioStore = useAgentStudioStore()
const agentTeamStore = useAgentTeamStore()

const showModal = ref(false)
const editingTeamId = ref<string | null>(null)
const isSaving = ref(false)

const draft = reactive<AgentTeamDraft>({
  name: '',
  description: '',
  mode: 'coordinator_subagents',
  coordinatorAgentId: undefined,
  coordinatorModelId: '',
  subagentMaxConcurrent: 3,
  maxChildrenPerTask: 5,
  members: [],
  relations: []
})

const memberOptions = computed(() =>
  draft.members.map(member => ({
    label: member.displayName?.trim() || agentStudioStore.getAgent(member.agentId)?.name || member.agentId,
    value: member.id || ''
  }))
)

function createMember(role: AgentTeamDraftMember['role'] = 'research_subagent'): AgentTeamDraftMember {
  return {
    id: crypto.randomUUID(),
    agentId: agentStudioStore.agents[0]?.id || '',
    modelId: '',
    role,
    capabilityPreset: role === 'coordinator' ? 'full' : 'readonly_research',
    displayName: '',
    sortOrder: draft.members.length
  }
}

function createRelation(): AgentTeamDraftRelation {
  const parentId = draft.members.find(member => member.role === 'coordinator')?.id || draft.members[0]?.id || ''
  const childId = draft.members.find(member => member.role !== 'coordinator')?.id || draft.members[1]?.id || ''
  return {
    id: crypto.randomUUID(),
    parentMemberId: parentId,
    childMemberId: childId,
    relationType: 'delegates_to',
    autoDispatchAllowed: true
  }
}

function resetDraft() {
  draft.name = ''
  draft.description = ''
  draft.mode = 'coordinator_subagents'
  draft.coordinatorAgentId = undefined
  draft.coordinatorModelId = ''
  draft.subagentMaxConcurrent = 3
  draft.maxChildrenPerTask = 5
  draft.members = [createMember('coordinator')]
  draft.relations = []
  editingTeamId.value = null
}

function applyTeam(team: AgentTeam) {
  draft.name = team.name
  draft.description = team.description || ''
  draft.mode = team.mode
  draft.coordinatorAgentId = team.coordinatorAgentId
  draft.coordinatorModelId = team.coordinatorModelId || ''
  draft.subagentMaxConcurrent = team.subagentMaxConcurrent
  draft.maxChildrenPerTask = team.maxChildrenPerTask
  draft.members = team.members.map(member => ({
    id: member.id,
    agentId: member.agentId,
    modelId: member.modelId || '',
    role: member.role,
    capabilityPreset: member.capabilityPreset,
    displayName: member.displayName || '',
    sortOrder: member.sortOrder
  }))
  draft.relations = team.relations.map(relation => ({
    id: relation.id,
    parentMemberId: relation.parentMemberId,
    childMemberId: relation.childMemberId,
    relationType: relation.relationType,
    autoDispatchAllowed: relation.autoDispatchAllowed
  }))
  editingTeamId.value = team.id
}

function openCreate() {
  resetDraft()
  showModal.value = true
}

function openEdit(team: AgentTeam) {
  applyTeam(team)
  showModal.value = true
}

function closeModal() {
  showModal.value = false
  resetDraft()
}

function addMember(role: AgentTeamDraftMember['role'] = 'research_subagent') {
  draft.members.push(createMember(role))
}

function removeMember(memberId?: string) {
  if (!memberId) return
  draft.members = draft.members.filter(member => member.id !== memberId)
  draft.relations = draft.relations.filter(relation =>
    relation.parentMemberId !== memberId && relation.childMemberId !== memberId
  )
}

function addRelation() {
  draft.relations.push(createRelation())
}

function removeRelation(relationId?: string) {
  if (!relationId) return
  draft.relations = draft.relations.filter(relation => relation.id !== relationId)
}

async function saveTeam() {
  if (!draft.name.trim() || draft.members.length === 0) return
  isSaving.value = true
  try {
    if (editingTeamId.value) {
      await agentTeamStore.updateTeam(editingTeamId.value, {
        ...draft,
        description: draft.description?.trim() || undefined,
        coordinatorModelId: draft.coordinatorModelId?.trim() || undefined
      })
    } else {
      await agentTeamStore.createTeam({
        ...draft,
        description: draft.description?.trim() || undefined,
        coordinatorModelId: draft.coordinatorModelId?.trim() || undefined
      })
    }
    closeModal()
  } finally {
    isSaving.value = false
  }
}

async function deleteTeam(team: AgentTeam) {
  const confirmed = window.confirm(`确定删除团队「${team.name}」吗？`)
  if (!confirmed) return
  await agentTeamStore.deleteTeam(team.id)
}

onMounted(() => {
  if (agentStudioStore.agents.length === 0) {
    void agentStudioStore.loadAgents()
  }
  if (agentTeamStore.teams.length === 0) {
    void agentTeamStore.loadTeams()
  }
  resetDraft()
})
</script>

<template>
  <section class="team-manager">
    <div class="team-manager__header">
      <div>
        <h3 class="team-manager__title">
          多 Agent 团队
        </h3>
        <p class="team-manager__desc">
          配置主控与只读研究型子代理的父子关系，供任务拆解时复用。
        </p>
      </div>
      <EaButton
        type="primary"
        size="small"
        @click="openCreate"
      >
        <EaIcon
          name="plus"
          :size="14"
        />
        新建团队
      </EaButton>
    </div>

    <div
      v-if="agentTeamStore.sortedTeams.length > 0"
      class="team-list"
    >
      <article
        v-for="team in agentTeamStore.sortedTeams"
        :key="team.id"
        class="team-card"
      >
        <div class="team-card__header">
          <div>
            <h4>{{ team.name }}</h4>
            <p>{{ team.description || '未填写说明' }}</p>
          </div>
          <div class="team-card__actions">
            <EaButton
              type="ghost"
              size="small"
              @click="openEdit(team)"
            >
              <EaIcon
                name="edit-2"
                :size="14"
              />
            </EaButton>
            <EaButton
              type="ghost"
              size="small"
              @click="deleteTeam(team)"
            >
              <EaIcon
                name="trash-2"
                :size="14"
              />
            </EaButton>
          </div>
        </div>
        <div class="team-card__meta">
          <span>模式：{{ team.mode === 'coordinator_subagents' ? '主控 + 子代理' : '单 Agent' }}</span>
          <span>并发：{{ team.subagentMaxConcurrent }}</span>
          <span>每任务子代理上限：{{ team.maxChildrenPerTask }}</span>
        </div>
        <div class="team-card__members">
          <div
            v-for="member in team.members"
            :key="member.id"
            class="team-card__member"
          >
            <strong>{{ member.displayName || agentStudioStore.getAgent(member.agentId)?.name || member.agentId }}</strong>
            <span>{{ member.role === 'coordinator' ? '主控' : '研究子代理' }}</span>
            <span>{{ member.capabilityPreset }}</span>
          </div>
        </div>
      </article>
    </div>

    <EaStateBlock
      v-else
      icon="git-branch"
      title="暂无团队"
      description="先创建一个主控 + 研究子代理团队，再在计划拆解里启用。"
    />

    <Teleport to="body">
      <div
        v-if="showModal"
        class="team-modal__overlay"
        @click.self="closeModal"
      >
        <div class="team-modal">
          <div class="team-modal__header">
            <h3>{{ editingTeamId ? '编辑团队' : '新建团队' }}</h3>
            <button
              class="team-modal__close"
              @click="closeModal"
            >
              ×
            </button>
          </div>

          <div class="team-modal__body">
            <div class="team-form__row">
              <label>团队名称</label>
              <input
                v-model="draft.name"
                type="text"
                placeholder="例如：需求拆解团队"
              >
            </div>
            <div class="team-form__row">
              <label>团队说明</label>
              <textarea
                v-model="draft.description"
                rows="2"
                placeholder="说明主控和子代理的职责"
              />
            </div>
            <div class="team-form__grid">
              <div class="team-form__row">
                <label>模式</label>
                <select v-model="draft.mode">
                  <option value="coordinator_subagents">
                    主控 + 研究子代理
                  </option>
                  <option value="single">
                    单 Agent
                  </option>
                </select>
              </div>
              <div class="team-form__row">
                <label>主控 Agent</label>
                <select v-model="draft.coordinatorAgentId">
                  <option
                    v-for="agent in agentStudioStore.agents"
                    :key="agent.id"
                    :value="agent.id"
                  >
                    {{ agent.name }}
                  </option>
                </select>
              </div>
              <div class="team-form__row">
                <label>主控模型</label>
                <input
                  v-model="draft.coordinatorModelId"
                  type="text"
                  placeholder="可留空使用默认"
                >
              </div>
              <div class="team-form__row">
                <label>子代理并发</label>
                <input
                  v-model.number="draft.subagentMaxConcurrent"
                  type="number"
                  min="1"
                  max="8"
                >
              </div>
              <div class="team-form__row">
                <label>每任务子代理上限</label>
                <input
                  v-model.number="draft.maxChildrenPerTask"
                  type="number"
                  min="1"
                  max="12"
                >
              </div>
            </div>

            <div class="team-section">
              <div class="team-section__header">
                <h4>成员</h4>
                <EaButton
                  type="ghost"
                  size="small"
                  @click="addMember()"
                >
                  <EaIcon
                    name="plus"
                    :size="14"
                  />
                  添加成员
                </EaButton>
              </div>
              <div
                v-for="member in draft.members"
                :key="member.id"
                class="team-member-row"
              >
                <input
                  v-model="member.displayName"
                  type="text"
                  placeholder="显示名称"
                >
                <select v-model="member.role">
                  <option value="coordinator">
                    主控
                  </option>
                  <option value="research_subagent">
                    研究子代理
                  </option>
                </select>
                <select v-model="member.agentId">
                  <option
                    v-for="agent in agentStudioStore.agents"
                    :key="agent.id"
                    :value="agent.id"
                  >
                    {{ agent.name }}
                  </option>
                </select>
                <input
                  v-model="member.modelId"
                  type="text"
                  placeholder="模型 ID"
                >
                <select v-model="member.capabilityPreset">
                  <option value="full">
                    full
                  </option>
                  <option value="readonly_research">
                    readonly_research
                  </option>
                </select>
                <button
                  class="team-row__remove"
                  @click="removeMember(member.id)"
                >
                  ×
                </button>
              </div>
            </div>

            <div class="team-section">
              <div class="team-section__header">
                <h4>父子关系</h4>
                <EaButton
                  type="ghost"
                  size="small"
                  @click="addRelation"
                >
                  <EaIcon
                    name="plus"
                    :size="14"
                  />
                  添加关系
                </EaButton>
              </div>
              <div
                v-for="relation in draft.relations"
                :key="relation.id"
                class="team-member-row"
              >
                <select v-model="relation.parentMemberId">
                  <option
                    v-for="option in memberOptions"
                    :key="option.value"
                    :value="option.value"
                  >
                    {{ option.label }}
                  </option>
                </select>
                <select v-model="relation.childMemberId">
                  <option
                    v-for="option in memberOptions"
                    :key="option.value"
                    :value="option.value"
                  >
                    {{ option.label }}
                  </option>
                </select>
                <input
                  v-model="relation.relationType"
                  type="text"
                  placeholder="delegates_to"
                >
                <label class="team-form__checkbox">
                  <input
                    v-model="relation.autoDispatchAllowed"
                    type="checkbox"
                  >
                  自动派发
                </label>
                <button
                  class="team-row__remove"
                  @click="removeRelation(relation.id)"
                >
                  ×
                </button>
              </div>
            </div>
          </div>

          <div class="team-modal__footer">
            <EaButton
              type="ghost"
              size="small"
              @click="closeModal"
            >
              取消
            </EaButton>
            <EaButton
              type="primary"
              size="small"
              :loading="isSaving"
              @click="saveTeam"
            >
              保存
            </EaButton>
          </div>
        </div>
      </div>
    </Teleport>
  </section>
</template>

<style scoped>
.team-manager {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 0;
  width: 100%;
  min-width: 0;
}

.team-manager__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.team-manager__title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.team-manager__desc {
  margin: 4px 0 0;
  color: var(--color-text-secondary);
  font-size: 12px;
}

.team-list {
  display: grid;
  gap: 12px;
}

.team-card {
  border: 1px solid var(--color-border);
  border-radius: 12px;
  background: var(--color-surface);
  padding: 12px;
}

.team-card__header,
.team-section__header,
.team-modal__header,
.team-modal__footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.team-card__header h4,
.team-section__header h4,
.team-modal__header h3 {
  margin: 0;
}

.team-card__header p {
  margin: 4px 0 0;
  color: var(--color-text-secondary);
  font-size: 12px;
}

.team-card__actions,
.team-card__meta,
.team-card__members {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.team-card__meta {
  margin-top: 8px;
  color: var(--color-text-secondary);
  font-size: 12px;
}

.team-card__member {
  display: inline-flex;
  gap: 6px;
  align-items: center;
  padding: 4px 8px;
  border-radius: 999px;
  background: var(--color-bg-secondary);
  font-size: 12px;
}

.team-modal__overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 3000;
}

.team-modal {
  width: min(960px, 92vw);
  max-height: 88vh;
  background: var(--color-surface);
  border-radius: 16px;
  display: flex;
  flex-direction: column;
}

.team-modal__header,
.team-modal__footer {
  padding: 16px 20px;
  border-bottom: 1px solid var(--color-border);
}

.team-modal__footer {
  border-top: 1px solid var(--color-border);
  border-bottom: none;
}

.team-modal__body {
  overflow: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.team-form__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.team-form__row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.team-form__row label,
.team-form__checkbox {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.team-form__row input,
.team-form__row textarea,
.team-form__row select,
.team-member-row input,
.team-member-row select {
  width: 100%;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 8px 10px;
  background: var(--color-surface);
  color: var(--color-text-primary);
}

.team-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.team-member-row {
  display: grid;
  grid-template-columns: 1.2fr 0.8fr 1fr 1fr 1fr auto;
  gap: 8px;
  align-items: center;
}

.team-row__remove,
.team-modal__close {
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: 18px;
  color: var(--color-text-secondary);
}

@media (max-width: 880px) {
  .team-form__grid,
  .team-member-row {
    grid-template-columns: 1fr;
  }
}
</style>
