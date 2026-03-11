import type { AgentConfig } from '@/stores/agent'
import type { Session } from '@/stores/session'

function normalize(value?: string | null): string {
  return (value || '').trim().toLowerCase()
}

export function resolveSessionAgent(
  session: Pick<Session, 'agentId' | 'agentType'> | null | undefined,
  agents: AgentConfig[]
): AgentConfig | null {
  if (!session) {
    return null
  }

  const identifiers = [session.agentId, session.agentType]
    .map(normalize)
    .filter(Boolean)

  for (const identifier of identifiers) {
    const direct = agents.find(agent => normalize(agent.id) === identifier)
    if (direct) {
      return direct
    }

    const byProvider = agents.find(agent => normalize(agent.provider) === identifier)
    if (byProvider) {
      return byProvider
    }

    const byName = agents.find(agent => normalize(agent.name).includes(identifier))
    if (byName) {
      return byName
    }
  }

  return null
}

export function resolveSessionAgentId(
  session: Pick<Session, 'agentId' | 'agentType'> | null | undefined,
  agents: AgentConfig[]
): string | null {
  return resolveSessionAgent(session, agents)?.id || null
}
