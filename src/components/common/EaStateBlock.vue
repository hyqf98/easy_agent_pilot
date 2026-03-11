<script setup lang="ts">
import { computed } from 'vue'
import EaIcon from './EaIcon.vue'

type StateVariant = 'loading' | 'error' | 'empty' | 'success'

interface Props {
  variant?: StateVariant
  title?: string
  description?: string
  icon?: string
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'empty',
  title: '',
  description: '',
  icon: ''
})

const resolvedIcon = computed(() => {
  if (props.icon) {
    return props.icon
  }

  if (props.variant === 'loading') {
    return 'loader'
  }

  if (props.variant === 'error') {
    return 'alert-circle'
  }

  if (props.variant === 'success') {
    return 'check-circle'
  }

  return 'inbox'
})
</script>

<template>
  <div
    class="ea-state-block"
    :class="`ea-state-block--${variant}`"
  >
    <EaIcon
      :name="resolvedIcon"
      :size="20"
      :spin="variant === 'loading'"
      class="ea-state-block__icon"
    />
    <div class="ea-state-block__content">
      <div
        v-if="title"
        class="ea-state-block__title"
      >
        {{ title }}
      </div>
      <div
        v-if="description"
        class="ea-state-block__description"
      >
        {{ description }}
      </div>
      <slot />
    </div>
    <div
      v-if="$slots.actions"
      class="ea-state-block__actions"
    >
      <slot name="actions" />
    </div>
  </div>
</template>

<style scoped>
.ea-state-block {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-3);
  padding: var(--spacing-5);
  border: 1px dashed var(--color-border);
  border-radius: var(--radius-lg);
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--color-bg-secondary) 92%, white 8%) 0%, var(--color-bg-secondary) 100%);
  text-align: center;
}

.ea-state-block__icon {
  flex-shrink: 0;
}

.ea-state-block__content {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-width: 420px;
}

.ea-state-block__title {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
}

.ea-state-block__description {
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
  line-height: 1.5;
}

.ea-state-block__actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.ea-state-block--loading {
  color: var(--color-primary);
}

.ea-state-block--error {
  border-color: color-mix(in srgb, var(--color-error) 25%, var(--color-border) 75%);
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--color-error-light) 35%, var(--color-bg-secondary) 65%) 0%, var(--color-bg-secondary) 100%);
  color: var(--color-error);
}

.ea-state-block--empty {
  color: var(--color-text-tertiary);
}

.ea-state-block--success {
  border-color: color-mix(in srgb, var(--color-success) 24%, var(--color-border) 76%);
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--color-success-light) 45%, var(--color-bg-secondary) 55%) 0%, var(--color-bg-secondary) 100%);
  color: var(--color-success);
}
</style>
