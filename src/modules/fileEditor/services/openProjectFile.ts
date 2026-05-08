import { useFileEditorStore } from '../stores/fileEditor'
import type { FileEditorOpenInput } from '../types'
export async function openProjectFileInWorkspace(input: FileEditorOpenInput): Promise<boolean> {
  const fileEditorStore = useFileEditorStore()
  return fileEditorStore.openFile(input)
}
