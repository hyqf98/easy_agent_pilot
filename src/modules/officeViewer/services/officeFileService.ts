import { invoke } from '@tauri-apps/api/core'

export async function readBinaryFile(filePath: string): Promise<Uint8Array> {
  console.log('[officeFileService] readBinaryFile:', filePath)
  const data: number[] = await invoke('read_binary_file', { filePath })
  console.log('[officeFileService] readBinaryFile: got', data.length, 'bytes')
  return new Uint8Array(data)
}

export async function writeBinaryFile(filePath: string, data: Uint8Array): Promise<void> {
  const arr = Array.from(data)
  console.log('[officeFileService] writeBinaryFile:', filePath, 'data length =', data.length, 'Array.from length =', arr.length)
  console.log('[officeFileService] writeBinaryFile: first 20 bytes =', arr.slice(0, 20))
  await invoke('write_binary_file', {
    filePath,
    data: arr,
  })
  console.log('[officeFileService] writeBinaryFile: invoke completed')
}
