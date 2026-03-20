<script setup lang="ts">
import { ref } from 'vue'
import { EaIcon } from '@/components/common'
import ProviderSwitch from './ProviderSwitch.vue'
import AgentSettings from './AgentSettings.vue'

const activeTab = ref<'channels' | 'runtimes'>('channels')
</script>

<template>
  <div class="model-management">
    <div class="model-management__header">
      <div>
        <h2 class="model-management__title">
          模型管理
        </h2>
        <p class="model-management__subtitle">
          统一管理上游模型渠道、Provider Profile 与运行时模型配置。
        </p>
      </div>
    </div>

    <div class="model-management__tabs">
      <button
        class="model-management__tab"
        :class="{ 'model-management__tab--active': activeTab === 'channels' }"
        @click="activeTab = 'channels'"
      >
        <EaIcon
          name="repeat"
          :size="16"
        />
        渠道配置
      </button>
      <button
        class="model-management__tab"
        :class="{ 'model-management__tab--active': activeTab === 'runtimes' }"
        @click="activeTab = 'runtimes'"
      >
        <EaIcon
          name="cpu"
          :size="16"
        />
        运行配置
      </button>
    </div>

    <div class="model-management__content">
      <ProviderSwitch v-if="activeTab === 'channels'" />
      <AgentSettings v-else />
    </div>
  </div>
</template>

<style scoped>
.model-management {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  min-width: 0;
}

.model-management__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-4);
}

.model-management__title {
  margin: 0;
  font-size: 22px;
  font-weight: 700;
  color: var(--color-text-primary);
}

.model-management__subtitle {
  margin: var(--spacing-2) 0 0;
  color: var(--color-text-secondary);
  font-size: 14px;
}

.model-management__tabs {
  display: flex;
  gap: var(--spacing-2);
  flex-wrap: wrap;
}

.model-management__tab {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: 10px 14px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  color: var(--color-text-secondary);
}

.model-management__tab--active {
  border-color: var(--color-primary);
  background: var(--color-primary-light);
  color: var(--color-primary);
}

.model-management__content {
  min-width: 0;
}
</style>
