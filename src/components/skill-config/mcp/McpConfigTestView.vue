<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSkillConfigStore, type McpTool, type UnifiedMcpConfig } from '@/stores/skillConfig'
import { EaButton, EaIcon, EaJsonViewer, EaStateBlock } from '@/components/common'

const props = defineProps<{
  config: UnifiedMcpConfig
}>()

const emit = defineEmits<{
  back: []
}>()

const { t } = useI18n()
const skillConfigStore = useSkillConfigStore()

const isLoading = ref(false)
const tools = ref<McpTool[]>([])
const testError = ref<string | null>(null)
const selectedTool = ref<McpTool | null>(null)
const paramValues = ref<Record<string, unknown>>({})
const isCalling = ref(false)
const callResult = ref<{ success: boolean; data?: unknown; error?: string } | null>(null)
const activeTab = ref<'params' | 'result'>('params')

async function loadTools() {
  isLoading.value = true
  testError.value = null
  tools.value = []
  selectedTool.value = null
  callResult.value = null

  try {
    const result = await skillConfigStore.listMcpTools(props.config)
    if (result.success) {
      tools.value = result.tools
    } else {
      testError.value = result.message || t('settings.mcp.toolTester.loadFailed')
    }
  } catch (error) {
    testError.value = String(error)
  } finally {
    isLoading.value = false
  }
}

function selectTool(tool: McpTool) {
  selectedTool.value = tool
  paramValues.value = {}
  callResult.value = null
  activeTab.value = 'params'

  const properties = tool.inputSchema?.properties as Record<string, { default?: unknown }> | undefined
  if (!properties) return

  for (const [key, prop] of Object.entries(properties)) {
    if (prop.default !== undefined) {
      paramValues.value[key] = prop.default
    }
  }
}

async function handleCallTool() {
  if (!selectedTool.value) return

  isCalling.value = true
  activeTab.value = 'result'
  callResult.value = null

  try {
    const result = await skillConfigStore.callMcpTool(
      props.config,
      selectedTool.value.name,
      paramValues.value
    )

    callResult.value = {
      success: result.success,
      data: result.result,
      error: result.error
    }
  } catch (error) {
    callResult.value = {
      success: false,
      error: String(error)
    }
  } finally {
    isCalling.value = false
  }
}

function isRequired(paramName: string): boolean {
  const required = selectedTool.value?.inputSchema?.required as string[] | undefined
  return required?.includes(paramName) ?? false
}

function getParamType(paramName: string): string {
  const properties = selectedTool.value?.inputSchema?.properties as Record<string, { type?: string }> | undefined
  return properties?.[paramName]?.type || 'string'
}

void loadTools()
</script>

<template>
  <div class="mcp-test-view">
    <div class="mcp-test-view__header">
      <EaButton
        variant="ghost"
        size="small"
        @click="emit('back')"
      >
        <EaIcon name="lucide:arrow-left" />
        {{ t('common.back') }}
      </EaButton>
      <div class="mcp-test-view__title">
        <EaIcon name="lucide:server" />
        <span>{{ config.name }}</span>
      </div>
    </div>

    <div class="mcp-test-view__content">
      <div class="mcp-test-view__tools">
        <div class="tools-header">
          <h4>{{ t('settings.mcp.toolTester.availableTools') }}</h4>
          <span class="tools-count">{{ tools.length }}</span>
        </div>

        <EaStateBlock
          v-if="isLoading"
          variant="loading"
          :title="t('settings.mcp.toolTester.loadingTools')"
        />

        <EaStateBlock
          v-else-if="testError"
          variant="error"
          :title="t('common.loadFailed')"
          :description="testError"
        >
          <template #actions>
            <EaButton
              size="small"
              variant="ghost"
              @click="loadTools"
            >
              <EaIcon name="lucide:refresh-cw" />
              {{ t('common.retry') }}
            </EaButton>
          </template>
        </EaStateBlock>

        <EaStateBlock
          v-else-if="tools.length === 0"
          icon="lucide:wrench"
          :description="t('settings.mcp.toolTester.noTools')"
        />

        <div
          v-else
          class="tools-list"
        >
          <button
            v-for="tool in tools"
            :key="tool.name"
            class="tool-item"
            :class="{ 'tool-item--active': selectedTool?.name === tool.name }"
            @click="selectTool(tool)"
          >
            <EaIcon
              name="lucide:wrench"
              class="tool-item__icon"
            />
            <div class="tool-item__info">
              <span class="tool-item__name">{{ tool.name }}</span>
              <span class="tool-item__desc">{{ tool.description || '-' }}</span>
            </div>
          </button>
        </div>
      </div>

      <div class="mcp-test-view__main">
        <EaStateBlock
          v-if="!selectedTool"
          icon="lucide:mouse-pointer-click"
          :description="t('settings.mcp.toolTester.selectTool')"
        />

        <template v-else>
          <div class="detail-tabs">
            <button
              class="detail-tab"
              :class="{ 'detail-tab--active': activeTab === 'params' }"
              @click="activeTab = 'params'"
            >
              {{ t('settings.mcp.toolTester.tabParams') }}
            </button>
            <button
              class="detail-tab"
              :class="{ 'detail-tab--active': activeTab === 'result' }"
              @click="activeTab = 'result'"
            >
              {{ t('settings.mcp.toolTester.tabResult') }}
            </button>
          </div>

          <div
            v-show="activeTab === 'params'"
            class="detail-panel"
          >
            <div class="params-header">
              <h4>{{ selectedTool.name }}</h4>
              <p>{{ selectedTool.description }}</p>
            </div>

            <EaStateBlock
              v-if="!selectedTool.inputSchema?.properties"
              :description="t('settings.mcp.toolTester.noParams')"
            />

            <div
              v-else
              class="params-form"
            >
              <div
                v-for="(_prop, key) in selectedTool.inputSchema.properties"
                :key="key"
                class="form-group"
              >
                <label>
                  {{ key }}
                  <span
                    v-if="isRequired(key as string)"
                    class="required"
                  >*</span>
                  <span class="param-type">({{ getParamType(key as string) }})</span>
                </label>

                <input
                  v-if="getParamType(key as string) === 'string' || getParamType(key as string) === 'number'"
                  :value="paramValues[key as string]"
                  :type="getParamType(key as string) === 'number' ? 'number' : 'text'"
                  :placeholder="t('settings.mcp.toolTester.paramPlaceholder')"
                  @input="paramValues[key as string] = ($event.target as HTMLInputElement).value"
                >

                <textarea
                  v-else-if="getParamType(key as string) === 'object' || getParamType(key as string) === 'array'"
                  :value="String(paramValues[key as string] ?? '')"
                  :placeholder="t('settings.mcp.toolTester.jsonPlaceholder')"
                  rows="4"
                  @input="paramValues[key as string] = ($event.target as HTMLTextAreaElement).value"
                />

                <input
                  v-else
                  :value="paramValues[key as string]"
                  type="text"
                  :placeholder="t('settings.mcp.toolTester.paramPlaceholder')"
                  @input="paramValues[key as string] = ($event.target as HTMLInputElement).value"
                >
              </div>
            </div>

            <div class="params-actions">
              <EaButton
                :loading="isCalling"
                @click="handleCallTool"
              >
                <EaIcon name="lucide:play" />
                {{ t('settings.mcp.toolTester.callTool') }}
              </EaButton>
            </div>
          </div>

          <div
            v-show="activeTab === 'result'"
            class="detail-panel"
          >
            <EaStateBlock
              v-if="isCalling"
              variant="loading"
              :title="t('settings.mcp.toolTester.calling')"
            />

            <EaStateBlock
              v-else-if="!callResult"
              :description="t('settings.mcp.toolTester.noResult')"
            />

            <template v-else>
              <div
                v-if="callResult.success"
                class="result-card"
              >
                <h4>{{ t('settings.mcp.toolTester.resultData') }}</h4>
                <EaJsonViewer :data="callResult.data" />
              </div>

              <div
                v-else
                class="result-card result-card--error"
              >
                <h4>{{ t('settings.mcp.toolTester.errorDetails') }}</h4>
                <EaJsonViewer :data="callResult.error" />
              </div>
            </template>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mcp-test-view {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.mcp-test-view__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
}

.mcp-test-view__title {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
}

.mcp-test-view__content {
  display: grid;
  grid-template-columns: 320px minmax(0, 1fr);
  gap: var(--spacing-4);
  min-height: 560px;
}

.mcp-test-view__tools,
.mcp-test-view__main {
  min-height: 0;
  padding: var(--spacing-4);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.mcp-test-view__tools {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.tools-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.tools-header h4,
.params-header h4,
.result-card h4 {
  margin: 0;
}

.tools-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 24px;
  padding: 0 var(--spacing-2);
  height: 24px;
  border-radius: var(--radius-full);
  background: var(--color-background-secondary);
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}

.tools-list {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--spacing-2);
  overflow-y: auto;
}

.tool-item {
  display: flex;
  gap: var(--spacing-3);
  align-items: flex-start;
  width: 100%;
  padding: var(--spacing-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  text-align: left;
  cursor: pointer;
  transition: border-color 0.2s, background-color 0.2s;
}

.tool-item:hover,
.tool-item--active {
  border-color: var(--color-primary);
  background: var(--color-primary-bg);
}

.tool-item__icon {
  margin-top: 2px;
  color: var(--color-primary);
}

.tool-item__info {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 4px;
}

.tool-item__name {
  font-weight: var(--font-weight-medium);
}

.tool-item__desc {
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
  line-height: 1.5;
}

.mcp-test-view__main {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.detail-tabs {
  display: flex;
  gap: var(--spacing-2);
}

.detail-tab {
  padding: var(--spacing-2) var(--spacing-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  color: var(--color-text-secondary);
  cursor: pointer;
}

.detail-tab--active {
  border-color: var(--color-primary);
  background: var(--color-primary-bg);
  color: var(--color-primary);
}

.detail-panel {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
}

.params-header {
  margin-bottom: var(--spacing-4);
}

.params-header p {
  margin: var(--spacing-2) 0 0;
  color: var(--color-text-secondary);
}

.params-form {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.form-group label {
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.form-group input,
.form-group textarea {
  width: 100%;
  padding: var(--spacing-2) var(--spacing-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-background);
  color: var(--color-text);
}

.required {
  color: var(--color-danger);
}

.param-type {
  margin-left: var(--spacing-1);
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}

.params-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: var(--spacing-4);
}

.result-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.result-card--error h4 {
  color: var(--color-danger);
}

@media (max-width: 960px) {
  .mcp-test-view__content {
    grid-template-columns: 1fr;
  }
}
</style>
