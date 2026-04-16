<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { StyleValue } from 'vue'
import type { MessageAttachment } from '@/stores/message'
import { getAttachmentPreviewUrl, resolveAttachmentPreviewUrl } from '@/utils/attachmentPreview'
import {
  getAttachmentExtension,
  getAttachmentIconName,
  getAttachmentKind,
  type AttachmentKind
} from '@/utils/attachmentMeta'
import EaIcon from './EaIcon.vue'
import ImageHoverPreview from './ImageHoverPreview.vue'

interface AttachmentThumbnailProps {
  attachment: MessageAttachment
  wrapperClass?: string
  mediaClass?: string
  wrapperStyle?: StyleValue
  mediaStyle?: StyleValue
  previewMaxWidth?: number
  previewMaxHeight?: number
}

const props = withDefaults(defineProps<AttachmentThumbnailProps>(), {
  wrapperClass: '',
  mediaClass: '',
  wrapperStyle: undefined,
  mediaStyle: undefined,
  previewMaxWidth: 360,
  previewMaxHeight: 420
})

const resolvedPreviewUrl = ref(getAttachmentPreviewUrl(props.attachment))

const attachmentKind = computed<AttachmentKind>(() => getAttachmentKind(props.attachment))
const attachmentTitle = computed(() => props.attachment.name.trim() || props.attachment.path.trim())
const attachmentExtension = computed(() => getAttachmentExtension(props.attachment))
const attachmentIconName = computed(() => getAttachmentIconName(props.attachment))
const imageClassName = computed(() => ['attachment-thumbnail__image', props.mediaClass].filter(Boolean).join(' '))

watch(
  () => `${props.attachment.id}:${props.attachment.path}:${props.attachment.mimeType}:${props.attachment.previewUrl ?? ''}`,
  async () => {
    resolvedPreviewUrl.value = getAttachmentPreviewUrl(props.attachment)

    if (attachmentKind.value !== 'image') {
      return
    }

    resolvedPreviewUrl.value = await resolveAttachmentPreviewUrl(props.attachment)
  },
  { immediate: true }
)
</script>

<template>
  <ImageHoverPreview
    v-if="attachmentKind === 'image'"
    :src="resolvedPreviewUrl"
    :alt="attachmentTitle"
    :title="attachmentTitle"
    :wrapper-class="wrapperClass"
    :image-class="imageClassName"
    :wrapper-style="wrapperStyle"
    :image-style="mediaStyle"
    :preview-max-width="previewMaxWidth"
    :preview-max-height="previewMaxHeight"
  >
    <slot />
  </ImageHoverPreview>

  <div
    v-else
    class="attachment-thumbnail"
    :class="[wrapperClass, `attachment-thumbnail--${attachmentKind}`]"
    :style="wrapperStyle"
    tabindex="0"
    :title="attachmentTitle"
  >
    <video
      v-if="attachmentKind === 'video'"
      class="attachment-thumbnail__video"
      :class="mediaClass"
      :style="mediaStyle"
      :src="resolvedPreviewUrl"
      muted
      playsinline
      preload="metadata"
    />

    <div
      v-else
      class="attachment-thumbnail__generic"
      :class="mediaClass"
      :style="mediaStyle"
    >
      <EaIcon
        :name="attachmentIconName"
        :size="18"
      />
      <span class="attachment-thumbnail__ext">
        {{ attachmentExtension }}
      </span>
    </div>

    <div class="attachment-thumbnail__overlay">
      <div
        v-if="attachmentKind === 'video'"
        class="attachment-thumbnail__video-badge"
      >
        <EaIcon
          name="play"
          :size="12"
        />
        <span>{{ attachmentExtension }}</span>
      </div>
      <span class="attachment-thumbnail__name">{{ attachmentTitle }}</span>
    </div>

    <slot />
  </div>
</template>

<style scoped>
.attachment-thumbnail {
  position: relative;
  display: inline-flex;
  flex-shrink: 0;
  overflow: hidden;
  outline: none;
}

.attachment-thumbnail__image,
.attachment-thumbnail__video,
.attachment-thumbnail__generic {
  width: 100%;
  height: 100%;
}

.attachment-thumbnail__image,
.attachment-thumbnail__video {
  display: block;
  object-fit: cover;
}

.attachment-thumbnail__generic {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px;
  background:
    radial-gradient(circle at top, rgba(96, 165, 250, 0.18), transparent 48%),
    linear-gradient(180deg, rgba(241, 245, 249, 0.98), rgba(226, 232, 240, 0.98));
  color: #0f172a;
}

.attachment-thumbnail--audio .attachment-thumbnail__generic {
  background:
    radial-gradient(circle at top, rgba(34, 197, 94, 0.16), transparent 48%),
    linear-gradient(180deg, rgba(240, 253, 244, 0.98), rgba(220, 252, 231, 0.98));
}

.attachment-thumbnail__ext {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.attachment-thumbnail__overlay {
  position: absolute;
  inset-inline: 0;
  bottom: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 6px;
  background: linear-gradient(180deg, transparent, rgba(15, 23, 42, 0.9));
  color: white;
  pointer-events: none;
}

.attachment-thumbnail__name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 10px;
  font-weight: 600;
}

.attachment-thumbnail__video-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  align-self: flex-start;
  padding: 2px 6px;
  border-radius: 999px;
  background: rgba(15, 23, 42, 0.72);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.06em;
}

:global([data-theme='dark']) .attachment-thumbnail__generic,
:global(.dark) .attachment-thumbnail__generic {
  background:
    radial-gradient(circle at top, rgba(96, 165, 250, 0.2), transparent 48%),
    linear-gradient(180deg, rgba(30, 41, 59, 0.98), rgba(15, 23, 42, 0.98));
  color: rgba(226, 232, 240, 0.94);
}

:global([data-theme='dark']) .attachment-thumbnail--audio .attachment-thumbnail__generic,
:global(.dark) .attachment-thumbnail--audio .attachment-thumbnail__generic {
  background:
    radial-gradient(circle at top, rgba(34, 197, 94, 0.18), transparent 48%),
    linear-gradient(180deg, rgba(22, 101, 52, 0.36), rgba(15, 23, 42, 0.98));
}
</style>
