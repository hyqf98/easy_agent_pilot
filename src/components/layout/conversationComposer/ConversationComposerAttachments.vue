<script setup lang="ts">
import { EaIcon } from '@/components/common'
import type { ConversationComposerViewState } from './useConversationComposerView'

type Resolved<T> = T extends { value: infer V } ? V : T
type PendingImage = Resolved<ConversationComposerViewState['pendingImages']>[number]

defineProps<{
  images: PendingImage[]
  main: boolean
  removeImage: (imageId: string) => void
}>()
</script>

<template>
  <div
    v-if="images.length > 0"
    class="conversation-composer__attachments"
    :class="{ 'conversation-composer__attachments--main': main }"
  >
    <div
      v-for="image in images"
      :key="image.id"
      class="conversation-composer__attachment"
    >
      <img
        :src="image.previewUrl"
        :alt="image.name"
        class="conversation-composer__attachment-image"
      >
      <button
        class="conversation-composer__attachment-remove"
        @click="removeImage(image.id)"
      >
        <EaIcon
          name="x"
          :size="12"
        />
      </button>
    </div>
  </div>
</template>
