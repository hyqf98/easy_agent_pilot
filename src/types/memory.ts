export interface MemoryLibrary {
  id: string
  name: string
  description?: string
  contentMd: string
  createdAt: string
  updatedAt: string
}

export interface RawMemoryRecord {
  id: string
  sessionId?: string
  sessionName?: string
  projectId?: string
  projectName?: string
  messageId?: string
  content: string
  sourceRole: string
  createdAt: string
  updatedAt: string
}

export interface MemoryMergeRun {
  id: string
  libraryId: string
  sourceRecordIds: string[]
  sourceRecordCount: number
  previousContentMd: string
  mergedContentMd: string
  agentId?: string
  modelId?: string
  createdAt: string
}

export interface CreateMemoryLibraryInput {
  name: string
  description?: string
  contentMd?: string
}

export interface UpdateMemoryLibraryInput {
  name?: string
  description?: string
  contentMd?: string
}

export interface ListRawMemoryRecordsQuery {
  sessionId?: string
  projectId?: string
  search?: string
}

export type RawMemoryDeleteOrder = 'oldest' | 'latest'

export interface BatchDeleteRawMemoryRecordsInput extends ListRawMemoryRecordsQuery {
  startAt?: string
  endAt?: string
  limit?: number
  deleteOrder?: RawMemoryDeleteOrder
}

export interface BatchDeleteRawMemoryRecordsResult {
  deletedCount: number
  deletedIds: string[]
}

export interface CreateRawMemoryRecordInput {
  sessionId?: string
  projectId?: string
  messageId?: string
  content: string
  sourceRole?: string
}

export interface UpdateRawMemoryRecordInput {
  content?: string
}

export interface CaptureUserMessageInput {
  sessionId: string
  messageId: string
  content: string
}

export interface ListMemoryMergeRunsQuery {
  libraryId: string
}

export interface MergeRawMemoriesIntoLibraryInput {
  libraryId: string
  sourceRecordIds: string[]
  mergedContentMd: string
  agentId?: string
  modelId?: string
}

export interface MergeRawMemoriesIntoLibraryResult {
  library: MemoryLibrary
  mergeRun: MemoryMergeRun
}

export type MemorySuggestionSourceType = 'library_chunk' | 'raw_record'

export interface MemorySuggestion {
  sourceType: MemorySuggestionSourceType
  sourceId: string
  title: string
  snippet: string
  fullContent: string
  score: number
  matchedTerms: string[]
  libraryId?: string
  libraryName?: string
  sessionId?: string
  sessionName?: string
  projectId?: string
  projectName?: string
  createdAt?: string
}

export interface SearchMemorySuggestionsInput {
  sessionId: string
  projectId?: string
  draftText: string
  limit?: number
}

export interface SearchMemorySuggestionsResult {
  librarySuggestions: MemorySuggestion[]
  rawSuggestions: MemorySuggestion[]
}

export interface RecordSessionMemoryReferenceItem {
  sourceType: MemorySuggestionSourceType
  sourceId: string
}

export interface RecordSessionMemoryReferencesInput {
  sessionId: string
  messageId: string
  references: RecordSessionMemoryReferenceItem[]
}
