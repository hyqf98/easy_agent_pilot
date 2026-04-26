<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch, toRef } from 'vue'
import MarkdownIt from 'markdown-it'
import hljs from 'highlight.js'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useTypewriterText } from '@/composables/useTypewriterText'
import { useProjectStore } from '@/stores/project'
import { useUIStore } from '@/stores/ui'
import { useNotificationStore } from '@/stores/notification'
import { useFileEditorStore } from '@/modules/fileEditor'

const props = withDefaults(defineProps<{
  content: string
  animate?: boolean
}>(), {
  animate: false
})

const containerRef = ref<HTMLDivElement | null>(null)
const projectStore = useProjectStore()
const uiStore = useUIStore()
const notificationStore = useNotificationStore()
const fileEditorStore = useFileEditorStore()

// 存储代码块原始内容，用于复制功能
const codeBlockContents = ref(new Map<string, string>())
const codeBlockCounter = ref(0)

const WINDOWS_ABSOLUTE_PATH = /^[A-Za-z]:[\\/]/
const EXTERNAL_URL_SCHEME = /^(https?|mailto|tel):/i
const GENERIC_URL_SCHEME = /^[A-Za-z][A-Za-z\d+.-]*:/

function trimCodeFencePadding(value: string): string {
  const lines = value.replace(/\r\n/g, '\n').split('\n')

  while (lines.length > 0 && lines[0].trim().length === 0) {
    lines.shift()
  }

  while (lines.length > 0 && lines[lines.length - 1].trim().length === 0) {
    lines.pop()
  }

  return lines.join('\n')
}

function safeDecodeUriComponent(value: string): string {
  try {
    return decodeURIComponent(value)
  } catch {
    return value
  }
}

function normalizePath(value: string): string {
  return value.replace(/\\/g, '/').replace(/\/+/g, '/')
}

function trimTrailingSlash(value: string): string {
  return value.length > 1 && value.endsWith('/') ? value.slice(0, -1) : value
}

function normalizeComparablePath(value: string): string {
  return trimTrailingSlash(normalizePath(value)).toLowerCase()
}

function stripFileProtocol(value: string): string {
  if (!value.toLowerCase().startsWith('file://')) {
    return value
  }

  const withoutProtocol = value.slice('file://'.length)
  if (!withoutProtocol.startsWith('/')) {
    return withoutProtocol
  }

  const windowsStylePath = withoutProtocol.slice(1)
  if (WINDOWS_ABSOLUTE_PATH.test(windowsStylePath)) {
    return windowsStylePath
  }

  return withoutProtocol
}

function stripLineSuffix(value: string): string {
  return value.replace(/:(\d+)(?::(\d+))?$/, '')
}

function normalizeFileLinkPath(href: string): string {
  const decodedHref = safeDecodeUriComponent(href).trim()
  const withoutHash = decodedHref.replace(/[?#].*$/, '')
  const withoutFileProtocol = stripFileProtocol(withoutHash)
  return stripLineSuffix(withoutFileProtocol)
}

function isLikelyLocalFileHref(href: string): boolean {
  const normalizedHref = normalizeFileLinkPath(href)
  if (!normalizedHref || normalizedHref.startsWith('#') || normalizedHref.startsWith('//')) {
    return false
  }

  if (EXTERNAL_URL_SCHEME.test(normalizedHref)) {
    return false
  }

  if (href.toLowerCase().startsWith('file://')) {
    return true
  }

  return WINDOWS_ABSOLUTE_PATH.test(normalizedHref)
    || normalizedHref.startsWith('/')
    || !GENERIC_URL_SCHEME.test(normalizedHref)
}

function joinProjectPath(projectPath: string, relativePath: string): string {
  return normalizePath(`${trimTrailingSlash(projectPath)}/${relativePath.replace(/^\.?\//, '')}`)
}

function resolveProjectFileTarget(href: string): {
  projectId: string
  projectPath: string
  filePath: string
} | null {
  const normalizedHref = normalizeFileLinkPath(href)
  if (!normalizedHref) {
    return null
  }

  const sortedProjects = [...projectStore.projects]
    .sort((left, right) => normalizeComparablePath(right.path).length - normalizeComparablePath(left.path).length)

  if (WINDOWS_ABSOLUTE_PATH.test(normalizedHref) || normalizedHref.startsWith('/')) {
    const normalizedTarget = normalizeComparablePath(normalizedHref)
    const matchedProject = sortedProjects.find((project) => {
      const normalizedProjectPath = normalizeComparablePath(project.path)
      return normalizedTarget === normalizedProjectPath || normalizedTarget.startsWith(`${normalizedProjectPath}/`)
    })

    if (!matchedProject) {
      return null
    }

    return {
      projectId: matchedProject.id,
      projectPath: matchedProject.path,
      filePath: normalizedHref
    }
  }

  const currentProject = projectStore.currentProject
  if (!currentProject) {
    return null
  }

  return {
    projectId: currentProject.id,
    projectPath: currentProject.path,
    filePath: joinProjectPath(currentProject.path, normalizedHref)
  }
}

function appendTokenClass(
  tokens: Array<{
    attrIndex: (name: string) => number
    attrPush: (attrData: [string, string]) => void
    attrs?: [string, string][] | null
  }>,
  idx: number,
  className: string
): void {
  const classIndex = tokens[idx].attrIndex('class')
  if (classIndex < 0) {
    tokens[idx].attrPush(['class', className])
    return
  }

  if (tokens[idx].attrs) {
    tokens[idx].attrs[classIndex][1] = `${tokens[idx].attrs[classIndex][1]} ${className}`.trim()
  }
}

// 创建 MarkdownIt 实例
const md = new MarkdownIt({
  html: false,
  breaks: true,
  linkify: true,
  typographer: true
})

// 覆盖 fence 渲染规则，生成自定义代码块 HTML
// 不能用 highlight 选项，因为它返回的字符串会被包在 <pre><code> 里，导致非法嵌套
md.renderer.rules.fence = (tokens, idx): string => {
  const token = tokens[idx]
  const lang = (token.info || '').trim()
  const rawCode = token.content || ''
  const normalizedCode = trimCodeFencePadding(rawCode)

  // 生成唯一 ID 并存储原始代码
  const blockId = `code-block-${codeBlockCounter.value++}`
  codeBlockContents.value.set(blockId, normalizedCode)

  // 确定语言标签
  let languageLabel = lang || 'text'
  let highlightedCode: string

  if (lang && hljs.getLanguage(lang)) {
    try {
      highlightedCode = hljs.highlight(normalizedCode, { language: lang, ignoreIllegals: true }).value
    } catch {
      highlightedCode = md.utils.escapeHtml(normalizedCode)
    }
  } else {
    try {
      const result = hljs.highlightAuto(normalizedCode)
      highlightedCode = result.value
      if (result.language) {
        languageLabel = result.language
      }
    } catch {
      highlightedCode = md.utils.escapeHtml(normalizedCode)
    }
  }

  return `<div class="code-block-wrapper" data-code-id="${blockId}">
    <div class="code-block-header">
      <span class="code-block-language">${languageLabel}</span>
      <button class="code-block-copy-btn" data-code-id="${blockId}" title="复制代码">
        <svg class="copy-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
        </svg>
        <svg class="check-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="20 6 9 17 4 12"></polyline>
        </svg>
      </button>
    </div>
    <pre class="hljs"><code>${highlightedCode}</code></pre>
  </div>`
}

// 自定义链接渲染，使用 Tauri opener 在外部浏览器打开
const defaultLinkOpenRender = md.renderer.rules.link_open ||
  ((tokens, idx, options, _env, self) => self.renderToken(tokens, idx, options))

md.renderer.rules.link_open = (tokens, idx, options, env, self): string => {
  const href = tokens[idx].attrGet('href') ?? ''
  const linkKind = isLikelyLocalFileHref(href) ? 'file' : 'external'

  const kindIndex = tokens[idx].attrIndex('data-link-kind')
  if (kindIndex < 0) {
    tokens[idx].attrPush(['data-link-kind', linkKind])
  } else if (tokens[idx].attrs) {
    tokens[idx].attrs[kindIndex][1] = linkKind
  }

  appendTokenClass(tokens, idx, 'markdown-link')
  appendTokenClass(tokens, idx, linkKind === 'file' ? 'file-link' : 'external-link')

  if (linkKind === 'external') {
    const aIndex = tokens[idx].attrIndex('target')
    if (aIndex < 0) {
      tokens[idx].attrPush(['target', '_blank'])
    } else if (tokens[idx].attrs) {
      tokens[idx].attrs[aIndex][1] = '_blank'
    }

    const relIndex = tokens[idx].attrIndex('rel')
    if (relIndex < 0) {
      tokens[idx].attrPush(['rel', 'noopener noreferrer'])
    } else if (tokens[idx].attrs) {
      tokens[idx].attrs[relIndex][1] = 'noopener noreferrer'
    }
  }

  return defaultLinkOpenRender(tokens, idx, options, env, self)
}

const { displayedText } = useTypewriterText(
  toRef(props, 'content'),
  toRef(props, 'animate'),
  { charsPerSecond: 140, maxChunkSize: 24 }
)

const renderedContent = computed(() => md.render(displayedText.value))

// 处理链接点击，使用 Tauri opener
const handleLinkClick = async (e: MouseEvent): Promise<void> => {
  const target = e.target as HTMLElement
  const link = target.closest('a.markdown-link') as HTMLAnchorElement | null

  if (link) {
    e.preventDefault()
    const href = link.getAttribute('href')
    if (href) {
      if (link.dataset.linkKind === 'file') {
        const fileTarget = resolveProjectFileTarget(href)
        if (!fileTarget) {
          notificationStore.warning('无法打开文件', '当前路径未匹配到已导入项目，请确认项目已导入。')
          return
        }

        projectStore.setCurrentProject(fileTarget.projectId)
        uiStore.setAppMode('chat')

        await fileEditorStore.openFile(fileTarget)
        return
      }

      try {
        await openUrl(href)
      } catch (error) {
        console.error('Failed to open URL:', error)
        // 回退到默认行为
        window.open(href, '_blank', 'noopener,noreferrer')
      }
    }
  }
}

// 处理复制按钮点击
const handleCopyClick = async (e: MouseEvent): Promise<void> => {
  const target = e.target as HTMLElement
  const copyBtn = target.closest('.code-block-copy-btn') as HTMLButtonElement | null

  if (copyBtn) {
    e.preventDefault()
    const codeId = copyBtn.dataset.codeId
    if (codeId) {
      const codeContent = codeBlockContents.value.get(codeId)
      if (codeContent) {
        try {
          await navigator.clipboard.writeText(codeContent)
          // 显示复制成功反馈
          copyBtn.classList.add('copied')
          setTimeout(() => {
            copyBtn.classList.remove('copied')
          }, 2000)
        } catch (error) {
          console.error('Failed to copy code:', error)
        }
      }
    }
  }
}

// 处理点击事件（链接和复制按钮）
const handleClick = async (e: MouseEvent): Promise<void> => {
  await handleLinkClick(e)
  await handleCopyClick(e)
}

// 清理代码块内容缓存
const clearCodeBlockContents = (): void => {
  codeBlockContents.value.clear()
  codeBlockCounter.value = 0
}

// 监听内容变化，清理旧的缓存
watch(displayedText, () => {
  clearCodeBlockContents()
})

onMounted(() => {
  // 添加事件委托处理链接点击和复制按钮点击
  if (containerRef.value) {
    containerRef.value.addEventListener('click', handleClick)
  }
})

onUnmounted(() => {
  // 移除事件监听器
  if (containerRef.value) {
    containerRef.value.removeEventListener('click', handleClick)
  }
  // 清理缓存
  clearCodeBlockContents()
})
</script>

<template>
  <div
    ref="containerRef"
    class="markdown-content"
    v-html="renderedContent"
  />
</template>

<style>
/* highlight.js 样式必须放在非 scoped 块中，否则 v-html 渲染的内容无法匹配 */
@import 'highlight.js/styles/github-dark.css';
</style>

<style scoped>
.markdown-content {
  --md-code-bg: linear-gradient(180deg, #f8fbff 0%, #edf4ff 100%);
  --md-code-header-bg: rgba(219, 234, 254, 0.92);
  --md-code-border: rgba(96, 165, 250, 0.28);
  --md-code-fg: #1e293b;
  --md-code-muted: #475569;
  --md-code-hover-bg: rgba(191, 219, 254, 0.95);
  line-height: 1.55;
  color: var(--color-text-primary);
  width: 100%;
}

.markdown-content :deep(h1),
.markdown-content :deep(h2),
.markdown-content :deep(h3),
.markdown-content :deep(h4),
.markdown-content :deep(h5),
.markdown-content :deep(h6) {
  margin: 0.55em 0 0.22em;
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
  line-height: 1.3;
}

.markdown-content :deep(h1) { font-size: 1.5em; }
.markdown-content :deep(h2) { font-size: 1.3em; }
.markdown-content :deep(h3) { font-size: 1.1em; }
.markdown-content :deep(h4) { font-size: 1em; }
.markdown-content :deep(h5) { font-size: 0.9em; }
.markdown-content :deep(h6) { font-size: 0.85em; }

.markdown-content :deep(p) {
  margin: 0.1em 0 0.15em;
  color: inherit;
}

.markdown-content :deep(ul),
.markdown-content :deep(ol) {
  margin: 0.1em 0 0.2em;
  padding-left: 1.5em;
}

.markdown-content :deep(li) {
  margin: 0.04em 0;
  color: inherit;
}

/* 行内代码样式 */
.markdown-content :deep(code) {
  padding: 0.15em 0.38em;
  background-color: color-mix(in srgb, var(--color-primary) 8%, var(--color-bg-secondary));
  border-radius: 4px;
  font-family: var(--font-family-mono);
  font-size: 0.875em;
  font-weight: 450;
  letter-spacing: -0.01em;
  color: var(--color-text-primary);
}

/* 代码块容器样式 */
.markdown-content :deep(pre) {
  margin: 0;
  padding: 0;
  background-color: transparent;
}

/* 代码块包装器样式 */
.markdown-content :deep(.code-block-wrapper) {
  margin: 0 0 0.15em;
  width: 100%;
  max-width: 100%;
  border: 1px solid var(--md-code-border);
  border-radius: var(--radius-md);
  background: var(--md-code-bg);
  box-shadow: 0 4px 12px rgba(148, 163, 184, 0.12);
  overflow: hidden;
}

/* 代码块头部样式 */
.markdown-content :deep(.code-block-header) {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.08rem 0.5rem;
  background: var(--md-code-header-bg);
  border-bottom: 1px solid var(--md-code-border);
  line-height: 1.1;
}

/* 语言标签样式 */
.markdown-content :deep(.code-block-language) {
  font-size: 0.65rem;
  color: var(--md-code-muted);
  text-transform: lowercase;
  font-family: var(--font-family-mono);
  letter-spacing: 0.02em;
  line-height: 1.1;
}

/* 复制按钮样式 */
.markdown-content :deep(.code-block-copy-btn) {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0.14rem;
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--md-code-muted);
  cursor: pointer;
  transition: all 0.2s ease;
}

.markdown-content :deep(.code-block-copy-btn:hover) {
  background-color: var(--md-code-hover-bg);
  color: var(--md-code-fg);
}

.markdown-content :deep(.code-block-copy-btn .check-icon) {
  display: none;
}

.markdown-content :deep(.code-block-copy-btn.copied) {
  color: #4ade80;
}

.markdown-content :deep(.code-block-copy-btn.copied .copy-icon) {
  display: none;
}

.markdown-content :deep(.code-block-copy-btn.copied .check-icon) {
  display: block;
}

.markdown-content :deep(.hljs) {
  color: var(--md-code-fg);
  background: transparent;
}

/* highlight.js 代码块样式 */
.markdown-content :deep(pre.hljs) {
  padding: 0.18rem 0.72rem 0.3rem;
  background: transparent;
  margin: 0;
  border-radius: 0;
}

.markdown-content :deep(pre.hljs code) {
  padding: 0;
  background: none;
  font-size: 0.82rem;
  line-height: 1.38;
}

.markdown-content :deep(.hljs-comment),
.markdown-content :deep(.hljs-quote) {
  color: #64748b;
  font-style: italic;
}

.markdown-content :deep(.hljs-keyword),
.markdown-content :deep(.hljs-selector-tag),
.markdown-content :deep(.hljs-subst) {
  color: #7c3aed;
}

.markdown-content :deep(.hljs-string),
.markdown-content :deep(.hljs-doctag),
.markdown-content :deep(.hljs-template-variable),
.markdown-content :deep(.hljs-variable),
.markdown-content :deep(.hljs-regexp) {
  color: #0f766e;
}

.markdown-content :deep(.hljs-title),
.markdown-content :deep(.hljs-section),
.markdown-content :deep(.hljs-selector-id),
.markdown-content :deep(.hljs-selector-class) {
  color: #2563eb;
}

.markdown-content :deep(.hljs-number),
.markdown-content :deep(.hljs-literal),
.markdown-content :deep(.hljs-symbol),
.markdown-content :deep(.hljs-bullet),
.markdown-content :deep(.hljs-attr) {
  color: #c2410c;
}

.markdown-content :deep(.hljs-type),
.markdown-content :deep(.hljs-built_in),
.markdown-content :deep(.hljs-class .hljs-title) {
  color: #b45309;
}

/* 链接样式 */
.markdown-content :deep(a) {
  color: var(--color-primary);
  text-decoration: none;
  cursor: pointer;
}

.markdown-content :deep(a:hover) {
  text-decoration: underline;
}

.markdown-content :deep(a.markdown-link) {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}

.markdown-content :deep(a.external-link) {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}

.markdown-content :deep(a.external-link::after) {
  content: '↗';
  font-size: 0.8em;
  opacity: 0.7;
}

/* 引用块样式 */
.markdown-content :deep(blockquote) {
  margin: 0.5em 0;
  padding: var(--spacing-2) var(--spacing-4);
  border-left: 3px solid var(--color-border);
  background-color: var(--color-bg-secondary);
  color: var(--color-text-secondary);
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
}

/* 粗体和斜体 */
.markdown-content :deep(strong) {
  font-weight: 550;
  color: var(--color-text-primary);
}

.markdown-content :deep(em) {
  font-style: italic;
}

/* 删除线 */
.markdown-content :deep(del) {
  text-decoration: line-through;
  color: var(--color-text-tertiary);
}

/* 表格样式 */
.markdown-content :deep(table) {
  width: 100%;
  margin: 1em 0;
  border-collapse: collapse;
  font-size: var(--font-size-sm);
}

.markdown-content :deep(th),
.markdown-content :deep(td) {
  padding: var(--spacing-2) var(--spacing-3);
  border: 1px solid var(--color-border);
  text-align: left;
  color: inherit;
}

.markdown-content :deep(th) {
  background-color: var(--color-bg-secondary);
  font-weight: var(--font-weight-medium);
}

.markdown-content :deep(tr:nth-child(even)) {
  background-color: var(--color-bg-secondary);
}

/* 水平分隔线 */
.markdown-content :deep(hr) {
  margin: var(--spacing-4) 0;
  border: none;
  border-top: 1px solid var(--color-border);
}

/* 图片样式 */
.markdown-content :deep(img) {
  max-width: 100%;
  height: auto;
  border-radius: var(--radius-md);
  margin: var(--spacing-2) 0;
}

:global([data-theme='dark']) .markdown-content,
:global(.dark) .markdown-content {
  --md-code-bg: linear-gradient(180deg, #0f172a 0%, #111827 100%);
  --md-code-header-bg: rgba(30, 41, 59, 0.92);
  --md-code-border: rgba(148, 163, 184, 0.18);
  --md-code-fg: #e2e8f0;
  --md-code-muted: #94a3b8;
  --md-code-hover-bg: rgba(51, 65, 85, 0.96);
  color: #e5e7eb;
}

:global([data-theme='dark']) .markdown-content :deep(h1),
:global(.dark) .markdown-content :deep(h1),
:global([data-theme='dark']) .markdown-content :deep(h2),
:global(.dark) .markdown-content :deep(h2),
:global([data-theme='dark']) .markdown-content :deep(h3),
:global(.dark) .markdown-content :deep(h3),
:global([data-theme='dark']) .markdown-content :deep(h4),
:global(.dark) .markdown-content :deep(h4),
:global([data-theme='dark']) .markdown-content :deep(h5),
:global(.dark) .markdown-content :deep(h5),
:global([data-theme='dark']) .markdown-content :deep(h6),
:global(.dark) .markdown-content :deep(h6),
:global([data-theme='dark']) .markdown-content :deep(strong),
:global(.dark) .markdown-content :deep(strong) {
  color: #f8fafc;
}

:global([data-theme='dark']) .markdown-content :deep(code),
:global(.dark) .markdown-content :deep(code) {
  background-color: rgba(148, 163, 184, 0.14);
}

:global([data-theme='dark']) .markdown-content :deep(blockquote),
:global(.dark) .markdown-content :deep(blockquote) {
  border-left-color: rgba(148, 163, 184, 0.3);
  background-color: rgba(30, 41, 59, 0.72);
  color: #cbd5e1;
}

:global([data-theme='dark']) .markdown-content :deep(th),
:global(.dark) .markdown-content :deep(th) {
  background-color: rgba(30, 41, 59, 0.76);
}

:global([data-theme='dark']) .markdown-content :deep(tr:nth-child(even)),
:global(.dark) .markdown-content :deep(tr:nth-child(even)) {
  background-color: rgba(30, 41, 59, 0.42);
}
</style>
