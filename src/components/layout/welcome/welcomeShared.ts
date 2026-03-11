import type { Project } from '@/stores'

export interface WelcomeAction {
  icon: string
  title: string
  description: string
  action: () => void
  shortcut: string
}

export interface WelcomeFeature {
  icon: string
  title: string
  description: string
}

export interface WelcomeProjectBrowserHandlers {
  onSelectProject: (projectId: string) => void
  onProjectContextMenu: (event: MouseEvent, project: Project) => void
}
