export type OfficeFileType = 'pdf' | 'docx' | 'xlsx' | 'pptx'

export type OfficeViewerMode = 'preview' | 'edit'

const OFFICE_EXTENSIONS: Record<string, OfficeFileType> = {
  pdf: 'pdf',
  docx: 'docx',
  doc: 'docx',
  xlsx: 'xlsx',
  xls: 'xlsx',
  csv: 'xlsx',
  pptx: 'pptx',
  ppt: 'pptx',
}

export function isOfficeFile(filePath: string): boolean {
  const ext = extractExtension(filePath)
  return ext in OFFICE_EXTENSIONS
}

export function getOfficeFileType(filePath: string): OfficeFileType {
  const ext = extractExtension(filePath)
  return OFFICE_EXTENSIONS[ext] ?? 'pdf'
}

export function extractExtension(filePath: string): string {
  const fileName = filePath.split(/[\\/]/).pop() ?? filePath
  const lowerName = fileName.toLowerCase()
  const lastDot = lowerName.lastIndexOf('.')
  if (lastDot < 0 || lastDot === lowerName.length - 1) return ''
  return lowerName.slice(lastDot + 1)
}

export interface OfficeViewerOpenInput {
  projectId: string
  projectPath: string
  filePath: string
}

export const OFFICE_FILE_TYPE_LABELS: Record<OfficeFileType, string> = {
  pdf: 'PDF',
  docx: 'Word',
  xlsx: 'Excel',
  pptx: 'PPT',
}
