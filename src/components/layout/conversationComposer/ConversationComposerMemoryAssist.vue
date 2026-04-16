<script setup lang="ts">
import { EaIcon } from '@/components/common'
import type { ConversationComposerViewState } from './useConversationComposerView'

type Resolved<T> = T extends { value: infer V } ? V : T
type MemoryReference = Resolved<ConversationComposerViewState['currentMemoryReferences']>[number]
type MemoryPreview = Resolved<ConversationComposerViewState['currentMemoryPreview']>
type VisibleMemorySuggestions = Resolved<ConversationComposerViewState['visibleMemorySuggestions']>
type LibrarySuggestion = VisibleMemorySuggestions['librarySuggestions'][number]
type RawSuggestion = VisibleMemorySuggestions['rawSuggestions'][number]
type MemorySuggestion = LibrarySuggestion | RawSuggestion

defineProps<{
  t: ConversationComposerViewState['t']
  isMainPanel: boolean
  currentMemoryReferences: MemoryReference[]
  currentMemoryPreview: MemoryPreview
  shouldShowMemorySuggestions: boolean
  hasVisibleMemorySuggestions: boolean
  isMemorySuggestionLoading: boolean
  shouldShowMemorySuggestionEmptyState: boolean
  shouldShowMemorySuggestionIdleHint: boolean
  visibleMemorySuggestions: VisibleMemorySuggestions
  isActiveMemorySuggestion: (suggestion: MemorySuggestion) => boolean
  previewMemoryReference: (reference: MemoryReference) => void
  previewMemorySuggestion: (suggestion: MemorySuggestion) => void
  clearMemoryPreview: () => void
  handleMemorySuggestionPointerEnter: () => void
  handleMemorySuggestionPointerLeave: () => void
  dismissMemorySuggestion: (suggestion: MemorySuggestion) => void
  insertMemoryReference: (suggestion: MemorySuggestion) => void
  removeMemoryReferenceFromDraft: (reference: MemoryReference) => void
}>()
</script>

<template>
  <div class="conversation-composer__memory-assist">
    <div
      v-if="isMainPanel && currentMemoryReferences.length > 0"
      class="conversation-composer__memory-tray"
    >
      <div class="conversation-composer__memory-tray-label">
        {{ t('message.memoryReferencesTitle') }}
      </div>
      <div class="conversation-composer__memory-chips">
        <button
          v-for="reference in currentMemoryReferences"
          :key="`${reference.sourceType}:${reference.sourceId}`"
          class="conversation-composer__memory-chip"
          type="button"
          :title="reference.fullContent"
          @mouseenter="previewMemoryReference(reference)"
          @mouseleave="clearMemoryPreview"
          @focus="previewMemoryReference(reference)"
          @blur="clearMemoryPreview"
          @click="removeMemoryReferenceFromDraft(reference)"
        >
          <span class="conversation-composer__memory-chip-type">
            {{ reference.sourceType === 'library_chunk' ? t('message.memorySourceLibrary') : t('message.memorySourceRaw') }}
          </span>
          <span class="conversation-composer__memory-chip-text">{{ reference.title }}</span>
          <EaIcon
            name="x"
            :size="12"
          />
        </button>
      </div>
    </div>

    <div
      v-if="isMainPanel && currentMemoryPreview && !shouldShowMemorySuggestions"
      class="conversation-composer__memory-preview"
      :class="{
        'conversation-composer__memory-preview--library': currentMemoryPreview.sourceType === 'library_chunk',
        'conversation-composer__memory-preview--raw': currentMemoryPreview.sourceType === 'raw_record'
      }"
    >
      <div class="conversation-composer__memory-preview-header">
        <span class="conversation-composer__memory-preview-label">
          {{ t('message.memoryPreviewTitle') }}
        </span>
        <span class="conversation-composer__memory-preview-source">
          {{ currentMemoryPreview.sourceLabel }}
        </span>
      </div>
      <div class="conversation-composer__memory-preview-name">
        {{ currentMemoryPreview.title }}
      </div>
      <pre class="conversation-composer__memory-preview-content">{{ currentMemoryPreview.fullContent }}</pre>
    </div>

    <div
      v-if="isMainPanel && shouldShowMemorySuggestions"
      class="conversation-composer__memory-panel conversation-composer__memory-panel--floating"
      @mouseenter="handleMemorySuggestionPointerEnter"
      @mouseleave="handleMemorySuggestionPointerLeave"
    >
      <div class="conversation-composer__memory-panel-header">
        <div>
          <div class="conversation-composer__memory-eyebrow">
            {{ t('message.memorySuggestionEyebrow') }}
          </div>
          <div class="conversation-composer__memory-title">
            {{ hasVisibleMemorySuggestions ? t('message.memorySuggestionTitle') : t('message.memorySearchingActive') }}
          </div>
          <div
            v-if="hasVisibleMemorySuggestions"
            class="conversation-composer__memory-keyboard-hint"
          >
            {{ t('message.memoryKeyboardHint') }}
          </div>
        </div>
        <div
          v-if="isMemorySuggestionLoading"
          class="conversation-composer__memory-loading"
        >
          <span class="conversation-composer__memory-spinner" />
          <span>{{ t('message.memorySearchingActive') }}</span>
        </div>
      </div>

      <div
        v-if="currentMemoryPreview"
        class="conversation-composer__memory-preview"
        :class="{
          'conversation-composer__memory-preview--library': currentMemoryPreview.sourceType === 'library_chunk',
          'conversation-composer__memory-preview--raw': currentMemoryPreview.sourceType === 'raw_record'
        }"
      >
        <div class="conversation-composer__memory-preview-header">
          <span class="conversation-composer__memory-preview-label">
            {{ t('message.memoryPreviewTitle') }}
          </span>
          <span class="conversation-composer__memory-preview-source">
            {{ currentMemoryPreview.sourceLabel }}
          </span>
        </div>
        <div class="conversation-composer__memory-preview-name">
          {{ currentMemoryPreview.title }}
        </div>
        <pre class="conversation-composer__memory-preview-content">{{ currentMemoryPreview.fullContent }}</pre>
      </div>

      <div
        v-if="shouldShowMemorySuggestionEmptyState"
        class="conversation-composer__memory-empty"
      >
        <div class="conversation-composer__memory-empty-title">
          {{ t('message.memoryNoMatches') }}
        </div>
        <div class="conversation-composer__memory-empty-hint">
          {{ t('message.memoryKeepTypingHint') }}
        </div>
      </div>

      <div
        v-else-if="shouldShowMemorySuggestionIdleHint"
        class="conversation-composer__memory-empty conversation-composer__memory-empty--subtle"
      >
        <div class="conversation-composer__memory-empty-hint">
          {{ t('message.memorySearchSettling') }}
        </div>
      </div>

      <div
        v-if="hasVisibleMemorySuggestions && visibleMemorySuggestions.librarySuggestions.length > 0"
        class="conversation-composer__memory-group"
      >
        <div class="conversation-composer__memory-group-title">
          {{ t('message.memorySourceLibrary') }}
        </div>
        <div class="conversation-composer__memory-list">
          <article
            v-for="suggestion in visibleMemorySuggestions.librarySuggestions"
            :key="`${suggestion.sourceType}:${suggestion.sourceId}`"
            class="conversation-composer__memory-card conversation-composer__memory-card--library"
            :class="{ 'conversation-composer__memory-card--active': isActiveMemorySuggestion(suggestion) }"
            role="option"
            :aria-selected="isActiveMemorySuggestion(suggestion)"
            :title="suggestion.fullContent"
            @mouseenter="previewMemorySuggestion(suggestion)"
            @mouseleave="clearMemoryPreview"
          >
            <div class="conversation-composer__memory-card-body">
              <div class="conversation-composer__memory-card-top">
                <span class="conversation-composer__memory-badge">
                  {{ t('message.memorySourceLibrary') }}
                </span>
                <span class="conversation-composer__memory-card-title">{{ suggestion.title }}</span>
              </div>
              <p class="conversation-composer__memory-card-snippet">
                {{ suggestion.snippet || suggestion.fullContent }}
              </p>
            </div>
            <div class="conversation-composer__memory-card-actions">
              <button
                class="conversation-composer__memory-action conversation-composer__memory-action--ghost"
                type="button"
                @click="dismissMemorySuggestion(suggestion)"
              >
                {{ t('message.memoryDismiss') }}
              </button>
              <button
                class="conversation-composer__memory-action conversation-composer__memory-action--primary"
                type="button"
                @click="insertMemoryReference(suggestion)"
              >
                {{ t('message.memoryInsert') }}
              </button>
            </div>
          </article>
        </div>
      </div>

      <div
        v-if="hasVisibleMemorySuggestions && visibleMemorySuggestions.rawSuggestions.length > 0"
        class="conversation-composer__memory-group"
      >
        <div class="conversation-composer__memory-group-title">
          {{ t('message.memorySourceRaw') }}
        </div>
        <div class="conversation-composer__memory-list">
          <article
            v-for="suggestion in visibleMemorySuggestions.rawSuggestions"
            :key="`${suggestion.sourceType}:${suggestion.sourceId}`"
            class="conversation-composer__memory-card conversation-composer__memory-card--raw"
            :class="{ 'conversation-composer__memory-card--active': isActiveMemorySuggestion(suggestion) }"
            role="option"
            :aria-selected="isActiveMemorySuggestion(suggestion)"
            :title="suggestion.fullContent"
            @mouseenter="previewMemorySuggestion(suggestion)"
            @mouseleave="clearMemoryPreview"
          >
            <div class="conversation-composer__memory-card-body">
              <div class="conversation-composer__memory-card-top">
                <span class="conversation-composer__memory-badge conversation-composer__memory-badge--raw">
                  {{ t('message.memorySourceRaw') }}
                </span>
                <span class="conversation-composer__memory-card-title">{{ suggestion.title }}</span>
              </div>
              <p class="conversation-composer__memory-card-snippet">
                {{ suggestion.snippet || suggestion.fullContent }}
              </p>
            </div>
            <div class="conversation-composer__memory-card-actions">
              <button
                class="conversation-composer__memory-action conversation-composer__memory-action--ghost"
                type="button"
                @click="dismissMemorySuggestion(suggestion)"
              >
                {{ t('message.memoryDismiss') }}
              </button>
              <button
                class="conversation-composer__memory-action conversation-composer__memory-action--primary"
                type="button"
                @click="insertMemoryReference(suggestion)"
              >
                {{ t('message.memoryInsert') }}
              </button>
            </div>
          </article>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped src="./styles.css"></style>
