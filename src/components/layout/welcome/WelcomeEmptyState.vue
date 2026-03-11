<script setup lang="ts">
import { EaIcon } from '@/components/common'
import type { WelcomeAction, WelcomeFeature } from './welcomeShared'

defineProps<{
  quickActions: WelcomeAction[]
  features: WelcomeFeature[]
}>()
</script>

<template>
  <div class="welcome-empty-state">
    <div class="welcome-header">
      <div class="welcome-logo">
        <div class="welcome-logo__icon">
          <EaIcon
            name="bot"
            :size="48"
          />
        </div>
        <div class="welcome-logo__pulse" />
      </div>
      <h1 class="welcome-title">
        <span class="welcome-title__brand">Easy Agent Pilot</span>
      </h1>
      <p class="welcome-subtitle">
        您的 AI 编程助手，让开发更高效
      </p>
    </div>

    <div class="welcome-actions">
      <div
        v-for="(action, index) in quickActions"
        :key="index"
        class="action-card"
        :style="{ '--delay': `${0.2 + index * 0.1}s` }"
        @click="action.action"
      >
        <div class="action-card__icon">
          <EaIcon
            :name="action.icon"
            :size="24"
          />
        </div>
        <div class="action-card__content">
          <div class="action-card__title">
            {{ action.title }}
            <span
              v-if="action.shortcut"
              class="action-card__shortcut"
            >
              {{ action.shortcut }}
            </span>
          </div>
          <div class="action-card__description">
            {{ action.description }}
          </div>
        </div>
        <div class="action-card__arrow">
          <EaIcon
            name="arrow-right"
            :size="16"
          />
        </div>
      </div>
    </div>

    <div class="welcome-features">
      <div
        v-for="(feature, index) in features"
        :key="index"
        class="feature-item"
        :style="{ '--delay': `${0.5 + index * 0.1}s` }"
      >
        <div class="feature-item__icon">
          <EaIcon
            :name="feature.icon"
            :size="20"
          />
        </div>
        <div class="feature-item__text">
          <div class="feature-item__title">
            {{ feature.title }}
          </div>
          <div class="feature-item__desc">
            {{ feature.description }}
          </div>
        </div>
      </div>
    </div>

    <div
      class="welcome-footer"
      :style="{ '--delay': '0.9s' }"
    >
      <div class="welcome-footer__hint">
        <EaIcon
          name="keyboard"
          :size="14"
        />
        <span>按 <kbd>⌘</kbd> + <kbd>N</kbd> 快速创建项目</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.welcome-empty-state {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.welcome-header {
  text-align: center;
  margin-bottom: var(--spacing-8);
  --delay: 0s;
}

.welcome-logo {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  margin-bottom: var(--spacing-4);
}

.welcome-logo__icon {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 80px;
  height: 80px;
  background: linear-gradient(
    135deg,
    var(--color-primary) 0%,
    var(--color-accent) 100%
  );
  border-radius: var(--radius-2xl);
  color: white;
  box-shadow:
    0 8px 32px -8px rgba(59, 130, 246, 0.4),
    inset 0 1px 0 rgba(255, 255, 255, 0.2);
}

.welcome-logo__pulse {
  position: absolute;
  inset: -8px;
  border-radius: var(--radius-2xl);
  border: 2px solid var(--color-primary);
  opacity: 0;
  animation: welcome-logo-pulse 2s ease-out infinite;
}

.welcome-title {
  margin: 0 0 var(--spacing-2);
  font-size: 28px;
  font-weight: 700;
  line-height: 1.2;
  letter-spacing: -0.02em;
  color: var(--color-text-primary);
}

.welcome-title__brand {
  background: linear-gradient(
    135deg,
    var(--color-text-primary) 0%,
    var(--color-primary) 50%,
    var(--color-accent) 100%
  );
  background-size: 200% 200%;
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  animation: welcome-gradient-shift 4s ease infinite;
}

.welcome-subtitle {
  margin: 0;
  font-size: var(--font-size-base);
  color: var(--color-text-secondary);
  font-weight: 400;
}

.welcome-actions {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  width: 100%;
  max-width: 480px;
  margin-bottom: var(--spacing-10);
}

.action-card {
  display: flex;
  align-items: center;
  gap: var(--spacing-4);
  padding: var(--spacing-4) var(--spacing-5);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  cursor: pointer;
  transition:
    background-color var(--transition-fast) var(--easing-default),
    border-color var(--transition-fast) var(--easing-default),
    transform var(--transition-fast) var(--easing-default),
    box-shadow var(--transition-fast) var(--easing-default);
}

.action-card:hover {
  background: var(--color-surface-hover);
  border-color: var(--color-primary);
  transform: translateX(4px);
  box-shadow: var(--shadow-md);
}

.action-card__icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  background: var(--color-primary-light);
  border-radius: var(--radius-lg);
  color: var(--color-primary);
  transition:
    background-color var(--transition-fast) var(--easing-default),
    transform var(--transition-fast) var(--easing-default);
}

.action-card:hover .action-card__icon {
  background: var(--color-primary);
  color: white;
  transform: scale(1.05);
}

[data-theme='dark'] .action-card__icon {
  background: rgba(96, 165, 250, 0.15);
}

[data-theme='dark'] .action-card:hover .action-card__icon {
  background: var(--color-primary);
}

.action-card__content {
  flex: 1;
  min-width: 0;
}

.action-card__title {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.action-card__shortcut {
  font-size: var(--font-size-xs);
  padding: 2px 6px;
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
  font-family: var(--font-family-mono);
}

.action-card__description {
  margin-top: 2px;
  font-size: var(--font-size-sm);
  color: var(--color-text-tertiary);
}

.action-card__arrow {
  flex-shrink: 0;
  color: var(--color-text-tertiary);
  opacity: 0;
  transform: translateX(-8px);
  transition:
    opacity var(--transition-fast) var(--easing-default),
    transform var(--transition-fast) var(--easing-default);
}

.action-card:hover .action-card__arrow {
  opacity: 1;
  transform: translateX(0);
}

.welcome-features {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--spacing-4);
  width: 100%;
  max-width: 600px;
  margin-bottom: var(--spacing-8);
}

.feature-item {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-3);
  padding: var(--spacing-3) var(--spacing-4);
  background: var(--color-bg-secondary);
  border-radius: var(--radius-lg);
  transition:
    background-color var(--transition-fast) var(--easing-default),
    transform var(--transition-fast) var(--easing-default);
}

.feature-item:hover {
  background: var(--color-surface-hover);
  transform: translateY(-2px);
}

.feature-item__icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  background: var(--color-surface);
  border-radius: var(--radius-md);
  color: var(--color-primary);
}

.feature-item__text {
  flex: 1;
  min-width: 0;
}

.feature-item__title {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
  margin-bottom: 2px;
}

.feature-item__desc {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  line-height: 1.4;
}

.welcome-footer {
  --delay: 0.9s;
}

.welcome-footer__hint {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-sm);
  color: var(--color-text-tertiary);
}

.welcome-footer__hint kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 20px;
  height: 20px;
  padding: 0 6px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  box-shadow: 0 1px 0 var(--color-border);
}

@keyframes welcome-logo-pulse {
  0% {
    opacity: 0.6;
    transform: scale(1);
  }
  100% {
    opacity: 0;
    transform: scale(1.2);
  }
}

@keyframes welcome-gradient-shift {
  0%, 100% {
    background-position: 0% 50%;
  }
  50% {
    background-position: 100% 50%;
  }
}

@media (max-width: 640px) {
  .welcome-title {
    font-size: 24px;
  }

  .welcome-features {
    grid-template-columns: 1fr;
  }

  .welcome-logo__icon {
    width: 64px;
    height: 64px;
  }
}
</style>
