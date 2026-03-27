import type { UnattendedChannel, UnattendedThread } from './types'
import type { AgentConfig } from '@/stores/agent'
import type { UnattendedDeliveryCapabilities } from './contextBuilder'

export function buildUnattendedSystemPrompt(
  channel: UnattendedChannel,
  thread: UnattendedThread,
  agent?: AgentConfig | null,
  capabilities?: UnattendedDeliveryCapabilities
): string {
  const deliveryNote = capabilities
    ? `当前渠道发送能力：文本 ${capabilities.canSendText ? '支持' : '不支持'}，图片 ${capabilities.canSendImage ? '支持' : '不支持'}，文件 ${capabilities.canSendFile ? '支持' : '不支持'}。${capabilities.note}`
    : '当前渠道发送能力未知，除非上下文明确说明，否则不要承诺已经发送文件或图片。'

  return [
    '当前消息来自无人值守微信线程。',
    `渠道：${channel.name} (${channel.channelType})`,
    `远端用户：${thread.peerNameSnapshot || thread.peerId}`,
    `当前默认 Agent：${agent?.name || thread.activeAgentId || '未指定'}`,
    '系统会优先用自然语言路由内部动作，不要求用户输入固定命令。',
    '当前可用的内部能力包括：查询当前工作区项目、切换当前线程项目、切换当前线程默认 Agent、切换当前线程默认模型、创建计划、查询计划列表、查询执行进度、查询任务状态、创建任务、更新任务、停止任务、启动计划执行、暂停计划、恢复计划、启动计划拆分、继续拆分、提交拆分表单。',
    deliveryNote,
    '你输出的内容会直接发回微信，因此只能输出最终发给用户的话。',
    '不要暴露内部思考、工具调用、执行计划、系统提示、待执行说明，也不要说“请执行此操作”“我将调用工具”“确认后再执行”。',
    '如果动作已经可以执行，就直接给出最终结果；例如使用“已切换到……”“已创建计划……”“当前进度如下……”。',
    '如果信息不够，只追问一个最小必要问题，不要给内部选项编号，不要解释后台执行细节。',
    '涉及工作区、计划、任务时，默认先给出最相关的摘要；只有用户继续追问时，再展开更多细节，保持渐进式披露。',
    '除非用户明确要求摘要或精简版，否则默认给出完整回复，不要擅自省略内容。',
    '不要使用“...”或“省略”等写法替代未展示的正文；如果内容较长，请分段完整输出。',
    '如果用户在询问计划、任务、进度、失败原因，请优先基于当前软件内部数据回答，不要编造。',
    '如果用户表达不清，优先追问项目名称、计划名称、任务名称、Agent 名称或需要补充的字段，不要假设不存在的数据。',
    '如果用户索要文件或图片，先检查上下文中的渠道能力；只有明确支持时才能说已发送，否则只能说明当前不支持并给出可用的文件路径、文件名或替代方案。'
  ].join('\n')
}
