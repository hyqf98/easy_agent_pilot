import i18n from '@/i18n'
import type { McpServerConfig } from './strategies/types'

export function buildMainConversationFormRequestPrompt(): string {
  const locale = i18n.mode === 'legacy'
    ? String((i18n.global as any).locale)
    : String((i18n.global.locale as any).value)

  if (locale === 'en-US') {
    return `You are collaborating with the user in the app's main conversation.

When you cannot continue the current task and must collect explicit parameters, scope, preferences, or environment details from the user:
1. Do not ask using normal paragraphs, numbered lists, markdown, or prose before the form.
2. Output exactly one valid JSON object and do not wrap it in a code block.
3. The object must use type="form_request" and include a short question plus exactly one formSchema object.
4. formSchema must include formId, title, optional description, optional submitText, and fields.
5. Each field must use these keys only: name, label, type, required, placeholder, default, suggestion, suggestionReason, options, validation, condition, allowOther, otherLabel.
6. Never use alternative keys such as formFieldId, fieldId, id, or formSchema.fields[].id.
7. Field type may only be text, textarea, select, multiselect, number, checkbox, radio, date, or slider.
8. select, radio, and multiselect must provide options in the shape [{label, value}] using the same JSON object structure.
9. Use standard ASCII JSON only: double quotes ", braces {}, brackets [], true/false/null, and no comments, no trailing commas, no Chinese punctuation, no smart quotes.
10. When outputting form_request, do not add explanations, headings, greetings, bullet points, or any extra text before or after the JSON.
11. Output form_request only when you truly need more user input to continue. Otherwise reply normally.
12. If the user sends a JSON object with type="form_response", treat it as the form answer and continue.
13. Put every field required to continue into that single formSchema. Do not split the request into multiple forms, multiple form_request objects, or follow-up form rounds for the same missing information.
14. Never output a forms array for the main conversation form request.
15. Even if you would normally use brainstorming, questionnaires, or A/B/C/D follow-up questions, in this app you must still output one form_request JSON instead of plain text questions.
16. Before answering with form_request, silently self-check that your final output is directly parseable by JSON.parse, uses formSchema instead of forms, and that every field uses name rather than formFieldId.

Valid shape example:
{"type":"form_request","question":"Please confirm the missing parameters","formSchema":{"formId":"deployment","title":"Deployment","fields":[{"name":"interface","label":"Interface","type":"radio","options":[{"label":"CLI","value":"cli"},{"label":"Web","value":"web"}]}]}}`
  }

  return `你正在桌面应用的主会话中与用户协作。

当你还不能继续当前任务、必须向用户补充收集明确参数、范围、偏好、环境信息时：
1. 不要用普通段落、编号列表、markdown，或任何表单前置说明来提问。
2. 只输出一个合法 JSON 对象，且不要放在代码块里。
3. 这个对象必须使用 type="form_request"，并包含一句简短 question 和唯一一个 formSchema 对象。
4. formSchema 必须包含 formId、title，以及可选的 description、submitText、fields。
5. 每个字段只能使用这些键：name、label、type、required、placeholder、default、suggestion、suggestionReason、options、validation、condition、allowOther、otherLabel。
6. 严禁使用 formFieldId、fieldId、id、或其他替代字段名；字段标识必须叫 name。
7. 字段 type 只能是 text、textarea、select、multiselect、number、checkbox、radio、date、slider。
8. select、radio、multiselect 必须提供 options，结构严格为 [{label, value}]。
9. 必须使用标准 ASCII JSON：半角双引号 "、{}、[]、true/false/null；禁止注释、禁止尾逗号、禁止中文标点、禁止智能引号。
10. 输出 form_request 时，JSON 前后不要再附加解释、标题、问候语、列表或其他任何文本。
11. 只有在确实需要用户补充信息才能继续时，才输出 form_request；其余场景正常回答。
12. 如果用户发送一个 type="form_response" 的 JSON 对象，把它视为表单回答并继续处理。
13. 所有继续所需的信息都必须一次性收集到这一个 formSchema 里，不允许拆成多个表单、多个 form_request，或在同一轮缺失信息上继续追问第二个表单。
14. 主会话表单请求严禁输出 forms 数组。
15. 即使你原本会采用 brainstorming、问卷式追问、A/B/C/D 选项提问，也必须改为输出一个 form_request JSON，不能输出普通文本问题。
16. 在输出 form_request 前，先静默自检：最终结果必须能被 JSON.parse 直接解析，必须使用 formSchema 而不是 forms，且所有字段都使用 name 而不是 formFieldId。

合法结构示例：
{"type":"form_request","question":"请确认缺失参数","formSchema":{"formId":"deployment","title":"部署形态","fields":[{"name":"interface","label":"交互形式","type":"radio","options":[{"label":"CLI 命令行","value":"cli"},{"label":"Web 应用","value":"web"}]}]}}`
}

interface ImageAttachmentFallbackPromptOptions {
  runtimeName: string
  mcpServers?: McpServerConfig[]
}

function formatMcpServerNames(mcpServers?: McpServerConfig[]): string {
  const names = (mcpServers ?? [])
    .map(server => server.name.trim())
    .filter(Boolean)

  return Array.from(new Set(names)).join(', ')
}

/**
 * 为图片附件提供通用的视觉降级策略提示。
 * 仅描述处理原则，不绑定具体模型名称，允许运行时按能力自行选择直读图片或改走 MCP。
 */
export function buildImageAttachmentFallbackPrompt(
  options: ImageAttachmentFallbackPromptOptions
): string {
  const locale = i18n.mode === 'legacy'
    ? String((i18n.global as any).locale)
    : String((i18n.global.locale as any).value)
  const mcpServerList = formatMcpServerNames(options.mcpServers)

  if (locale === 'en-US') {
    const lines = [
      `Image attachment handling for ${options.runtimeName}:`,
      '1. If the current model/runtime can directly inspect attached images, do that first.',
      '2. If the current model/runtime cannot inspect images directly, do not stop at saying you cannot view the screenshot.',
      '3. Immediately inspect the available MCP tools in this run and prefer tools/servers that support image understanding, OCR, screenshot analysis, multimodal reading, or visual extraction.',
      '4. Treat the MCP result as the image observation source, cite it explicitly, and continue the task based on that result.',
      '5. Never invent visual details. If no suitable MCP tool exists, explicitly say that no vision-capable tool is currently configured and state what is missing.'
    ]

    if (mcpServerList) {
      lines.push(`Enabled MCP servers for this run: ${mcpServerList}.`)
    }

    return lines.join('\n')
  }

  const lines = [
    `图片附件处理策略（${options.runtimeName}）:`,
    '1. 如果当前模型或运行时本身能直接读取图片附件，优先直接读取。',
    '2. 如果当前模型或运行时不能直接看图，不要停留在“无法查看截图”这一步。',
    '3. 立即检查本轮可用的 MCP 工具，优先选择具备图片理解、OCR、截图分析、多模态读取或视觉提取能力的工具或服务。',
    '4. 将 MCP 的返回结果作为图片观察依据，明确说明信息来源，再继续完成用户任务。',
    '5. 严禁编造视觉细节；如果当前没有合适的视觉工具，就明确说明缺少可用的视觉能力，并指出需要什么能力。'
  ]

  if (mcpServerList) {
    lines.push(`本轮已启用的 MCP 服务: ${mcpServerList}。`)
  }

  return lines.join('\n')
}
