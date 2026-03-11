import { buildStructuredResultContent } from '@/utils/structuredContent'

interface ResultRecordLike {
  result_summary?: string | null
  fail_reason?: string | null
  result_files: string[]
}

interface ResultFileGroups {
  generatedFiles: string[]
  modifiedFiles: string[]
  changedFiles: string[]
  deletedFiles: string[]
}

export function groupTaskResultFiles(resultFiles: string[]): ResultFileGroups {
  const groups: ResultFileGroups = {
    generatedFiles: [],
    modifiedFiles: [],
    changedFiles: [],
    deletedFiles: []
  }

  resultFiles.forEach((raw) => {
    if (raw.startsWith('added:')) {
      groups.generatedFiles.push(raw.slice(6))
      return
    }
    if (raw.startsWith('modified:')) {
      groups.modifiedFiles.push(raw.slice(9))
      return
    }
    if (raw.startsWith('changed:')) {
      groups.changedFiles.push(raw.slice(8))
      return
    }
    if (raw.startsWith('deleted:')) {
      groups.deletedFiles.push(raw.slice(8))
      return
    }

    groups.changedFiles.push(raw)
  })

  return groups
}

export function buildStructuredResultContentFromRecord(record: ResultRecordLike): string {
  const groupedFiles = groupTaskResultFiles(record.result_files)
  const summary = record.fail_reason
    ? `${record.result_summary || '任务执行失败'}\n\n失败原因: ${record.fail_reason}`
    : record.result_summary

  return buildStructuredResultContent({
    summary,
    ...groupedFiles
  })
}
