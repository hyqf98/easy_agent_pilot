<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'
import { useUniverEngine } from '../composables/useUniverEngine'
import { useOfficeViewerStore, setCachedSnapshot, getCachedSnapshot } from '../stores/officeViewer'

const props = defineProps<{
  buffer: Uint8Array | null
}>()

const emit = defineEmits<{
  (e: 'loading', value: boolean): void
}>()

const containerRef = ref<HTMLElement | null>(null)
const engine = useUniverEngine()
const store = useOfficeViewerStore()

const getUniverSnapshot = async (): Promise<Uint8Array | null> => {
  try {
    const api = engine.univerAPI
    if (!api) {
      console.warn('[SheetEditor] export: univerAPI is null')
      return null
    }

    const workbook = api.getActiveWorkbook()
    if (!workbook) {
      console.warn('[SheetEditor] export: no active workbook')
      return null
    }

    const snapshot = workbook.getSnapshot()
    if (!snapshot) {
      console.warn('[SheetEditor] export: getSnapshot() returned null')
      return null
    }

    console.log('[SheetEditor] export: snapshot keys =', Object.keys(snapshot))
    console.log('[SheetEditor] export: snapshot.sheets keys =', snapshot.sheets ? Object.keys(snapshot.sheets) : 'NO SHEETS')
    console.log('[SheetEditor] export: snapshot.sheetOrder =', snapshot.sheetOrder)

    const sheets = snapshot.sheets
    const sheetOrder = snapshot.sheetOrder || (sheets ? Object.keys(sheets) : [])
    console.log('[SheetEditor] export: sheetOrder to process =', sheetOrder)

    if (!sheets || Object.keys(sheets).length === 0) {
      console.warn('[SheetEditor] export: no sheets in snapshot, returning null')
      return null
    }

    setCachedSnapshot(snapshot as Record<string, unknown>)

    const ExcelJS = await import('exceljs')
    const workbookXlsx = new ExcelJS.default.Workbook()

    let totalCells = 0

    for (const sheetId of sheetOrder) {
      const sheetData = sheets?.[sheetId]
      if (!sheetData) continue

      const worksheet = workbookXlsx.addWorksheet(sheetData.name || sheetId)

      const cellData = sheetData.cellData || {}
      console.log(`[SheetEditor] export: sheet "${sheetData.name || sheetId}" cellData keys (rows) =`, Object.keys(cellData))

      const maxRow = Math.max(
        sheetData.rowCount || 0,
        ...Object.keys(cellData).map(Number),
      )
      const maxCol = Math.max(
        sheetData.columnCount || 0,
        ...Object.values(cellData).flatMap(
          (row: any) => Object.keys(row).map(Number),
        ),
      )

      console.log(`[SheetEditor] export: sheet range = rows 0..${maxRow}, cols 0..${maxCol}`)

      for (let r = 0; r <= maxRow; r++) {
        const row = cellData[r]
        if (!row) continue
        const excelRow = worksheet.getRow(r + 1)
        for (let c = 0; c <= maxCol; c++) {
          const cell = row[c]
          if (!cell) continue

          totalCells++
          const excelCell = excelRow.getCell(c + 1)
          if (cell.v !== undefined) {
            if (cell.t === 1) {
              excelCell.value = cell.v
            } else if (cell.t === 2) {
              excelCell.value = Number(cell.v)
            } else if (cell.t === 3) {
              excelCell.value = cell.v === true
            } else {
              excelCell.value = String(cell.v)
            }
          }
        }
        excelRow.commit()
      }
    }

    console.log(`[SheetEditor] export: total cells written to ExcelJS = ${totalCells}`)

    const buffer = await workbookXlsx.xlsx.writeBuffer()
    const uint8 = new Uint8Array(buffer)
    console.log(`[SheetEditor] export: xlsx buffer size = ${uint8.length} bytes`)
    return uint8
  } catch (error) {
    console.error('[SheetEditor] Failed to export xlsx:', error)
    return null
  }
}

const bufferToUniverSnapshot = async (
  xlsxBuffer: Uint8Array,
): Promise<Record<string, unknown> | null> => {
  try {
    const ExcelJS = await import('exceljs')
    const wb = new ExcelJS.default.Workbook()
    await wb.xlsx.load(xlsxBuffer)

    if (wb.worksheets.length === 0) return null

    const sheets: Record<string, any> = {}
    const sheetOrder: string[] = []

    wb.worksheets.forEach((ws, idx) => {
      const sheetId = `sheet-${idx}`
      sheetOrder.push(sheetId)

      const cellData: Record<number, Record<number, any>> = {}
      let maxRow = 0
      let maxCol = 0

      ws.eachRow({ includeEmpty: false }, (row, rowNumber) => {
        const r = rowNumber - 1
        if (r > maxRow) maxRow = r
        const rowData: Record<number, any> = {}
        row.eachCell({ includeEmpty: false }, (cell, colNumber) => {
          const c = colNumber - 1
          if (c > maxCol) maxCol = c
          const cellObj: any = {}
          if (cell.value === null || cell.value === undefined) return

          if (typeof cell.value === 'number') {
            cellObj.v = cell.value
            cellObj.t = 2
          } else if (typeof cell.value === 'boolean') {
            cellObj.v = cell.value
            cellObj.t = 3
          } else if (typeof cell.value === 'object' && (cell.value as any).richText) {
            const text = (cell.value as any).richText
              .map((r: any) => r.text)
              .join('')
            cellObj.v = text
            cellObj.t = 1
          } else if (typeof cell.value === 'object' && (cell.value as any).result !== undefined) {
            cellObj.v = String((cell.value as any).result)
            cellObj.t = 1
          } else {
            cellObj.v = String(cell.value)
            cellObj.t = 1
          }

          if (!rowData[c]) rowData[c] = cellObj
          else Object.assign(rowData[c], cellObj)
        })
        if (Object.keys(rowData).length > 0) {
          cellData[r] = rowData
        }
      })

      sheets[sheetId] = {
        id: sheetId,
        name: ws.name || `Sheet${idx + 1}`,
        tabColor: '',
        hidden: 0,
        rowCount: Math.max(maxRow + 1, 1000),
        columnCount: Math.max(maxCol + 1, 20),
        cellData,
        freeze: {},
        scrollTop: 0,
        scrollLeft: 0,
        defaultColumnWidth: 93,
        defaultRowHeight: 27,
        mergeData: [],
        cellMeta: [],
        selections: ['A1'],
      }
    })

    return {
      id: 'workbook-' + Date.now(),
      sheetOrder,
      name: '',
      appVersion: '0.1.0',
      locale: 'zhCN',
      styles: {},
      sheets,
      resources: [],
    } as unknown as Record<string, unknown>
  } catch (error) {
    console.error('[SheetEditor] xlsx import failed:', error)
    return null
  }
}

let loadedBufferRef: Uint8Array | null = null

const loadSheet = async () => {
  if (!containerRef.value) return

  emit('loading', true)
  try {
    const api = await engine.init(containerRef.value, 'sheets')
    const cached = getCachedSnapshot()
    if (cached && typeof cached === 'object' && cached.sheets) {
      console.log('[SheetEditor] loadSheet: using cached snapshot')
      api.createWorkbook(cached as any)
    } else if (props.buffer && props.buffer.length > 0) {
      console.log('[SheetEditor] loadSheet: importing from xlsx buffer, size =', props.buffer.length)
      const snapshot = await bufferToUniverSnapshot(props.buffer)
      if (snapshot) {
        console.log('[SheetEditor] loadSheet: snapshot created, sheets =', Object.keys((snapshot as any).sheets || {}))
        api.createWorkbook(snapshot as any)
      } else {
        console.log('[SheetEditor] loadSheet: snapshot creation failed, creating blank')
        api.createWorkbook({})
      }
    } else {
      console.log('[SheetEditor] loadSheet: no buffer, creating blank workbook')
      api.createWorkbook({})
    }
    loadedBufferRef = props.buffer
    store.registerExportProvider(getUniverSnapshot)
  } catch (error) {
    console.error('Failed to load Excel editor:', error)
  } finally {
    emit('loading', false)
  }
}

watch(() => props.buffer, (newBuffer) => {
  if (newBuffer !== loadedBufferRef) {
    engine.dispose()
    loadSheet()
  }
})

onMounted(loadSheet)

onBeforeUnmount(() => {
  try {
    const api = engine.univerAPI
    if (api) {
      const workbook = api.getActiveWorkbook()
      if (workbook) {
        const snapshot = workbook.getSnapshot()
        if (snapshot) {
          setCachedSnapshot(snapshot as Record<string, unknown>)
        }
      }
    }
  } catch {
    // ignore cleanup errors
  }
  store.registerExportProvider(null)
  engine.dispose()
})
</script>

<template>
  <div class="sheet-editor">
    <div
      ref="containerRef"
      class="sheet-editor__container"
    />
  </div>
</template>

<style scoped>
.sheet-editor {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.sheet-editor__container {
  flex: 1;
  min-height: 0;
}
</style>
