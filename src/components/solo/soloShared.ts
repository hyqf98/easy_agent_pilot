export interface SoloAgentOption {
  label: string
  value: string
  description?: string
}

export type SoloRunFormMode = 'create' | 'edit'

export interface SoloCreateFormState {
  projectId: string
  executionPath: string
  name: string
  requirement: string
  goal: string
  memoryLibraryIds: string[]
  maxDispatchDepth: number
  participantExpertIds: string[]
  coordinatorExpertId: string | null
}
