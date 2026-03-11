<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { McpConfigScope, McpTransportType, UnifiedMcpConfig } from '@/stores/skillConfig'
import { EaButton, EaIcon } from '@/components/common'

interface KeyValueItem {
  key: string
  value: string
}

const props = defineProps<{
  config: UnifiedMcpConfig
}>()

const emit = defineEmits<{
  back: []
  save: [config: Partial<UnifiedMcpConfig>, originalId?: string]
}>()

const { t } = useI18n()
const isCreating = computed(() => !props.config.id)

const form = ref({
  name: '',
  transportType: 'stdio' as McpTransportType,
  scope: 'user' as McpConfigScope,
  command: '',
  args: '',
  envItems: [] as KeyValueItem[],
  url: '',
  headerItems: [] as KeyValueItem[]
})

const transportOptions: Array<{ label: string; value: McpTransportType }> = [
  { label: '(STDIO) 标准输入输出', value: 'stdio' },
  { label: '(SSE) 服务器推送事件', value: 'sse' },
  { label: '(HTTP) HTTP 请求', value: 'http' }
]

function toItems(record?: Record<string, string>): KeyValueItem[] {
  if (!record) return []
  return Object.entries(record).map(([key, value]) => ({ key, value }))
}

function toRecord(items: KeyValueItem[]): Record<string, string> | undefined {
  const record = items.reduce<Record<string, string>>((acc, item) => {
    if (item.key.trim()) {
      acc[item.key.trim()] = item.value
    }
    return acc
  }, {})

  return Object.keys(record).length > 0 ? record : undefined
}

function syncForm(config: UnifiedMcpConfig) {
  form.value = {
    name: config.name,
    transportType: config.transportType,
    scope: config.scope,
    command: config.command || '',
    args: config.args?.join('\n') || '',
    envItems: toItems(config.env),
    url: config.url || '',
    headerItems: toItems(config.headers)
  }
}

function addEnvItem() {
  form.value.envItems.push({ key: '', value: '' })
}

function removeEnvItem(index: number) {
  form.value.envItems.splice(index, 1)
}

function addHeaderItem() {
  form.value.headerItems.push({ key: '', value: '' })
}

function removeHeaderItem(index: number) {
  form.value.headerItems.splice(index, 1)
}

function handleSave() {
  emit('save', {
    name: form.value.name,
    transportType: form.value.transportType,
    scope: form.value.scope,
    command: form.value.command || undefined,
    args: form.value.args ? form.value.args.split('\n').filter(Boolean) : undefined,
    env: toRecord(form.value.envItems),
    url: form.value.url || undefined,
    headers: toRecord(form.value.headerItems)
  }, props.config.id)
}

watch(() => props.config, syncForm, { immediate: true })
</script>

<template>
  <div class="mcp-edit-view">
    <div class="mcp-edit-view__header">
      <EaButton
        variant="ghost"
        size="small"
        @click="emit('back')"
      >
        <EaIcon name="lucide:arrow-left" />
        {{ t('common.back') }}
      </EaButton>
      <div class="mcp-edit-view__title">
        <EaIcon name="lucide:pencil" />
        <span>
          {{ isCreating ? t('settings.sdkConfig.mcp.add') : t('settings.sdkConfig.mcp.edit') }}
          <template v-if="config.name">
            : {{ config.name }}
          </template>
        </span>
      </div>
    </div>

    <div class="mcp-edit-view__card">
      <div class="form-group">
        <label>{{ t('settings.sdkConfig.mcp.name') }}</label>
        <input
          v-model="form.name"
          type="text"
          :placeholder="t('settings.sdkConfig.mcp.namePlaceholder')"
        >
      </div>

      <div class="form-row">
        <div class="form-group">
          <label class="label-with-icon">
            <EaIcon
              name="lucide:plug"
              class="label-icon"
            />
            {{ t('settings.sdkConfig.mcp.transportType') }}
          </label>
          <select v-model="form.transportType">
            <option
              v-for="option in transportOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </option>
          </select>
        </div>

        <div class="form-group">
          <label class="label-with-icon">
            <EaIcon
              name="lucide:map-pin"
              class="label-icon"
            />
            {{ t('settings.sdkConfig.mcp.scope') }}
          </label>
          <select v-model="form.scope">
            <option value="user">
              {{ t('settings.agent.scan.scopeTypes.user') }}
            </option>
            <option value="local">
              {{ t('settings.agent.scan.scopeTypes.local') }}
            </option>
            <option value="project">
              {{ t('settings.agent.scan.scopeTypes.project') }}
            </option>
          </select>
        </div>
      </div>

      <template v-if="form.transportType === 'stdio'">
        <div class="form-group">
          <label>{{ t('settings.sdkConfig.mcp.command') }}</label>
          <input
            v-model="form.command"
            type="text"
            :placeholder="t('settings.sdkConfig.mcp.commandPlaceholder')"
          >
        </div>

        <div class="form-group">
          <label>{{ t('settings.sdkConfig.mcp.args') }}</label>
          <textarea
            v-model="form.args"
            :placeholder="t('settings.sdkConfig.mcp.argsPlaceholder')"
            rows="3"
          />
        </div>

        <div class="form-group">
          <label class="label-with-icon">
            <EaIcon
              name="lucide:variable"
              class="label-icon"
            />
            {{ t('settings.sdkConfig.mcp.env') }}
          </label>
          <div class="kv-list">
            <div
              v-for="(item, index) in form.envItems"
              :key="index"
              class="kv-item"
            >
              <input
                v-model="item.key"
                type="text"
                placeholder="KEY"
                class="kv-item__key"
              >
              <span class="kv-item__separator">=</span>
              <input
                v-model="item.value"
                type="text"
                placeholder="value"
                class="kv-item__value"
              >
              <button
                type="button"
                class="kv-item__remove"
                @click="removeEnvItem(index)"
              >
                <EaIcon name="lucide:x" />
              </button>
            </div>

            <button
              type="button"
              class="kv-list__add"
              @click="addEnvItem"
            >
              <EaIcon name="lucide:plus" />
              {{ t('settings.sdkConfig.mcp.addEnvVar') }}
            </button>
          </div>
        </div>
      </template>

      <template v-else>
        <div class="form-group">
          <label>{{ t('settings.sdkConfig.mcp.url') }}</label>
          <input
            v-model="form.url"
            type="text"
            :placeholder="t('settings.sdkConfig.mcp.urlPlaceholder')"
          >
        </div>

        <div class="form-group">
          <label class="label-with-icon">
            <EaIcon
              name="lucide:file-text"
              class="label-icon"
            />
            {{ t('settings.sdkConfig.mcp.headers') }}
          </label>
          <div class="kv-list">
            <div
              v-for="(item, index) in form.headerItems"
              :key="index"
              class="kv-item"
            >
              <input
                v-model="item.key"
                type="text"
                placeholder="Header Name"
                class="kv-item__key"
              >
              <span class="kv-item__separator">:</span>
              <input
                v-model="item.value"
                type="text"
                placeholder="Header Value"
                class="kv-item__value"
              >
              <button
                type="button"
                class="kv-item__remove"
                @click="removeHeaderItem(index)"
              >
                <EaIcon name="lucide:x" />
              </button>
            </div>

            <button
              type="button"
              class="kv-list__add"
              @click="addHeaderItem"
            >
              <EaIcon name="lucide:plus" />
              {{ t('settings.sdkConfig.mcp.addHeader') }}
            </button>
          </div>
        </div>
      </template>

      <div class="mcp-edit-view__actions">
        <EaButton
          variant="ghost"
          @click="emit('back')"
        >
          {{ t('common.cancel') }}
        </EaButton>
        <EaButton @click="handleSave">
          {{ t('common.save') }}
        </EaButton>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mcp-edit-view {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.mcp-edit-view__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
}

.mcp-edit-view__title {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
}

.mcp-edit-view__card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  padding: var(--spacing-4);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
}

.form-row {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--spacing-4);
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.form-group label {
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.label-with-icon {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
}

.label-icon {
  width: 14px;
  height: 14px;
  color: var(--color-primary);
}

.form-group input,
.form-group textarea,
.form-group select {
  width: 100%;
  padding: var(--spacing-2) var(--spacing-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-background);
  color: var(--color-text);
  font-size: var(--font-size-sm);
  transition: border-color 0.2s, box-shadow 0.2s;
}

.form-group input:focus,
.form-group textarea:focus,
.form-group select:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px var(--color-primary-bg);
}

.kv-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.kv-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr) auto;
  gap: var(--spacing-2);
  align-items: center;
}

.kv-item__separator {
  color: var(--color-text-tertiary);
}

.kv-item__remove,
.kv-list__add {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-1);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  cursor: pointer;
}

.kv-item__remove {
  width: 36px;
  height: 36px;
  color: var(--color-text-tertiary);
}

.kv-list__add {
  align-self: flex-start;
  padding: var(--spacing-2) var(--spacing-3);
  color: var(--color-primary);
}

.mcp-edit-view__actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-2);
}

@media (max-width: 720px) {
  .form-row {
    grid-template-columns: 1fr;
  }

  .kv-item {
    grid-template-columns: 1fr;
  }

  .kv-item__separator {
    display: none;
  }
}
</style>
