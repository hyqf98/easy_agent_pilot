<script setup lang="ts">
import { EaIcon } from '@/components/common'
import AttachmentThumbnail from '@/components/common/AttachmentThumbnail.vue'
import type { ConversationComposerViewState } from './useConversationComposerView'
import { computed } from 'vue'

type Resolved<T> = T extends { value: infer V } ? V : T
type PendingAttachment = Resolved<ConversationComposerViewState['pendingImages']>[number]

const props = defineProps<{
  attachments: PendingAttachment[]
  main: boolean
  removeAttachment: (attachmentId: string) => void
}>()

const attachmentWrapperStyle = computed(() => {
  if (props.main) {
    return {
      width: '56px',
      height: '56px',
      overflow: 'hidden',
      borderRadius: '12px',
      border: '1px solid rgba(226, 232, 240, 0.92)',
      background: 'rgba(248, 250, 252, 0.94)',
      boxShadow: '0 6px 14px rgba(15, 23, 42, 0.05)'
    }
  }

  return {
    width: '68px',
    height: '68px',
    overflow: 'hidden',
    borderRadius: '14px',
    border: '1px solid color-mix(in srgb, var(--color-border) 72%, transparent)'
  }
})

const attachmentImageStyle = {
  width: '100%',
  height: '100%',
  objectFit: 'cover'
} as const
</script>

<template>
  <div
    v-if="attachments.length > 0"
    class="conversation-composer__attachments"
    :class="{ 'conversation-composer__attachments--main': main }"
  >
    <AttachmentThumbnail
      v-for="attachment in attachments"
      :key="attachment.id"
      :attachment="attachment"
      wrapper-class="conversation-composer__attachment"
      media-class="conversation-composer__attachment-image"
      :wrapper-style="attachmentWrapperStyle"
      :media-style="attachmentImageStyle"
      :preview-max-width="420"
      :preview-max-height="480"
    >
      <button
        type="button"
        class="conversation-composer__attachment-remove"
        @click="removeAttachment(attachment.id)"
      >
        <EaIcon
          name="x"
          :size="12"
        />
      </button>
    </AttachmentThumbnail>
  </div>
</template>

<style scoped>
.conversation-composer__attachments {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.conversation-composer__attachments--main {
  gap: 6px;
  align-items: flex-start;
  padding: 0 0 8px;
}

.conversation-composer__attachment {
  position: relative;
  width: 68px;
  height: 68px;
  overflow: hidden;
  border-radius: 14px;
  border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
}

.conversation-composer__attachments--main .conversation-composer__attachment {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  border-color: rgba(226, 232, 240, 0.92);
  background: rgba(248, 250, 252, 0.94);
  box-shadow: 0 6px 14px rgba(15, 23, 42, 0.05);
}

.conversation-composer__attachment-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.conversation-composer__attachment-remove {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 20px;
  height: 20px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: rgba(15, 23, 42, 0.72);
  color: white;
  border: 0;
  opacity: 0;
  transform: translateY(-2px);
  transition: opacity 0.16s ease, transform 0.16s ease, background-color 0.16s ease;
}

.conversation-composer__attachment:hover .conversation-composer__attachment-remove,
.conversation-composer__attachment:focus-within .conversation-composer__attachment-remove {
  opacity: 1;
  transform: translateY(0);
}

.conversation-composer__attachment-remove:hover {
  background: rgba(15, 23, 42, 0.9);
}

.conversation-composer__attachments--main .conversation-composer__attachment-remove {
  width: 16px;
  height: 16px;
}

:global([data-theme='dark']) .conversation-composer__attachment,
:global(.dark) .conversation-composer__attachment {
  border-color: rgba(71, 85, 105, 0.72);
  background: rgba(15, 23, 42, 0.9);
  box-shadow: none;
}

:global([data-theme='dark']) .conversation-composer__attachment-remove,
:global(.dark) .conversation-composer__attachment-remove {
  background: rgba(2, 6, 23, 0.8);
}

:global([data-theme='dark']) .conversation-composer__attachment-remove:hover,
:global(.dark) .conversation-composer__attachment-remove:hover {
  background: rgba(2, 6, 23, 0.94);
}
</style>
