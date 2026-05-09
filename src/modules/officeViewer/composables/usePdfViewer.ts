import { ref, shallowRef, onBeforeUnmount } from 'vue'
import type { PDFDocumentProxy } from 'pdfjs-dist'

export function usePdfViewer() {
  let pdfDoc: PDFDocumentProxy | null = null
  const pdfjsLib = shallowRef<typeof import('pdfjs-dist') | null>(null)

  const currentPage = ref(1)
  const totalPages = ref(0)
  const scale = ref(1)
  const isLoading = ref(false)
  const renderingPages = new Set<number>()

  const loadPdfJs = async () => {
    if (pdfjsLib.value) return pdfjsLib.value
    const lib = await import('pdfjs-dist')
    lib.GlobalWorkerOptions.workerSrc = new URL(
      'pdfjs-dist/build/pdf.worker.min.mjs',
      import.meta.url,
    ).toString()
    pdfjsLib.value = lib
    return lib
  }

  const loadDocument = async (buffer: Uint8Array): Promise<void> => {
    isLoading.value = true
    try {
      const lib = await loadPdfJs()
      const loadingTask = lib.getDocument({ data: buffer })
      pdfDoc = await loadingTask.promise
      totalPages.value = pdfDoc.numPages
      currentPage.value = 1
    } finally {
      isLoading.value = false
    }
  }

  const calcFitScale = async (containerWidth: number): Promise<number> => {
    if (!pdfDoc) return 1
    const page = await pdfDoc.getPage(1)
    return page ? containerWidth / page.getViewport({ scale: 1 }).width : 1
  }

  const renderPage = async (
    pageNum: number,
    canvas: HTMLCanvasElement,
  ): Promise<void> => {
    if (!pdfDoc || renderingPages.has(pageNum)) return

    const lib = pdfjsLib.value
    if (!lib) return

    renderingPages.add(pageNum)
    try {
      const page = await pdfDoc.getPage(pageNum)
      const viewport = page.getViewport({ scale: scale.value })
      const outputScale = window.devicePixelRatio || 1

      canvas.width = Math.floor(viewport.width * outputScale)
      canvas.height = Math.floor(viewport.height * outputScale)
      canvas.style.width = `${Math.floor(viewport.width)}px`
      canvas.style.height = `${Math.floor(viewport.height)}px`

      const transform =
        outputScale !== 1 ? [outputScale, 0, 0, outputScale, 0, 0] : undefined

      const context = canvas.getContext('2d')
      if (!context) return

      await (page.render as any)({
        canvasContext: context,
        viewport,
        transform,
      }).promise
    } finally {
      renderingPages.delete(pageNum)
    }
  }

  const renderAllPages = async (
    container: HTMLElement,
  ): Promise<void> => {
    if (!pdfDoc) return

    container.innerHTML = ''

    const fitScale = await calcFitScale(container.clientWidth - 48)
    scale.value = Math.round(fitScale * 100) / 100

    const wrapper = document.createElement('div')
    wrapper.style.cssText = 'display:flex;flex-direction:column;align-items:center;gap:16px;padding:24px 24px;'

    for (let i = 1; i <= totalPages.value; i++) {
      const pageWrap = document.createElement('div')
      pageWrap.style.cssText = 'box-shadow:0 2px 8px rgba(0,0,0,0.12);background:#fff;line-height:0;'

      const canvas = document.createElement('canvas')
      canvas.style.display = 'block'
      pageWrap.appendChild(canvas)
      wrapper.appendChild(pageWrap)
      container.appendChild(wrapper)
      await renderPage(i, canvas)
    }
  }

  const setScale = (newScale: number): void => {
    scale.value = Math.max(0.5, Math.min(3, newScale))
  }

  const zoomIn = (): void => setScale(scale.value + 0.25)
  const zoomOut = (): void => setScale(scale.value - 0.25)

  const nextPage = (): void => {
    if (currentPage.value < totalPages.value) {
      currentPage.value++
    }
  }

  const prevPage = (): void => {
    if (currentPage.value > 1) {
      currentPage.value--
    }
  }

  const goToPage = (page: number): void => {
    if (page >= 1 && page <= totalPages.value) {
      currentPage.value = page
    }
  }

  const dispose = (): void => {
    pdfDoc?.destroy()
    pdfDoc = null
    totalPages.value = 0
    currentPage.value = 1
    renderingPages.clear()
  }

  onBeforeUnmount(dispose)

  return {
    currentPage,
    totalPages,
    scale,
    isLoading,
    loadDocument,
    renderPage,
    renderAllPages,
    setScale,
    zoomIn,
    zoomOut,
    nextPage,
    prevPage,
    goToPage,
    dispose,
  }
}
