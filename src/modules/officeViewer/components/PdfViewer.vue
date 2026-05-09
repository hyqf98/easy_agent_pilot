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
const thumbnailRefs = ref<Map<number, HTMLElement>>(new Map())

const currentPage = ref(1)
const totalPages = ref(0)
const scale = ref(1)
let pdfDocProxy: any = null
let pdfjsLibRef: any = null

const zoomIn = () => {
  scale.value = Math.min(scale.value + 0.25, 3)
}

const zoomOut = () => {
  scale.value = Math.max(scale.value - 0.25, 0.5)
}

const loadPdfJs = async () => {
  if (pdfjsLibRef) return pdfjsLibRef
  const lib = await import('pdfjs-dist')
  lib.GlobalWorkerOptions.workerSrc = new URL(
    'pdfjs-dist/build/pdf.worker.min.mjs',
    import.meta.url,
  ).toString()
  pdfjsLibRef = lib
  return lib
}

const renderPageToCanvas = async (
  page: any,
  canvas: HTMLCanvasElement,
  renderScale: number,
) => {
  const viewport = page.getViewport({ scale: renderScale })
  const outputScale = window.devicePixelRatio || 1

  canvas.width = Math.floor(viewport.width * outputScale)
  canvas.height = Math.floor(viewport.height * outputScale)
  canvas.style.width = `${Math.floor(viewport.width)}px`
  canvas.style.height = `${Math.floor(viewport.height)}px`

  const context = canvas.getContext('2d')
  if (!context) return

  const transform =
    outputScale !== 1 ? [outputScale, 0, 0, outputScale, 0, 0] : undefined

  await (page.render as any)({
    canvasContext: context,
    viewport,
    transform,
  }).promise
}

const loadPdf = async () => {
  if (!props.buffer) return

  emit('loading', true)
  try {
    const lib = await loadPdfJs()
    const loadingTask = lib.getDocument({ data: props.buffer })
    pdfDocProxy = await loadingTask.promise
    totalPages.value = pdfDocProxy.numPages
    currentPage.value = 1
    scale.value = 1

    await nextTick()
    await renderAllThumbnails()
    await renderMainPage()
    scrollToCurrentPage()
  } catch (error) {
    console.error('Failed to load PDF:', error)
  } finally {
    emit('loading', false)
  }
}

const renderAllThumbnails = async () => {
  if (!pdfDocProxy || !sidebarRef.value) return

  sidebarRef.value.innerHTML = ''
  thumbnailRefs.value.clear()

  const thumbW = 140
  for (let i = 1; i <= totalPages.value; i++) {
    try {
      const page = await pdfDocProxy.getPage(i)
      const viewport = page.getViewport({ scale: 1 })
      const ratio = viewport.height / viewport.width
      const thumbH = Math.round(thumbW * ratio)
      const actualScale = thumbW / viewport.width

      const thumbItem = document.createElement('div')
      thumbItem.className = 'pdf-thumb-item'
      thumbItem.dataset.page = String(i)
      if (i === currentPage.value) {
        thumbItem.classList.add('pdf-thumb-item--active')
      }

      const labelWrap = document.createElement('div')
      labelWrap.className = 'pdf-thumb-label-wrap'

      const canvas = document.createElement('canvas')
      canvas.style.width = `${thumbW}px`
      canvas.style.height = `${thumbH}px`
      canvas.className = 'pdf-thumb-canvas'
      labelWrap.appendChild(canvas)

      const label = document.createElement('span')
      label.className = 'pdf-thumb-label'
      label.textContent = String(i)
      labelWrap.appendChild(label)

      thumbItem.appendChild(labelWrap)
      sidebarRef.value.appendChild(thumbItem)
      thumbnailRefs.value.set(i, thumbItem)

      const pageNum = i
      thumbItem.addEventListener('click', () => {
        goToPage(pageNum)
      })

      await renderPageToCanvas(page, canvas, actualScale)
    } catch (e) {
      console.error(`Failed to render thumbnail page ${i}:`, e)
    }
  }
}

const renderMainPage = async () => {
  if (!pdfDocProxy || !contentAreaRef.value) return

  const containerW = contentAreaRef.value.clientWidth - 64
  const containerH = contentAreaRef.value.clientHeight - 64
  if (containerW <= 0 || containerH <= 0) return

  try {
    const page = await pdfDocProxy.getPage(currentPage.value)
    const viewport = page.getViewport({ scale: 1 })

    const fitScaleW = containerW / viewport.width
    const fitScaleH = containerH / viewport.height
    const fitScale = Math.min(fitScaleW, fitScaleH)
    const finalScale = Math.min(fitScale, 2) * scale.value

    const existingCanvas = contentAreaRef.value.querySelector('canvas')
    const canvas = existingCanvas || document.createElement('canvas')

    if (!existingCanvas) {
      const wrapper = document.createElement('div')
      wrapper.className = 'pdf-main-page-wrapper'
      wrapper.appendChild(canvas)
      contentAreaRef.value.innerHTML = ''
      contentAreaRef.value.appendChild(wrapper)
    }

    await renderPageToCanvas(page, canvas, finalScale)
  } catch (e) {
    console.error(`Failed to render main page ${currentPage.value}:`, e)
  }
}

const goToPage = (page: number) => {
  if (page >= 1 && page <= totalPages.value && page !== currentPage.value) {
    currentPage.value = page
  }
}

const scrollToCurrentPage = () => {
  const thumbEl = thumbnailRefs.value.get(currentPage.value)
  if (thumbEl && sidebarRef.value) {
    thumbEl.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  }
}

const updateActiveThumbnail = () => {
  thumbnailRefs.value.forEach((el, pageNum) => {
    if (pageNum === currentPage.value) {
      el.classList.add('pdf-thumb-item--active')
    } else {
      el.classList.remove('pdf-thumb-item--active')
    }
  })
}

watch(() => props.buffer, loadPdf)

watch(currentPage, async () => {
  updateActiveThumbnail()
  scrollToCurrentPage()
  await renderMainPage()
})

watch(scale, async () => {
  await renderMainPage()
})

onMounted(loadPdf)

onBeforeUnmount(() => {
  pdfDocProxy?.destroy()
  pdfDocProxy = null
})
</script>

<template>
  <div class="pdf-viewer">
    <div class="pdf-viewer__toolbar">
      <span class="pdf-viewer__page-info">
        {{ currentPage }} / {{ totalPages }}
      </span>

      <span class="pdf-viewer__divider" />

      <button
        type="button"
        class="pdf-viewer__btn"
        @click="zoomOut"
      >
        −
      </button>
      <span class="pdf-viewer__scale">{{ Math.round(scale * 100) }}%</span>
      <button
        type="button"
        class="pdf-viewer__btn"
        @click="zoomIn"
      >
        +
      </button>
    </div>

    <div class="pdf-viewer__body">
      <div class="pdf-viewer__sidebar">
        <div
          ref="sidebarRef"
          class="pdf-viewer__thumbnails"
        />
      </div>

      <div
        ref="contentAreaRef"
        class="pdf-viewer__content"
      >
        <div
          v-if="totalPages === 0"
          class="pdf-viewer__empty"
        >
          加载中...
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pdf-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  background: var(--color-bg-secondary, #525659);
}

.pdf-viewer__toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 16px;
  background: var(--color-background-secondary, #f5f5f5);
  border-bottom: 1px solid var(--color-border, #e0e0e0);
  flex-shrink: 0;
}

.pdf-viewer__btn {
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

.pdf-viewer__btn:hover {
  background: var(--color-background-tertiary, #eee);
}

.pdf-viewer__page-info {
  font-size: 13px;
  color: var(--color-text-secondary, #666);
  min-width: 50px;
  text-align: center;
  user-select: none;
}

.pdf-viewer__scale {
  font-size: 13px;
  color: var(--color-text-secondary, #666);
  min-width: 40px;
  text-align: center;
  user-select: none;
}

.pdf-viewer__divider {
  width: 1px;
  height: 18px;
  background: var(--color-border, #e0e0e0);
  margin: 0 4px;
}

.pdf-viewer__body {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}

.pdf-viewer__sidebar {
  width: 176px;
  flex-shrink: 0;
  background: var(--color-background-secondary, #f5f5f5);
  border-right: 1px solid var(--color-border, #e0e0e0);
  overflow-y: auto;
  overflow-x: hidden;
}

.pdf-viewer__thumbnails {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 12px 8px;
}

.pdf-viewer__content {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: auto;
  padding: 32px;
}

.pdf-viewer__empty {
  color: var(--color-text-tertiary, #999);
  font-size: 14px;
}

.pdf-viewer__content :deep(.pdf-main-page-wrapper) {
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.2);
  background: #fff;
  line-height: 0;
}

.pdf-viewer__content :deep(.pdf-main-canvas) {
  display: block;
}

.pdf-viewer__sidebar :deep(.pdf-thumb-item) {
  cursor: pointer;
  border: 2px solid transparent;
  border-radius: 4px;
  transition: border-color 0.15s, background 0.15s;
}

.pdf-viewer__sidebar :deep(.pdf-thumb-item:hover) {
  border-color: var(--color-primary, #4a90d9);
}

.pdf-viewer__sidebar :deep(.pdf-thumb-item--active) {
  border-color: var(--color-primary, #4a90d9);
  background: color-mix(in srgb, var(--color-primary, #4a90d9) 10%, transparent);
}

.pdf-viewer__sidebar :deep(.pdf-thumb-label-wrap) {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.pdf-viewer__sidebar :deep(.pdf-thumb-canvas) {
  display: block;
  border-radius: 2px;
  background: #fff;
}

.pdf-viewer__sidebar :deep(.pdf-thumb-label) {
  font-size: 11px;
  color: var(--color-text-secondary, #666);
  user-select: none;
}
</style>
