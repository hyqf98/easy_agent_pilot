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
      <span
        v-if="segment.type === 'text'"
        class="conversation-composer__text"
      >{{ segment.content }}</span>
      <span
        v-else-if="segment.type === 'file'"
        class="conversation-composer__file-tag"
        :title="segment.titleContent"
      >{{ segment.displayContent || segment.content }}</span>
      <span
        v-else-if="segment.type === 'memory'"
        class="conversation-composer__memory-tag"
        :title="segment.titleContent"
      >{{ segment.displayContent || segment.content }}</span>
      <span
        v-else
        class="conversation-composer__slash-tag"
      >{{ segment.content }}</span>
    </template>
  </template>

  <div
    v-if="isMainPanel && !inputText"
    class="conversation-composer__ghost-hints"
  >
    <span class="conversation-composer__ghost-hint-pill">
      <EaIcon
        name="image-up"
        :size="11"
      />
      <span>{{ t('message.ghostHintImages') }}</span>
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
