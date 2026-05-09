<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'

const props = defineProps<{
  buffer: Uint8Array | null
}>()

const emit = defineEmits<{
  (e: 'loading', value: boolean): void
}>()

const sidebarRef = ref<HTMLElement | null>(null)
const contentAreaRef = ref<HTMLElement | null>(null)
const currentSlide = ref(1)
const slideCount = ref(0)
const scale = ref(1)
let mainSlidePreviewer: any = null
let pptxPreviewLib: any = null

const SLIDE_W = 960
const SLIDE_H = 540
const THUMB_W = 140
const THUMB_H = Math.round(THUMB_W * SLIDE_H / SLIDE_W)

const loadPptxLib = async () => {
  if (pptxPreviewLib) return pptxPreviewLib
  const lib = await import('pptx-preview')
  pptxPreviewLib = lib
  return lib
}

const zoomIn = () => {
  scale.value = Math.min(scale.value + 0.25, 3)
}

const zoomOut = () => {
  scale.value = Math.max(scale.value - 0.25, 0.5)
}

const cleanupPptxNav = (container: HTMLElement) => {
  container.querySelectorAll(
    '.pptx-preview-wrapper-next, .pptx-preview-wrapper-pagination'
  ).forEach((el) => el.remove())
}

const renderSingleSlideToElement = async (
  container: HTMLElement,
  slideIndex: number,
): Promise<void> => {
  if (!props.buffer) return

  const { init } = await loadPptxLib()
  const thumbScale = THUMB_W / SLIDE_W

  const renderRoot = document.createElement('div')
  renderRoot.style.cssText = `width:${SLIDE_W}px;height:${SLIDE_H}px;position:absolute;top:0;left:0;overflow:hidden;clip:rect(0,${SLIDE_W}px,${SLIDE_H}px,0);`

  const previewer = init(renderRoot, { mode: 'slide', width: SLIDE_W, height: SLIDE_H })
  await previewer.preview(props.buffer.buffer as ArrayBuffer)

  for (let s = 0; s < slideIndex; s++) {
    previewer.renderNextSlide()
  }

  previewer.destroy()
  cleanupPptxNav(renderRoot)

  const scaleBox = document.createElement('div')
  scaleBox.style.cssText = `position:relative;width:${THUMB_W}px;height:${THUMB_H}px;overflow:hidden;`
  scaleBox.appendChild(renderRoot)
  renderRoot.style.transform = `scale(${thumbScale})`
  renderRoot.style.transformOrigin = 'top left'

  container.appendChild(scaleBox)
}

const loadSlides = async () => {
  if (!props.buffer) return

  emit('loading', true)
  try {
    const { init } = await loadPptxLib()

    const tempContainer = document.createElement('div')
    tempContainer.style.cssText = `width:${THUMB_W}px;height:${THUMB_H}px;position:absolute;left:-9999px;overflow:hidden;`
    document.body.appendChild(tempContainer)

    const tempPreviewer = init(tempContainer, {
      mode: 'slide',
      width: THUMB_W,
      height: THUMB_H,
    })
    await tempPreviewer.preview(props.buffer.buffer as ArrayBuffer)
    const count = tempPreviewer.slideCount ?? 0
    tempPreviewer.destroy()
    document.body.removeChild(tempContainer)

    slideCount.value = count
    currentSlide.value = 1
    scale.value = 1

    await nextTick()
    await new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))
    await buildAndRenderThumbnails()
    await renderMainSlide()
  } catch (error) {
    console.error('Failed to load PPTX:', error)
  } finally {
    emit('loading', false)
  }
}

const buildAndRenderThumbnails = async () => {
  if (!sidebarRef.value) return

  sidebarRef.value.innerHTML = ''

  for (let i = 0; i < slideCount.value; i++) {
    const thumbItem = document.createElement('div')
    thumbItem.className = 'ppt-thumb-item'
    if (i === 0) thumbItem.classList.add('ppt-thumb-item--active')

    const labelWrap = document.createElement('div')
    labelWrap.className = 'ppt-thumb-label-wrap'

    const thumbCanvas = document.createElement('div')
    thumbCanvas.className = 'ppt-thumb-canvas'
    labelWrap.appendChild(thumbCanvas)

    const label = document.createElement('span')
    label.className = 'ppt-thumb-label'
    label.textContent = String(i + 1)
    labelWrap.appendChild(label)

    thumbItem.appendChild(labelWrap)
    sidebarRef.value.appendChild(thumbItem)

    const slideIdx = i
    thumbItem.addEventListener('click', () => goToSlide(slideIdx + 1))

    try {
      await renderSingleSlideToElement(thumbCanvas, slideIdx)
    } catch (e) {
      console.error(`Failed to render thumbnail slide ${i + 1}:`, e)
    }
  }
}

const renderMainSlide = async () => {
  if (!contentAreaRef.value || !props.buffer) return

  if (mainSlidePreviewer) {
    mainSlidePreviewer.destroy()
    mainSlidePreviewer = null
  }

  contentAreaRef.value.innerHTML = ''

  const containerW = contentAreaRef.value.clientWidth
  const containerH = contentAreaRef.value.clientHeight
  if (containerW <= 0 || containerH <= 0) return

  const padding = 24
  const availW = containerW - padding * 2
  const availH = containerH - padding * 2
  if (availW <= 0 || availH <= 0) return

  const fitScaleX = availW / SLIDE_W
  const fitScaleY = availH / SLIDE_H
  const fitScale = Math.min(fitScaleX, fitScaleY)
  const finalScale = Math.min(fitScale, 1.5) * scale.value

  const { init } = await loadPptxLib()

  const renderRoot = document.createElement('div')
  renderRoot.className = 'ppt-main-frame'
  renderRoot.style.cssText = `width:${SLIDE_W}px;height:${SLIDE_H}px;position:absolute;top:0;left:0;overflow:hidden;clip:rect(0,${SLIDE_W}px,${SLIDE_H}px,0);`

  mainSlidePreviewer = init(renderRoot, {
    mode: 'slide',
    width: SLIDE_W,
    height: SLIDE_H,
  })
  await mainSlidePreviewer.preview(props.buffer.buffer as ArrayBuffer)

  const targetSlide = currentSlide.value - 1
  for (let s = 0; s < targetSlide; s++) {
    mainSlidePreviewer.renderNextSlide()
  }

  cleanupPptxNav(renderRoot)

  renderRoot.style.transform = `scale(${finalScale})`
  renderRoot.style.transformOrigin = 'top left'

  const wrapper = document.createElement('div')
  wrapper.className = 'ppt-main-slide-inner'
  wrapper.style.width = `${Math.round(SLIDE_W * finalScale)}px`
  wrapper.style.height = `${Math.round(SLIDE_H * finalScale)}px`
  wrapper.style.position = 'relative'
  wrapper.style.overflow = 'hidden'
  wrapper.style.boxShadow = '0 2px 12px rgba(0,0,0,0.2)'
  wrapper.style.background = '#fff'

  wrapper.appendChild(renderRoot)
  contentAreaRef.value.appendChild(wrapper)
}

const goToSlide = (slide: number) => {
  if (slide >= 1 && slide <= slideCount.value && slide !== currentSlide.value) {
    currentSlide.value = slide
  }
}

const updateActiveThumbnail = () => {
  if (!sidebarRef.value) return
  sidebarRef.value.querySelectorAll('.ppt-thumb-item').forEach((item, idx) => {
    if (idx + 1 === currentSlide.value) {
      item.classList.add('ppt-thumb-item--active')
    } else {
      item.classList.remove('ppt-thumb-item--active')
    }
  })
}

watch(() => props.buffer, loadSlides)

watch(currentSlide, async () => {
  updateActiveThumbnail()
  await renderMainSlide()
})

watch(scale, async () => {
  await renderMainSlide()
})

onMounted(loadSlides)

onBeforeUnmount(() => {
  if (mainSlidePreviewer) {
    mainSlidePreviewer.destroy()
    mainSlidePreviewer = null
  }
})
</script>

<template>
  <div class="slide-editor">
    <div class="slide-editor__toolbar">
      <span class="slide-editor__page-info">
        {{ currentSlide }} / {{ slideCount }}
      </span>
      <span class="slide-editor__divider" />
      <button
        type="button"
        class="slide-editor__btn"
        @click="zoomOut"
      >
        −
      </button>
      <span class="slide-editor__scale">{{ Math.round(scale * 100) }}%</span>
      <button
        type="button"
        class="slide-editor__btn"
        @click="zoomIn"
      >
        +
      </button>
      <span class="slide-editor__readonly-badge">只读</span>
    </div>

    <div class="slide-editor__body">
      <div class="slide-editor__sidebar">
        <div
          ref="sidebarRef"
          class="slide-editor__thumbnails"
        />
      </div>
      <div
        ref="contentAreaRef"
        class="slide-editor__content"
      >
        <div
          v-if="slideCount === 0"
          class="slide-editor__empty"
        >
          加载中...
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.slide-editor {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  background: var(--color-bg-secondary, #525659);
}

.slide-editor__toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 16px;
  background: var(--color-background-secondary, #f5f5f5);
  border-bottom: 1px solid var(--color-border, #e0e0e0);
  flex-shrink: 0;
}

.slide-editor__btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: 1px solid var(--color-border, #e0e0e0);
  border-radius: 4px;
  background: var(--color-surface, #fff);
  cursor: pointer;
  font-size: 15px;
  line-height: 1;
  color: var(--color-text-primary, #333);
  transition: background 0.15s;
}

.slide-editor__btn:hover {
  background: var(--color-background-tertiary, #eee);
}

.slide-editor__page-info,
.slide-editor__scale {
  font-size: 13px;
  color: var(--color-text-secondary, #666);
  min-width: 40px;
  text-align: center;
  user-select: none;
}

.slide-editor__divider {
  width: 1px;
  height: 18px;
  background: var(--color-border, #e0e0e0);
  margin: 0 4px;
}

.slide-editor__readonly-badge {
  margin-left: auto;
  padding: 2px 8px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-warning, #f59e0b) 15%, transparent);
  color: var(--color-warning, #f59e0b);
  font-size: 11px;
  font-weight: 600;
}

.slide-editor__body {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}

.slide-editor__sidebar {
  width: 176px;
  flex-shrink: 0;
  background: var(--color-background-secondary, #f5f5f5);
  border-right: 1px solid var(--color-border, #e0e0e0);
  overflow-y: auto;
  overflow-x: hidden;
}

.slide-editor__thumbnails {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 12px 8px;
}

.slide-editor__content {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.slide-editor__empty {
  color: var(--color-text-tertiary, #999);
  font-size: 14px;
}

.slide-editor__sidebar :deep(.ppt-thumb-item) {
  cursor: pointer;
  border: 2px solid transparent;
  border-radius: 4px;
  transition: border-color 0.15s, background 0.15s;
}

.slide-editor__sidebar :deep(.ppt-thumb-item:hover) {
  border-color: var(--color-primary, #4a90d9);
}

.slide-editor__sidebar :deep(.ppt-thumb-item--active) {
  border-color: var(--color-primary, #4a90d9);
  background: color-mix(in srgb, var(--color-primary, #4a90d9) 10%, transparent);
}

.slide-editor__sidebar :deep(.ppt-thumb-label-wrap) {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.slide-editor__sidebar :deep(.ppt-thumb-canvas) {
  width: 140px;
  height: 79px;
  border-radius: 2px;
  background: #fff;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.08);
  overflow: hidden;
}

.slide-editor__sidebar :deep(.ppt-thumb-label) {
  font-size: 11px;
  color: var(--color-text-secondary, #666);
  user-select: none;
}

.slide-editor__content :deep(.ppt-main-slide-inner) {
  overflow: hidden;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.2);
  background: #fff;
  position: relative;
}
</style>
