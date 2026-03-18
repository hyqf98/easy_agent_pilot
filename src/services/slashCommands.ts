export type SlashCommandPanelType = 'main' | 'mini'

export interface SlashCommandDescriptor {
  name: string
  aliases?: string[]
  scopes: SlashCommandPanelType[]
  descriptionKey: string
  usageKey: string
  insertText: string
}

export interface SlashCommandContext {
  panelType: SlashCommandPanelType
  sessionId: string
  isSending: boolean
  hasMessages: boolean
  currentWorkingDirectory?: string | null
  openCompressionDialog: () => void
  clearSession: () => Promise<void>
  setWorkingDirectory?: (path: string) => Promise<string>
  notifySuccess: (message: string) => void
  notifyWarning: (message: string) => void
  notifyError: (message: string) => void
}

export interface ParsedSlashCommand {
  name: string
  argsText: string
}

export interface SlashCommandExecutionResult {
  handled: boolean
  clearInput?: boolean
}

const COMMANDS: SlashCommandDescriptor[] = [
  {
    name: 'clear',
    scopes: ['main', 'mini'],
    descriptionKey: 'message.slash.clearDesc',
    usageKey: 'message.slash.clearUsage',
    insertText: '/clear'
  },
  {
    name: 'compact',
    aliases: ['compress', 'compect'],
    scopes: ['main', 'mini'],
    descriptionKey: 'message.slash.compactDesc',
    usageKey: 'message.slash.compactUsage',
    insertText: '/compact'
  },
  {
    name: 'cd',
    scopes: ['mini'],
    descriptionKey: 'message.slash.cdDesc',
    usageKey: 'message.slash.cdUsage',
    insertText: '/cd '
  }
]

function normalizeName(name: string): string {
  return name.trim().toLowerCase()
}

export function listSlashCommands(panelType: SlashCommandPanelType): SlashCommandDescriptor[] {
  return COMMANDS.filter(command => command.scopes.includes(panelType))
}

export function searchSlashCommands(
  panelType: SlashCommandPanelType,
  query: string
): SlashCommandDescriptor[] {
  const normalizedQuery = normalizeName(query)
  return listSlashCommands(panelType).filter(command => {
    if (!normalizedQuery) {
      return true
    }

    return command.name.startsWith(normalizedQuery)
      || command.aliases?.some(alias => alias.startsWith(normalizedQuery))
  })
}

export function parseSlashCommandInput(rawInput: string): ParsedSlashCommand | null {
  const trimmed = rawInput.trim()
  if (!trimmed.startsWith('/')) {
    return null
  }

  const body = trimmed.slice(1)
  if (!body || body.includes('\n')) {
    return null
  }

  const firstWhitespace = body.search(/\s/)
  if (firstWhitespace < 0) {
    return {
      name: normalizeName(body),
      argsText: ''
    }
  }

  return {
    name: normalizeName(body.slice(0, firstWhitespace)),
    argsText: body.slice(firstWhitespace + 1).trim()
  }
}

function resolveCommand(name: string, panelType: SlashCommandPanelType): SlashCommandDescriptor | null {
  const normalized = normalizeName(name)
  return listSlashCommands(panelType).find(command =>
    command.name === normalized || command.aliases?.includes(normalized)
  ) ?? null
}

export async function executeSlashCommand(
  parsed: ParsedSlashCommand,
  context: SlashCommandContext
): Promise<SlashCommandExecutionResult> {
  const command = resolveCommand(parsed.name, context.panelType)

  if (!command) {
    return { handled: false }
  }

  if (context.isSending) {
    context.notifyWarning('当前会话正在执行，暂时不能运行斜杠命令。')
    return { handled: true }
  }

  switch (command.name) {
    case 'clear':
      await context.clearSession()
      context.notifySuccess('当前会话消息已清空。')
      return { handled: true, clearInput: true }
    case 'compact':
      if (!context.hasMessages) {
        context.notifyWarning('当前会话还没有可压缩的消息。')
        return { handled: true, clearInput: true }
      }
      context.openCompressionDialog()
      return { handled: true, clearInput: true }
    case 'cd':
      if (context.panelType !== 'mini' || !context.setWorkingDirectory) {
        context.notifyWarning('`/cd` 仅在迷你面板可用。')
        return { handled: true }
      }
      if (!parsed.argsText) {
        context.notifyWarning('请提供要切换的目录路径。')
        return { handled: true }
      }

      try {
        const nextDirectory = await context.setWorkingDirectory(parsed.argsText)
        context.notifySuccess(`当前目录已切换到 ${nextDirectory}`)
        return { handled: true, clearInput: true }
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error)
        context.notifyError(message)
        return { handled: true }
      }
    default:
      return { handled: false }
  }
}
