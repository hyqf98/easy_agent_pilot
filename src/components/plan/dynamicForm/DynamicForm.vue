<script setup lang="ts">
import type { DynamicFormSchema } from '@/types/plan'
import { useDynamicForm, type DynamicFormEmits } from './useDynamicForm'

const props = defineProps<{
  schema: DynamicFormSchema
  question?: string
  initialValues?: Record<string, any>
  disabled?: boolean
  variant?: 'active' | 'submitted' | 'archived'
  cancelText?: string
  showHeader?: boolean
  showSubmittedState?: boolean
}>()

const emit = defineEmits<DynamicFormEmits>()

const {
  isDarkTheme,
  formValues,
  visibleFields,
  showActions,
  getFieldComponent,
  updateFieldValue,
  getFieldError,
  handleSubmit,
  handleCancel,
  validateForm,
  resetForm,
  getValues,
  setValues
} = useDynamicForm(props, emit)

defineExpose({
  validateForm,
  resetForm,
  getValues,
  setValues
})
</script>

<template>
  <div
    class="dynamic-form dynamic-form--compact"
    :class="[
      `dynamic-form--${props.variant || 'active'}`,
      { 'dynamic-form--headerless': props.showHeader === false },
      { 'dynamic-form--dark': isDarkTheme }
    ]"
  >
    <div
      v-if="props.showHeader !== false"
      class="form-header"
    >
      <h3 class="form-title">
        {{ schema.title }}
      </h3>
      <p
        v-if="question"
        class="form-question"
      >
        {{ question }}
      </p>
      <span
        v-if="props.variant === 'submitted' && props.showSubmittedState !== false"
        class="form-state-badge"
      >
        已提交
      </span>
      <p
        v-if="schema.description"
        class="form-description"
      >
        {{ schema.description }}
      </p>
    </div>

    <form
      class="form-body"
      @submit.prevent="handleSubmit"
    >
      <template
        v-for="field in visibleFields"
        :key="field.name"
      >
        <component
          :is="getFieldComponent(field.type)"
          :field="field"
          :model-value="formValues[field.name]"
          :error="getFieldError(field.name)"
          :disabled="disabled"
          @update:model-value="updateFieldValue(field.name, $event)"
        />
      </template>
    </form>

    <div
      v-if="showActions"
      class="form-footer"
    >
      <button
        type="button"
        class="btn btn-secondary"
        :disabled="disabled"
        @click="handleCancel"
      >
        {{ props.cancelText || '取消' }}
      </button>
      <button
        type="button"
        class="btn btn-primary"
        :disabled="disabled"
        @click="handleSubmit"
      >
        {{ schema.submitText || '提交' }}
      </button>
    </div>
  </div>
</template>

<style scoped src="./styles.css"></style>
