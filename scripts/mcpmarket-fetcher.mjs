import fs from 'node:fs'
import path from 'node:path'
import { chromium } from 'playwright'

const [, , mode, rawPayload = '{}'] = process.argv

if (!mode) {
  console.error('Missing mode')
  process.exit(1)
}

const payload = JSON.parse(rawPayload)
const locale = payload.locale || 'zh'
const MCPMARKET_SITE_ORIGIN = 'https://mcpmarket.com'
const MODELSCOPE_SITE_ORIGIN = 'https://modelscope.cn'

function browserCandidates() {
  const platform = process.platform
  if (platform === 'darwin') {
    return [
      '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
      '/Applications/Chromium.app/Contents/MacOS/Chromium',
      '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
      '/Applications/Brave Browser.app/Contents/MacOS/Brave Browser',
    ]
  }
  if (platform === 'win32') {
    const local = process.env.LOCALAPPDATA || ''
    const programFiles = process.env.PROGRAMFILES || 'C:\\Program Files'
    const programFilesX86 = process.env['PROGRAMFILES(X86)'] || 'C:\\Program Files (x86)'
    return [
      path.join(programFiles, 'Google/Chrome/Application/chrome.exe'),
      path.join(programFilesX86, 'Google/Chrome/Application/chrome.exe'),
      path.join(local, 'Google/Chrome/Application/chrome.exe'),
      path.join(programFiles, 'Microsoft/Edge/Application/msedge.exe'),
      path.join(programFilesX86, 'Microsoft/Edge/Application/msedge.exe'),
      path.join(local, 'Microsoft/Edge/Application/msedge.exe'),
    ]
  }
  return [
    '/usr/bin/google-chrome',
    '/usr/bin/google-chrome-stable',
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
    '/usr/bin/microsoft-edge',
    '/snap/bin/chromium',
  ]
}

function resolveBrowserExecutable() {
  return browserCandidates().find(candidate => fs.existsSync(candidate)) || null
}

function buildRouteTree(segments) {
  let node = ['__PAGE__', {}, null, null]
  for (let index = segments.length - 1; index >= 0; index -= 1) {
    node = [segments[index], { children: node }, null, null]
  }
  return [
    '',
    {
      children: [
        ['locale', locale, 'd'],
        { children: node },
        null,
        null,
      ],
    },
    null,
    null,
    true,
  ]
}

function routeForMode(currentMode, currentPayload) {
  switch (currentMode) {
    case 'mcp-list': {
      const page = Math.max(1, Number(currentPayload.page || 1))
      const search = String(currentPayload.search || '').trim()
      const categorySlug = String(currentPayload.category_slug || '').trim()

      if (search) {
        return {
          path: `/${locale}/search?q=${encodeURIComponent(search)}${page > 1 ? `&page=${page}` : ''}`,
          page,
        }
      }

      if (categorySlug) {
        return {
          path: `/${locale}/categories/${encodeURIComponent(categorySlug)}?page=${page}`,
          page,
        }
      }

      return {
        path: `/${locale}/server?page=${page}`,
        page,
      }
    }
    case 'mcp-detail':
      return {
        path: `/${locale}/server/${currentPayload.slug}`,
        segments: ['server', ['slug', currentPayload.slug, 'd']],
      }
    case 'skill-list': {
      const page = Math.max(1, Number(currentPayload.page || 1))
      const search = String(currentPayload.search || '').trim()
      const categorySlug = String(currentPayload.category_slug || '').trim()

      if (search) {
        return {
          path: `/${locale}/search?q=${encodeURIComponent(search)}&type=skills${page > 1 ? `&page=${page}` : ''}`,
          page,
        }
      }

      if (categorySlug) {
        return {
          path: `/${locale}/tools/skills/categories/${encodeURIComponent(categorySlug)}?page=${page}`,
          page,
        }
      }

      return {
        path: `/${locale}/tools/skills/all?page=${page}`,
        page,
      }
    }
    case 'skill-detail':
    case 'skill-archive':
      return {
        path: `/${locale}/tools/skills/${currentPayload.slug}`,
      }
    case 'modelscope-mcp-list': {
      const page = Math.max(1, Number(currentPayload.page || 1))
      const search = String(currentPayload.search || '').trim()
      const categorySlug = String(currentPayload.category_slug || '').trim()
      const params = new URLSearchParams()

      params.set('page', String(page))
      if (search) {
        params.set('name', search)
      } else if (categorySlug) {
        params.set('category', categorySlug)
      }

      return {
        origin: MODELSCOPE_SITE_ORIGIN,
        path: `/mcp?${params.toString()}`,
        page,
      }
    }
    case 'modelscope-mcp-detail': {
      const slug = String(currentPayload.slug || '')
        .split('/')
        .filter(Boolean)
        .map(segment => encodeURIComponent(segment))
        .join('/')
      return {
        origin: MODELSCOPE_SITE_ORIGIN,
        path: `/mcp/servers/${slug}`,
      }
    }
    default:
      throw new Error(`Unsupported mode: ${currentMode}`)
  }
}

function resolveOrigin(route) {
  return route.origin || MCPMARKET_SITE_ORIGIN
}

function joinUrl(origin, href) {
  if (!href) return null
  return href.startsWith('http') ? href : `${origin}${href}`
}

function parseNumber(value) {
  const normalized = String(value || '').replace(/,/g, '').trim()
  if (!normalized) return null
  const parsed = Number.parseInt(normalized, 10)
  return Number.isFinite(parsed) ? parsed : null
}

function parseCompactNumber(value) {
  const normalized = String(value || '').replace(/,/g, '').trim().toLowerCase()
  if (!normalized) return null

  const match = normalized.match(/^([\d.]+)\s*([kmb])?$/i)
  if (!match) {
    return parseNumber(normalized)
  }

  const base = Number.parseFloat(match[1])
  if (!Number.isFinite(base)) {
    return null
  }

  const multiplier = match[2] === 'k'
    ? 1_000
    : match[2] === 'm'
      ? 1_000_000
      : match[2] === 'b'
        ? 1_000_000_000
        : 1

  return Math.round(base * multiplier)
}

async function waitForStableSkillsPage(page, selector) {
  await page.waitForLoadState('domcontentloaded', { timeout: 45000 })
  if (selector) {
    try {
      await page.waitForSelector(selector, { timeout: 10000 })
    } catch {
      // The site streams content progressively; fall back to a fixed settle delay.
    }
  }
  await page.waitForTimeout(1200)
}

async function collectSkillCategories(context) {
  const page = await context.newPage()
  try {
    await page.goto(`${MCPMARKET_SITE_ORIGIN}/${locale}/tools/skills`, { waitUntil: 'domcontentloaded', timeout: 45000 })
    await page.waitForTimeout(1200)
    return await page.evaluate(() => {
      return Array.from(document.querySelectorAll('a[href*="/tools/skills/categories/"]'))
        .map(link => {
          const label = link.textContent?.trim() || ''
          const href = link.getAttribute('href') || ''
          const slug = href.split('/').filter(Boolean).pop() || ''
          if (!label || label === '全部' || !slug || slug === 'categories') {
            return null
          }
          return {
            label,
            value: slug,
            slug,
            count: null,
          }
        })
        .filter(Boolean)
    })
  } finally {
    await page.close()
  }
}

async function collectMcpCategories(context) {
  const page = await context.newPage()
  try {
    await page.goto(`${MCPMARKET_SITE_ORIGIN}/${locale}`, { waitUntil: 'domcontentloaded', timeout: 45000 })
    await page.waitForTimeout(1200)
    return await page.evaluate(() => {
      return Array.from(document.querySelectorAll('a[href*="/categories"]'))
        .map(link => {
          const label = link.textContent?.trim() || ''
          const href = link.getAttribute('href') || ''
          const parts = href.split('/').filter(Boolean)
          const categoryIndex = parts.lastIndexOf('categories')
          const slug = categoryIndex >= 0 ? parts[categoryIndex + 1] || '' : ''
          if (!label || label === '全部' || label.startsWith('查看所有') || !slug) {
            return null
          }
          return {
            label,
            value: slug,
            slug,
            count: null,
          }
        })
        .filter(Boolean)
    })
  } finally {
    await page.close()
  }
}

async function parseMcpList(page, context, currentPayload) {
  const response = await page.evaluate((rawPayload) => {
    const requestedCategory = String(rawPayload.category_slug || '').trim()
    const requestedCategoryLabel = String(rawPayload.category || '').trim()

    const parseNumber = value => {
      const normalized = String(value || '').replace(/,/g, '').trim()
      if (!normalized) return null
      const parsed = Number.parseInt(normalized, 10)
      return Number.isFinite(parsed) ? parsed : null
    }

    const textContent = node => node?.textContent?.replace(/\s+/g, ' ').trim() || ''

    const parseCurrentCategory = () => {
      if (!requestedCategory) {
        return null
      }

      const heading = textContent(document.querySelector('h1'))
      const normalizedHeading = heading.replace(/\s*MCP\s*服务器.*/i, '').trim()
      return {
        label: normalizedHeading || requestedCategoryLabel || requestedCategory,
        slug: requestedCategory,
      }
    }

    const currentCategory = parseCurrentCategory()
    const resultsRoot = Array.from(document.querySelectorAll('main')).at(-1) || document
    const cards = Array.from(resultsRoot.querySelectorAll('a[href*="/server/"]'))
      .filter(link => {
        const href = link.getAttribute('href') || ''
        return href.includes('/server/') && Boolean(link.querySelector('h3'))
      })

    const items = cards.map(link => {
      const href = link.getAttribute('href') || ''
      const slug = href.split('/').filter(Boolean).pop() || ''
      const name = textContent(link.querySelector('h3')) || slug
      const description = textContent(link.querySelector('p'))
      const author = link.querySelector('img[alt]')?.getAttribute('alt')?.trim() || ''
      const textNodes = Array.from(link.querySelectorAll('*'))
        .filter(node => node.children.length === 0)
        .map(node => textContent(node))
        .filter(Boolean)
      const stars = parseNumber(textContent(link).match(/(\d[\d,]*)\s*$/)?.[1] || '')
      const categoryLabel = currentCategory?.label
        || textNodes.find(value => value !== author && value !== name && value !== description && !/^\d[\d,]*$/.test(value))
        || ''

      return {
        id: slug,
        slug,
        name,
        description,
        author,
        category: categoryLabel,
        tags: categoryLabel ? [categoryLabel] : [],
        transport_type: 'stdio',
        install_command: null,
        install_args: [],
        env_template: {},
        logo: null,
        downloads: stars,
        rating: null,
        repository_url: null,
        source_market: 'mcpmarket',
        stars,
      }
    })

    const paginationMatches = Array.from(document.querySelectorAll('a'))
      .map(link => textContent(link))
      .map(text => text.match(/第\s*(\d+)\s*页，共\s*(\d*)\s*页/))
      .filter(Boolean)
      .map(match => ({
        page: Number.parseInt(match[1], 10),
        total: Number.parseInt(match[2], 10),
      }))
      .filter(match => Number.isFinite(match.page))
    const page = paginationMatches.find(match => match.page === Math.max(1, Number(rawPayload.page || 1)))?.page
      || Math.max(1, Number(rawPayload.page || 1))
    const totalPages = paginationMatches
      .map(match => match.total)
      .filter(total => Number.isFinite(total) && total > 0)
    const resolvedTotalPages = totalPages.length > 0 ? Math.min(...totalPages) : null
    const pageText = textContent(document.body)
    const hasMore = pageText.includes('Load More Results')
      || pageText.includes('More available')
      || (resolvedTotalPages ? page < resolvedTotalPages : false)

    return {
      items,
      total: items.length,
      page,
      has_more: hasMore,
    }
  }, currentPayload)

  const categories = await collectMcpCategories(context)
  return {
    ...response,
    categories,
  }
}

async function parseSkillsList(page, context, currentPayload) {
  const response = await page.evaluate((rawPayload) => {
    const currentPath = window.location.pathname
    const requestedCategory = String(rawPayload.category_slug || '').trim()
    const parseNumber = value => {
      const normalized = String(value || '').replace(/,/g, '').trim()
      if (!normalized) return null
      const parsed = Number.parseInt(normalized, 10)
      return Number.isFinite(parsed) ? parsed : null
    }

    const textContent = (node) => node?.textContent?.replace(/\s+/g, ' ').trim() || ''

    const parseCurrentCategory = () => {
      if (requestedCategory) {
        const button = Array.from(document.querySelectorAll('button'))
          .find(element => textContent(element).startsWith(rawPayload.category || ''))
        const selectedLabel = button ? textContent(button).replace(/\s+\d[\d,]*$/, '') : ''
        return {
          label: selectedLabel || String(rawPayload.category || '').trim(),
          slug: requestedCategory,
        }
      }

      if (currentPath.includes('/tools/skills/categories/')) {
        const slug = currentPath.split('/').filter(Boolean).pop() || ''
        const label =
          textContent(document.querySelector('h1')) ||
          textContent(document.querySelector('main')) ||
          slug
        return { label: label.replace(/\s+Claude.*/, '').trim(), slug }
      }

      return { label: '', slug: '' }
    }

    const currentCategory = parseCurrentCategory()
    const resultsRoot = Array.from(document.querySelectorAll('main')).at(-1) || document
    const cards = Array.from(resultsRoot.querySelectorAll('a[href*="/tools/skills/"]'))
      .filter(link => {
        const href = link.getAttribute('href') || ''
        if (!href.includes('/tools/skills/')) return false
        const lastSegment = href.split('/').filter(Boolean).pop() || ''
        if (['all', 'what-are-skills', 'how-to-install', 'leaderboard'].includes(lastSegment)) {
          return false
        }
        return Boolean(link.querySelector('h3'))
      })

    const items = cards.map(link => {
      const href = link.getAttribute('href') || ''
      const slug = href.split('/').filter(Boolean).pop() || ''
      const name = textContent(link.querySelector('h3')) || slug
      const description = textContent(link.querySelector('p'))
      const author = link.querySelector('img[alt]')?.getAttribute('alt')?.trim() || ''
      const stars = parseNumber(textContent(link).match(/(\d[\d,]*)\s*$/)?.[1] || '')
      const category = currentCategory.label || ''
      const categorySlug = currentCategory.slug || ''

      return {
        id: slug,
        slug,
        name,
        description,
        path: href,
        author,
        category,
        category_slug: categorySlug || null,
        tags: category ? [category] : [],
        trigger_scenario: description,
        source_market: 'mcpmarket',
        repository_url: null,
        downloads: stars,
        rating: null,
        stars,
      }
    })

    const paginationMatches = Array.from(document.querySelectorAll('a'))
      .map(link => textContent(link))
      .map(text => text.match(/第\s*(\d+)\s*页，共\s*(\d+)\s*页/))
      .filter(Boolean)
      .map(match => ({
        page: Number.parseInt(match[1], 10),
        total: Number.parseInt(match[2], 10),
      }))
      .filter(match => Number.isFinite(match.page) && Number.isFinite(match.total))
    const page = paginationMatches.find(match => match.page === Math.max(1, Number(rawPayload.page || 1)))?.page
      || Math.max(1, Number(rawPayload.page || 1))
    const totalPages = paginationMatches.length > 0
      ? Math.min(...paginationMatches.map(match => match.total))
      : null
    const pageText = textContent(document.body)
    const hasMore = pageText.includes('More available') || (totalPages ? page < totalPages : false)

    return {
      items,
      total: items.length,
      page,
      has_more: hasMore,
    }
  }, currentPayload)

  const categories = await collectSkillCategories(context)
  return {
    ...response,
    categories,
  }
}

async function parseSkillDetail(page) {
  const base = await page.evaluate(() => {
    const textContent = (node) => node?.textContent?.replace(/\s+/g, ' ').trim() || ''
    const parseNumber = value => {
      const normalized = String(value || '').replace(/,/g, '').trim()
      if (!normalized) return null
      const parsed = Number.parseInt(normalized, 10)
      return Number.isFinite(parsed) ? parsed : null
    }
    const heading = document.querySelector('h1')
    if (!heading) {
      throw new Error('Failed to parse MCP Market skill detail')
    }

    const githubLinks = Array.from(document.querySelectorAll('a[href^="https://github.com/"]'))
    const repositoryLink = githubLinks.find(link => textContent(link) === 'GitHub') || githubLinks[githubLinks.length - 1]
    const authorLink = Array.from(document.querySelectorAll('a[href^="https://github.com/"]'))
      .find(link => {
        const text = textContent(link)
        return Boolean(text) && text !== 'GitHub'
      })
    const categoryLink = Array.from(document.querySelectorAll('a[href*="/categories/"]'))
      .find(link => (link.getAttribute('href') || '').includes('/categories/'))
    const description = Array.from(document.querySelectorAll('p'))
      .map(node => textContent(node))
      .find(text => text && text !== 'For use in Claude.ai and ChatGPT') || ''
    const breadcrumbPath = window.location.pathname
    const slug = breadcrumbPath.split('/').filter(Boolean).pop() || ''
    const statsText = textContent(heading.parentElement?.parentElement)
    const stars = parseNumber(statsText.match(/(\d[\d,]*)/)?.[1] || '')
    const aboutPanel = document.querySelector('[role="tabpanel"]')
    const fullDescription = aboutPanel?.innerText?.trim() || textContent(aboutPanel)

    return {
      id: slug,
      slug,
      name: textContent(heading) || slug,
      description,
      path: breadcrumbPath,
      author: textContent(authorLink),
      category: textContent(categoryLink),
      category_slug: categoryLink ? (categoryLink.getAttribute('href') || '').split('/').filter(Boolean).pop() || null : null,
      tags: textContent(categoryLink) ? [textContent(categoryLink)] : [],
      trigger_scenario: description,
      source_market: 'mcpmarket',
      repository_url: repositoryLink?.getAttribute('href') || null,
      downloads: stars,
      rating: null,
      stars,
      full_description: fullDescription || description,
      download_url: document.querySelector('a[href*="/api/skills/download"]')?.getAttribute('href') || null,
    }
  })

  let skillContent = null
  const clickedSkillTab = await page.evaluate(() => {
    const tab = Array.from(document.querySelectorAll('[role="tab"]'))
      .find(element => element.textContent?.includes('SKILL.md'))
    if (!tab) {
      return false
    }
    tab.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    return true
  })

  if (clickedSkillTab) {
    await page.waitForTimeout(800)
    skillContent = await page.evaluate(() => {
      const article = document.querySelector('[role="tabpanel"] article')
      const tabPanel = article || document.querySelector('[role="tabpanel"]')
      const content = tabPanel?.innerText?.trim() || tabPanel?.textContent?.trim() || ''
      return content || null
    })
  }

  return {
    ...base,
    skill_content: skillContent,
  }
}

async function downloadSkillArchive(context, slug) {
  const page = await context.newPage()
  try {
    await page.goto(`${MCPMARKET_SITE_ORIGIN}/${locale}/tools/skills/${slug}`, { waitUntil: 'domcontentloaded', timeout: 45000 })
    await waitForStableSkillsPage(page, 'h1')
    const downloadUrl = await page.evaluate(() => {
      return document.querySelector('a[href*="/api/skills/download"]')?.getAttribute('href') || null
    })
    if (!downloadUrl) {
      throw new Error('Skill archive download URL is not available')
    }

    const absoluteDownloadUrl = joinUrl(MCPMARKET_SITE_ORIGIN, downloadUrl)
    const response = await fetch(absoluteDownloadUrl)
    if (!response.ok) {
      throw new Error(`Failed to download skill archive: ${response.status}`)
    }

    const disposition = response.headers.get('content-disposition') || ''
    const fileNameMatch = disposition.match(/filename="([^"]+)"/i)
    const fileName = fileNameMatch ? fileNameMatch[1] : `${slug}.zip`
    const buffer = Buffer.from(await response.arrayBuffer())

    return {
      slug,
      download_url: absoluteDownloadUrl,
      file_name: fileName,
      zip_base64: buffer.toString('base64'),
    }
  } finally {
    await page.close()
  }
}

async function waitForStableModelScopePage(page, selector) {
  await page.waitForLoadState('domcontentloaded', { timeout: 45000 })
  if (selector) {
    try {
      await page.waitForSelector(selector, { timeout: 15000 })
    } catch {
      // ModelScope data renders progressively. Continue with a settle delay.
    }
  }
  await page.waitForTimeout(1500)
}

async function parseModelScopeMcpList(page, currentPayload) {
  return await page.evaluate(rawPayload => {
    const textContent = node => node?.textContent?.replace(/\s+/g, ' ').trim() || ''
    const parseNumber = value => {
      const normalized = String(value || '').replace(/,/g, '').trim()
      if (!normalized) return null
      const parsed = Number.parseInt(normalized, 10)
      return Number.isFinite(parsed) ? parsed : null
    }
    const parseCompactNumber = value => {
      const normalized = String(value || '').replace(/,/g, '').trim().toLowerCase()
      if (!normalized) return null
      const match = normalized.match(/^([\d.]+)\s*([kmb])?$/i)
      if (!match) {
        return parseNumber(normalized)
      }
      const base = Number.parseFloat(match[1])
      if (!Number.isFinite(base)) return null
      const multiplier = match[2] === 'k'
        ? 1000
        : match[2] === 'm'
          ? 1000000
          : match[2] === 'b'
            ? 1000000000
            : 1
      return Math.round(base * multiplier)
    }
    const isMetric = value => /^[\d,.]+(?:\.\d+)?\s*[kmb]?$/i.test(String(value || '').trim())
    const isDate = value => /^\d{4}\.\d{2}\.\d{2}$/.test(String(value || '').trim())

    const categories = Array.from(document.querySelectorAll('[data-autolog*="c3=mcpCategorySelect"]'))
      .map(node => {
        const data = node.getAttribute('data-autolog') || ''
        const slug = data.match(/c4=([^&]+)/)?.[1] || ''
        const parts = Array.from(node.querySelectorAll('*'))
          .map(child => textContent(child))
          .filter(Boolean)
        const label = parts.find(value => !/^\d[\d,]*$/.test(value)) || ''
        const count = parseNumber(parts.find(value => /^\d[\d,]*$/.test(value)) || '')
        if (!label || !slug) {
          return null
        }
        return {
          label,
          value: slug,
          slug,
          count,
        }
      })
      .filter(Boolean)

    const cards = Array.from(document.querySelectorAll('a[href*="/mcp/servers/"]'))
      .filter(link => {
        const href = link.getAttribute('href') || ''
        return href.startsWith('/mcp/servers/') && textContent(link).length > 0
      })

    const items = cards.map(link => {
      const href = link.getAttribute('href') || ''
      const slug = href.replace(/^\/mcp\/servers\//, '').replace(/\/+$/, '')
      const lines = String(link.innerText || '')
        .split('\n')
        .map(value => value.trim())
        .filter(Boolean)

      const name = lines[0] || slug
      const typeIndex = lines.findIndex(value => value === 'Hosted' || value === 'Local')
      const type = typeIndex >= 0 ? lines[typeIndex] : 'Hosted'
      const authorIndex = lines.findIndex(value => value.startsWith('@'))
      const author = authorIndex >= 0 ? lines[authorIndex] : ''
      const metricsStartIndex = lines.findIndex((value, index) => index > typeIndex && isMetric(value) && !isDate(value))
      const contentLines = lines.slice(
        typeIndex >= 0 ? typeIndex + 1 : 1,
        metricsStartIndex >= 0 ? metricsStartIndex : undefined
      )
      const description = contentLines.find(value =>
        !value.startsWith('@') &&
        !value.startsWith('开发者：') &&
        !isMetric(value) &&
        !/license/i.test(value) &&
        !value.includes('开源协议') &&
        value.length > 12
      ) || ''
      const tagCandidates = contentLines.filter(value =>
        value !== description &&
        !value.startsWith('@') &&
        !value.startsWith('开发者：') &&
        !isMetric(value)
      )
      const category = tagCandidates.find(value => value && !/license|协议/i.test(value)) || ''
      const tags = Array.from(new Set(tagCandidates.filter(Boolean)))
      const metrics = lines
        .slice(authorIndex >= 0 ? authorIndex + 1 : 0)
        .map(value => parseCompactNumber(value))
        .filter(value => Number.isFinite(value))

      return {
        id: slug,
        slug,
        name,
        description,
        author,
        category,
        tags,
        transport_type: type === 'Hosted' ? 'http' : 'stdio',
        install_command: null,
        install_args: [],
        env_template: {},
        logo: null,
        downloads: metrics[0] ?? null,
        rating: null,
        repository_url: null,
        source_market: 'modelscope',
        stars: metrics[1] ?? metrics[0] ?? null,
      }
    })

    const bodyText = textContent(document.body)
    const params = new URLSearchParams(window.location.search)
    const page = Math.max(1, Number.parseInt(params.get('page') || String(rawPayload.page || 1), 10) || 1)
    const totalMatch = bodyText.match(/共找到\s*([\d,]+)\s*个结果/) || bodyText.match(/搜索MCP服务（共([\d,]+)个）/)
    const total = parseNumber(totalMatch?.[1] || '') || items.length
    const hasMore = total > page * items.length

    return {
      items,
      total,
      page,
      has_more: hasMore,
      categories,
    }
  }, currentPayload)
}

async function parseModelScopeMcpDetail(page) {
  return await page.evaluate(() => {
    const textContent = node => node?.textContent?.replace(/\s+/g, ' ').trim() || ''
    const parseNumber = value => {
      const normalized = String(value || '').replace(/,/g, '').trim()
      if (!normalized) return null
      const parsed = Number.parseInt(normalized, 10)
      return Number.isFinite(parsed) ? parsed : null
    }
    const parseCompactNumber = value => {
      const normalized = String(value || '').replace(/,/g, '').trim().toLowerCase()
      if (!normalized) return null
      const match = normalized.match(/^([\d.]+)\s*([kmb])?$/i)
      if (!match) {
        return parseNumber(normalized)
      }
      const base = Number.parseFloat(match[1])
      if (!Number.isFinite(base)) return null
      const multiplier = match[2] === 'k'
        ? 1000
        : match[2] === 'm'
          ? 1000000
          : match[2] === 'b'
            ? 1000000000
            : 1
      return Math.round(base * multiplier)
    }
    const isMetric = value => /^[\d,.]+(?:\.\d+)?\s*[kmb]?$/i.test(String(value || '').trim())
    const isDate = value => /^\d{4}\.\d{2}\.\d{2}$/.test(String(value || '').trim())

    const main = document.querySelector('main')
    if (!main) {
      throw new Error('Failed to parse ModelScope MCP detail')
    }

    const slug = decodeURIComponent(window.location.pathname.replace(/^\/mcp\/servers\//, '').replace(/\/+$/, ''))
    const lines = String(main.innerText || '')
      .split('\n')
      .map(value => value.trim())
      .filter(Boolean)
    const detailTabIndex = lines.indexOf('服务详情')
    const headerTexts = detailTabIndex > 0 ? lines.slice(0, detailTabIndex) : lines.slice(0, 24)
    const title = headerTexts.find(value => value && value !== 'Hosted' && value !== 'Local' && !value.startsWith('@')) || slug
    const type = headerTexts.find(value => value === 'Hosted' || value === 'Local') || 'Hosted'
    const description = headerTexts
      .find(value => value.startsWith('该服务器') || value.startsWith('一个') || value.length > 30) || ''
    const author = headerTexts.find(value => value.startsWith('@')) || ''
    const tags = headerTexts
      .filter(value =>
        value &&
        value !== title &&
        value !== type &&
        value !== author &&
        value !== description &&
        value !== 'GitHub' &&
        value !== '合集' &&
        !value.startsWith('/') &&
        !value.endsWith('：') &&
        !value.startsWith('开发者：') &&
        !isDate(value) &&
        !isMetric(value) &&
        !['服务详情', '工具测试', '交流反馈', '可部署', '服务配置', 'Remote', 'Stdio', '连接', '立即部署'].includes(value)
      )
      .filter((value, index, list) => list.indexOf(value) === index)
    const category = tags.find(value => !/license/i.test(value) && !value.includes('开源协议')) || ''
    const repositoryUrl = Array.from(document.querySelectorAll('a[href^="https://github.com/"]'))
      .find(link => textContent(link).includes('GitHub'))
      ?.getAttribute('href') || null
    const metrics = headerTexts
      .filter(value => isMetric(value) && !isDate(value))
      .map(value => parseCompactNumber(value))
      .filter(value => Number.isFinite(value))
    const contentHeading = Array.from(main.querySelectorAll('h1, h2, h3'))
      .find(node => textContent(node).includes('获取 MCP 服务器'))
    const contentRoot = contentHeading?.parentElement || main
    const fullDescription = contentRoot.innerText?.trim() || description
    const configBlocks = Array.from(main.querySelectorAll('code'))
      .map(node => textContent(node))
      .filter(Boolean)
    const configBlock = configBlocks.find(value => value.includes('"command"') && value.includes('"args"')) || ''
    const commandMatch = configBlock.match(/"command"\s*:\s*"([^"]+)"/)
    const argsMatch = configBlock.match(/"args"\s*:\s*\[([\s\S]*?)\]/)
    const installArgs = argsMatch
      ? Array.from(argsMatch[1].matchAll(/"([^"]+)"/g)).map(match => match[1])
      : []

    return {
      id: slug,
      slug,
      name: title,
      description,
      author,
      category,
      tags: tags.slice(0, 8),
      transport_type: type === 'Hosted' ? 'http' : 'stdio',
      install_command: commandMatch?.[1] || null,
      install_args: installArgs,
      env_template: {},
      logo: null,
      downloads: metrics[0] ?? null,
      rating: null,
      repository_url: repositoryUrl,
      source_market: 'modelscope',
      stars: metrics[1] ?? metrics[0] ?? null,
      full_description: fullDescription,
      readme_excerpt: fullDescription.slice(0, 4000) || null,
    }
  })
}

async function parseMcpWithRsc(page, currentMode, currentRoute) {
  return await page.evaluate(
    async ({ mode: activeMode, currentRoute: route }) => {
      const decodeJsonString = value => {
        if (typeof value !== 'string') return ''
        try {
          return JSON.parse(`"${value.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`)
        } catch {
          return value
        }
      }

      const decodeHtmlEntities = value =>
        String(value || '')
          .replace(/&quot;/g, '"')
          .replace(/&#39;/g, "'")
          .replace(/&#x27;/g, "'")
          .replace(/&amp;/g, '&')
          .replace(/&lt;/g, '<')
          .replace(/&gt;/g, '>')
          .replace(/&nbsp;/g, ' ')

      const stripHtml = value =>
        decodeHtmlEntities(String(value || ''))
          .replace(/<style[\s\S]*?<\/style>/gi, ' ')
          .replace(/<script[\s\S]*?<\/script>/gi, ' ')
          .replace(/<[^>]+>/g, ' ')
          .replace(/\s+/g, ' ')
          .trim()

      const parseNumber = value => {
        const normalized = String(value || '').replace(/,/g, '').trim()
        if (!normalized) return null
        const parsed = Number.parseInt(normalized, 10)
        return Number.isFinite(parsed) ? parsed : null
      }

      const buildRouteTreeLocal = segments => {
        let node = ['__PAGE__', {}, null, null]
        for (let index = segments.length - 1; index >= 0; index -= 1) {
          node = [segments[index], { children: node }, null, null]
        }
        return [
          '',
          {
            children: [
              ['locale', 'zh', 'd'],
              { children: node },
              null,
              null,
            ],
          },
          null,
          null,
          true,
        ]
      }

      const fetchRsc = async () => {
        const response = await fetch(`${route.path}${route.path.includes('?') ? '&' : '?'}_rsc=${Math.random().toString(36).slice(2)}`, {
          headers: {
            rsc: '1',
            'next-router-state-tree': encodeURIComponent(JSON.stringify(buildRouteTreeLocal(route.segments))),
            'next-url': route.path,
          },
        })

        if (!response.ok) {
          throw new Error(`Failed to fetch MCP Market RSC payload: ${response.status}`)
        }

        return response.text()
      }

      const extractBalanced = (text, startIndex, openChar = '{', closeChar = '}') => {
        let depth = 0
        let inString = false
        let escaped = false
        for (let index = startIndex; index < text.length; index += 1) {
          const char = text[index]
          if (inString) {
            if (escaped) {
              escaped = false
            } else if (char === '\\') {
              escaped = true
            } else if (char === '"') {
              inString = false
            }
            continue
          }

          if (char === '"') {
            inString = true
            continue
          }

          if (char === openChar) {
            depth += 1
          } else if (char === closeChar) {
            depth -= 1
            if (depth === 0) {
              return text.slice(startIndex, index + 1)
            }
          }
        }
        return null
      }

      const extractObjectAroundMarker = (text, marker, predicate) => {
        const markerIndex = text.indexOf(marker)
        if (markerIndex < 0) {
          return null
        }

        const starts = []
        let cursor = markerIndex
        while (cursor >= 0 && starts.length < 64) {
          const start = text.lastIndexOf('{', cursor)
          if (start < 0) break
          starts.push(start)
          cursor = start - 1
        }

        for (const start of starts) {
          const candidate = extractBalanced(text, start)
          if (!candidate) continue
          try {
            const parsed = JSON.parse(candidate)
            if (!predicate || predicate(parsed)) {
              return parsed
            }
          } catch {
            // continue
          }
        }

        return null
      }

      const extractItemList = text =>
        extractObjectAroundMarker(text, '"@type":"ItemList"', value => Array.isArray(value?.itemListElement))

      const extractFlightHtml = (text, refToken) => {
        if (typeof refToken !== 'string' || !refToken.startsWith('$')) {
          return null
        }
        const refId = refToken.slice(1)
        const lineIndex = text.indexOf(`${refId}:T`)
        if (lineIndex < 0) {
          return null
        }
        const commaIndex = text.indexOf(',', lineIndex)
        if (commaIndex < 0) {
          return null
        }
        const bodyStart = commaIndex + 1
        const remainder = text.slice(bodyStart)
        const nextEntryOffset = remainder.search(/\n[0-9a-z]+:/i)
        return (nextEntryOffset >= 0 ? remainder.slice(0, nextEntryOffset) : remainder).trim()
      }

      const parseArgs = value =>
        String(value || '')
          .match(/"[^"]*"|'[^']*'|\S+/g)
          ?.map(part => part.replace(/^['"]|['"]$/g, '')) || []

      const buildServerMarkdown = server => {
        const sections = []
        const description = String(server.longDescription || server.description || server.shortDescription || '').trim()
        const features = Array.isArray(server.features) ? server.features.filter(Boolean) : []
        const useCases = Array.isArray(server.useCases) ? server.useCases.filter(Boolean) : []
        const faq = Array.isArray(server?.seo?.faq) ? server.seo.faq.filter(Boolean) : []

        if (description) {
          sections.push(description)
        }

        if (features.length > 0) {
          sections.push(`## 主要功能\n${features.map((item, index) => `${index + 1}. ${item}`).join('\n')}`)
        }

        if (useCases.length > 0) {
          sections.push(`## 使用场景\n${useCases.map((item, index) => `${index + 1}. ${item}`).join('\n')}`)
        }

        if (faq.length > 0) {
          sections.push(
            `## 常见问题\n${faq
              .map(item => {
                const question = String(item.question || '').trim()
                const answer = String(item.answer || '').trim()
                return question && answer ? `### ${question}\n${answer}` : ''
              })
              .filter(Boolean)
              .join('\n\n')}`
          )
        }

        return sections.filter(Boolean).join('\n\n').trim()
      }

      const normalizeGithubUrl = value => {
        const input = String(value || '').trim()
        if (!input) return null
        if (input.startsWith('https://github.com/')) return input
        const parts = input.split('/').filter(Boolean)
        return parts.length >= 2 ? `https://github.com/${parts[0]}/${parts[1]}` : null
      }

      const extractCardMetadata = (text, pathPrefix) => {
        const chunks = text.split(`"href":"${pathPrefix}/`)
        const metadata = new Map()
        for (let index = 1; index < chunks.length; index += 1) {
          const segment = chunks[index]
          const slug = decodeJsonString(segment.slice(0, segment.indexOf('"')))
          if (!slug || metadata.has(slug)) continue

          const snippet = segment.slice(0, 2400)
          const author = decodeJsonString(snippet.match(/"alt":"((?:\\.|[^"])*)"/)?.[1] || '')
          const name = decodeJsonString(
            snippet.match(/"h3",null,\{"children":"((?:\\.|[^"])*)"/)?.[1] ||
              snippet.match(/"heading[^"]*","children":"((?:\\.|[^"])*)"/)?.[1] ||
              ''
          )
          const description = decodeJsonString(snippet.match(/"p",null,\{"children":"((?:\\.|[^"])*)"/)?.[1] || '')
          const textChildren = Array.from(snippet.matchAll(/"children":"((?:\\.|[^"])*)"/g))
            .map(match => decodeJsonString(match[1]))
            .filter(Boolean)
          const stars = parseNumber(textChildren.find(value => /^\d[\d,]*$/.test(value)) || '')
          const firstNumericIndex = textChildren.findIndex(value => /^\d[\d,]*$/.test(value))
          const candidatePool = (firstNumericIndex >= 0 ? textChildren.slice(0, firstNumericIndex) : textChildren).filter(
            value => value && value !== name && value !== description && value !== author
          )
          const category = candidatePool.length > 0 ? candidatePool[candidatePool.length - 1] : ''
          const repositoryUrl = snippet.match(/"href":"(https:\/\/github\.com\/[^"]+)"/)?.[1] || null
          metadata.set(slug, { author, name, description, category, stars, repositoryUrl })
        }
        return metadata
      }

      const parseInstallCommand = (rawHtml, plainText) => {
        const html = String(rawHtml || '')
        const text = String(plainText || '')
        const commandLine =
          decodeHtmlEntities(
            html.match(/data-snippet-clipboard-copy-content="claude mcp add [^"]* -- ([^"]+)"/i)?.[1] || ''
          ) ||
          text.match(/claude mcp add [^\n]+ -- ([^\n]+)/i)?.[1] ||
          ''

        if (commandLine) {
          const tokens = parseArgs(commandLine)
          if (tokens.length > 0) {
            return { installCommand: tokens[0], installArgs: tokens.slice(1) }
          }
        }

        const commandMatch = html.match(
          /&quot;command&quot;:\s*&quot;([^"]+)&quot;[\s\S]*?&quot;args&quot;:\s*\[([\s\S]*?)\]/i
        )
        if (commandMatch) {
          const args = Array.from(commandMatch[2].matchAll(/&quot;([^"]+)&quot;/g)).map(match =>
            decodeHtmlEntities(match[1])
          )
          return {
            installCommand: decodeHtmlEntities(commandMatch[1]),
            installArgs: args,
          }
        }

        return { installCommand: null, installArgs: [] }
      }

      const rscText = await fetchRsc()

      if (activeMode === 'mcp-list') {
        const itemList = extractItemList(rscText)
        if (!itemList) throw new Error('Failed to parse MCP Market server list')
        const cardMetadata = extractCardMetadata(rscText, '/zh/server')
        return itemList.itemListElement
          .map(entry => entry?.item)
          .filter(Boolean)
          .map(item => {
            const slug = String(item.url || '').split('/').filter(Boolean).pop() || ''
            const card = cardMetadata.get(slug)
            const stars = parseNumber(item?.interactionStatistic?.userInteractionCount) ?? card?.stars ?? null
            return {
              id: slug,
              slug,
              name: item.name || card?.name || slug,
              description: item.description || card?.description || '',
              author: item?.author?.name || card?.author || '',
              category: card?.category || '',
              tags: card?.category ? [card.category] : [],
              transport_type: 'stdio',
              install_command: null,
              install_args: [],
              env_template: {},
              logo: null,
              downloads: stars,
              rating: null,
              repository_url: card?.repositoryUrl || null,
              source_market: 'mcpmarket',
              stars,
            }
          })
          .filter(item => item.slug && item.name)
      }

      if (activeMode === 'mcp-detail') {
        const slug = route.path.split('/').filter(Boolean).pop().split('?')[0]
        const server = extractObjectAroundMarker(
          rscText,
          '"mcpTools":',
          value => value?.type === 'server' && Array.isArray(value?.mcpTools)
        )
        if (!server) throw new Error('Failed to parse MCP Market server detail')

        const readmeHtml = extractFlightHtml(rscText, server.readmeHtml) || ''
        const detailPanelText = document.querySelector('[role="tabpanel"]')?.innerText?.trim() || ''
        const readmeText = detailPanelText || buildServerMarkdown(server) || stripHtml(readmeHtml)
        const install = parseInstallCommand(readmeHtml, readmeText)
        const category = server.category || server.categories?.[0] || ''
        const tags = Array.isArray(server.categories)
          ? server.categories.filter(value => typeof value === 'string' && value)
          : category
            ? [category]
            : []
        const repositoryUrl = normalizeGithubUrl(server.github)
        const stars = parseNumber(server.github_stars)
        const author = server.owner?.name || server.author || ''
        const description = server.shortDescription || server.description || ''

        return {
          id: slug,
          slug,
          name: server.name || slug,
          description,
          author,
          category,
          tags,
          transport_type: 'stdio',
          install_command: install.installCommand,
          install_args: install.installArgs,
          env_template: {},
          logo: server.owner?.avatar || null,
          downloads: stars,
          rating: null,
          repository_url: repositoryUrl,
          source_market: 'mcpmarket',
          stars,
          full_description: readmeText || server.longDescription || description,
          readme_excerpt: (readmeText || server.readmeSummary || description || '').slice(0, 4000) || null,
        }
      }

      throw new Error(`Unsupported mode: ${activeMode}`)
    },
    { mode: currentMode, currentRoute }
  )
}

async function main() {
  const executablePath = resolveBrowserExecutable()
  if (!executablePath) {
    throw new Error('No supported Chromium-based browser found for MCP Market fetcher')
  }

  const browser = await chromium.launch({
    executablePath,
    headless: process.env.MCPMARKET_BROWSER_VISIBLE === '1' ? false : true,
  })

  try {
    const context = await browser.newContext({
      locale: 'zh-CN',
      userAgent:
        'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36',
      viewport: { width: 1440, height: 960 },
    })

    const route = routeForMode(mode, payload)
    const origin = resolveOrigin(route)

    if (mode === 'skill-archive') {
      const result = await downloadSkillArchive(context, payload.slug)
      console.log(JSON.stringify(result))
      await context.close()
      return
    }

    const page = await context.newPage()
    await page.goto(`${origin}${route.path}`, { waitUntil: 'domcontentloaded', timeout: 45000 })

    let result
    if (mode === 'skill-list') {
      await waitForStableSkillsPage(page, 'main a[href*="/tools/skills/"] h3')
      result = await parseSkillsList(page, context, payload)
    } else if (mode === 'mcp-list') {
      await waitForStableSkillsPage(page, 'main a[href*="/server/"] h3')
      result = await parseMcpList(page, context, payload)
    } else if (mode === 'skill-detail') {
      await waitForStableSkillsPage(page, 'h1')
      result = await parseSkillDetail(page)
    } else if (mode === 'modelscope-mcp-list') {
      await waitForStableModelScopePage(page, 'main a[href*="/mcp/servers/"]')
      result = await parseModelScopeMcpList(page, payload)
    } else if (mode === 'modelscope-mcp-detail') {
      await waitForStableModelScopePage(page, 'main')
      result = await parseModelScopeMcpDetail(page)
    } else {
      await page.waitForTimeout(1500)
      result = await parseMcpWithRsc(page, mode, route)
    }

    console.log(JSON.stringify(result))
    await context.close()
  } finally {
    await browser.close()
  }
}

main().catch(error => {
  console.error(error instanceof Error ? error.message : String(error))
  process.exit(1)
})
