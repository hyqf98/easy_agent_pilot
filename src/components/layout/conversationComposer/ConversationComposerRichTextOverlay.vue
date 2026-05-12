<script setup lang="ts">
import { EaIcon } from '@/components/common'
import type { ConversationComposerViewState } from './useConversationComposerView'

type Resolved<T> = T extends { value: infer V } ? V : T
type ParsedInputSegment = Resolved<ConversationComposerViewState['parsedInputText']>[number]

defineProps<{
  t: ConversationComposerViewState['t']
  parsedInputText: ParsedInputSegment[]
  shouldUseRichTextOverlay: boolean
  isMainPanel: boolean
  inputText: string
  composerSendShortcutHint: string
}>()
</script>

<template>
  <template v-if="parsedInputText.length > 0">
    <template
      v-for="(segment, index) in parsedInputText"
      :key="index"
    >
      <template v-if="segment.type === 'text'">
        <span class="conversation-composer__text">{{ segment.content }}</span>
      </template>
      <template v-else-if="segment.type === 'file'">
        <span
          class="conversation-composer__file-tag"
          :title="segment.titleContent"
        >{{ segment.displayContent || segment.content }}</span>
        <span
          v-if="segment.trailingSpace"
          class="conversation-composer__token-gap"
        >&nbsp;</span>
      </template>
      <template v-else-if="segment.type === 'memory'">
        <span
          class="conversation-composer__memory-tag"
          :title="segment.titleContent"
        >
          <span class="conversation-composer__memory-tag-ghost">{{ segment.content }}</span>
          <span class="conversation-composer__memory-tag-surface">
            <span
              v-if="segment.memorySourceLabel"
              class="conversation-composer__memory-tag-kind"
            >{{ segment.memorySourceLabel }}</span>
            <span class="conversation-composer__memory-tag-text">{{ segment.displayContent || segment.content }}</span>
          </span>
        </span>
      </template>
      <template v-else-if="segment.type === 'attachment'">
        <span
          class="conversation-composer__attachment-placeholder"
          :class="{
            'conversation-composer__attachment-placeholder--image': segment.attachmentType === 'image',
            'conversation-composer__attachment-placeholder--file': segment.attachmentType === 'file'
          }"
        >{{ segment.content }}</span>
        <span
          v-if="segment.trailingSpace"
          class="conversation-composer__token-gap"
        >&nbsp;</span>
      </template>
      <template v-else>
        <span
          class="conversation-composer__slash-tag"
        >{{ segment.content }}</span>
        <span
          v-if="segment.trailingSpace"
          class="conversation-composer__token-gap"
        >&nbsp;</span>
      </template>
    </template>
  </template>

  <div
    v-if="isMainPanel && !inputText"
    class="conversation-composer__ghost-hints"
  >
    <span class="conversation-composer__ghost-hint-pill">
      <EaIcon
        name="paperclip"
        :size="11"
      />
      <span>{{ t('message.ghostHintAttachments') }}</span>
    </span>
    <span class="conversation-composer__ghost-hint-pill">
      <EaIcon
        name="at-sign"
        :size="11"
      />
      <span>{{ t('message.ghostHintFiles') }}</span>
    </span>
    <span class="conversation-composer__ghost-hint-pill">
      <EaIcon
        name="corner-down-left"
        :size="11"
      />
      <span>{{ t('message.ghostHintSend', { shortcut: composerSendShortcutHint }) }}</span>
    </span>
  </div>
</template>
