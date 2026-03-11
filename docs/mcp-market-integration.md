# MCP Market 网站技术分析与集成方案

## Context（背景）

本文档是对 https://mcpmarket.com/zh 网站的完整技术分析，旨在了解其MCP服务和Skills技能的页面结构、功能逻辑，以便后续通过调用接口的方式将市场内容集成到当前软件中显示。

---

## 一、网站概述

### 1.1 基本信息
- **网站地址**: https://mcpmarket.com
- **技术栈**: Next.js（服务端渲染SSR）
- **多语言支持**: `/zh` 中文、`/en` 英文
- **数据量**: 约23,062个MCP服务器、59,728个Skills

### 1.2 核心功能模块
1. **MCP服务器市场** - 浏览、搜索、分类筛选
2. **Agent Skills目录** - 技能浏览、安装指南
3. **MCP客户端目录** - 支持MCP的客户端软件
4. **排行榜系统** - 热门、最新、精选

---

## 二、URL路由结构

### 2.1 MCP服务器相关

| 页面 | URL模式 | 说明 |
|------|---------|------|
| 首页 | `/zh` | 展示官方、精选、热门、最新服务器 |
| 服务器列表 | `/zh/server` | 所有服务器分页列表（729页） |
| 服务器详情 | `/zh/server/{slug}` | 单个服务器详情页 |
| 分类列表 | `/zh/categories` | 所有分类 |
| 分类详情 | `/zh/categories/{category-slug}` | 分类下服务器 |
| 搜索 | `/zh/search` | 搜索页面（支持服务器/Skills切换） |
| 排行榜 | `/zh/leaderboards` | Top 100服务器 |
| 今日热门 | `/zh/daily` | 今日热门MCP |

### 2.2 Agent Skills相关

| 页面 | URL模式 | 说明 |
|------|---------|------|
| Skills首页 | `/zh/tools/skills` | Skills目录首页 |
| Skills详情 | `/zh/tools/skills/{skill-slug}` | 单个技能详情 |
| Skills分类 | `/zh/tools/skills/categories` | Skills分类 |
| Skills分类详情 | `/zh/tools/skills/categories/{slug}` | 分类下Skills |
| Skills排行榜 | `/zh/tools/skills/leaderboard` | Top 100 Skills |
| 安装指南 | `/zh/tools/skills/how-to-install` | 安装教程 |
| 什么是Skills | `/zh/tools/skills/what-are-skills` | 介绍页面 |

### 2.3 其他页面

| 页面 | URL模式 | 说明 |
|------|---------|------|
| MCP客户端 | `/zh/client` | 客户端列表 |
| 客户端详情 | `/zh/client/{slug}` | 客户端详情 |
| 提交 | `/zh/submit` | 提交新MCP |
| 新闻 | `/zh/news` | 新闻动态 |

---

## 三、分类体系（20个分类）

| 分类slug | 中文名称 | 服务器数量 |
|----------|----------|------------|
| developer-tools | 开发者工具 | 15,501 |
| api-development | API 开发 | 10,026 |
| data-science-ml | 数据科学与机器学习 | 7,565 |
| productivity-workflow | 生产力与工作流 | 6,515 |
| other | 其他 | 3,731 |
| analytics-monitoring | 分析与监控 | 2,680 |
| deployment-devops | 部署与 DevOps | 2,671 |
| security-testing | 安全与测试 | 2,191 |
| web-scraping-data-collection | 网络抓取与数据收集 | 2,123 |
| learning-documentation | 学习与文档 | 1,858 |
| database-management | 数据库管理 | 1,762 |
| cloud-infrastructure | 云基础设施 | 1,357 |
| collaboration-tools | 协作工具 | 1,172 |
| content-management | 内容管理 | 1,078 |
| design-tools | 设计工具 | 537 |
| browser-automation | 浏览器自动化 | 519 |
| social-media-management | 社交媒体管理 | 421 |
| game-development | 游戏开发 | 387 |
| marketing-automation | 营销自动化 | 385 |
| e-commerce-solutions | 电子商务解决方案 | 347 |
| mobile-development | Mobile Development | - |
| official | Official | 官方 |
| featured | Featured | 精选 |

---

## 四、数据结构分析

### 4.1 MCP服务器卡片数据

```typescript
interface McpServerCard {
  slug: string              // URL标识
  name: string              // 名称
  author: string            // 作者/组织
  description: string       // 描述
  category: string          // 分类
  stars: number             // GitHub stars/下载量
  github_url: string        // GitHub仓库地址
  is_official: boolean      // 是否官方
  is_featured: boolean      // 是否精选
  is_sponsored: boolean     // 是否赞助
}
```

### 4.2 MCP服务器详情页数据

```typescript
interface McpServerDetail extends McpServerCard {
  // 分类（多个）
  categories: string[]

  // 标签页内容
  tabs: {
    about: string       // 详细描述
    readme: string      // README内容（从GitHub获取）
    faq: string         // 常见问题
  }

  // 主要功能列表
  features: string[]

  // 使用案例
  use_cases: UseCase[]

  // 相关Skills
  related_skills: SkillCard[]

  // 安装信息
  installation: {
    skillfish_command: string
  }
}
```

### 4.3 Skill卡片数据

```typescript
interface SkillCard {
  slug: string              // URL标识
  name: string              // 名称
  author: string            // 作者
  description: string       // 描述
  category: string          // 分类
  stars: number             // GitHub stars
  github_url: string        // GitHub仓库地址

  // 安装方式
  install_command: string   // npx skillfish add facebook/react fix
  download_url: string      // /api/skills/download?url=...
}
```

### 4.4 分类数据

```typescript
interface Category {
  slug: string              // URL标识
  name: string              // 显示名称
  icon: string              // 图标URL
  count: number             // 服务器数量
  description: string       // 分类描述
}
```

---

## 五、搜索功能分析

### 5.1 搜索页面
- URL: `/zh/search`
- 支持Tab切换：MCP服务器 / Claude技能
- 实时搜索（输入即搜索）
- 分类筛选器

### 5.2 搜索参数

```
GET /zh/search?q={keyword}&category={slug}&type={servers|skills}
```

---

## 六、安装机制

### 6.1 MCP服务器安装
MCP服务器需要通过MCP客户端配置安装，不同客户端有不同的配置方式。

### 6.2 Skills安装方式

#### 方式一：Claude.ai（网页版）
- 上传ZIP文件
- 开关预置Skills
- 访问合作伙伴目录

#### 方式二：Claude Code（命令行）
- 个人Skills: `~/.claude/skills/`
- 项目Skills: `.claude/skills/`
- 通过插件市场安装

#### 方式三：Skill.Fish CLI
```bash
npx skillfish add {author}/{repo} {skill-name}
```

#### 方式四：直接下载
```
GET /api/skills/download?url={github_skill_url}
```

---

## 七、API接口分析

### 7.1 已发现接口

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/skills/download` | GET | 下载Skill文件 |

### 7.2 无公开REST API
网站使用Next.js SSR渲染，数据直接嵌入HTML，**没有公开的RESTful API**。

---

## 八、集成方案

### 方案一：网页抓取（推荐）

#### 优点
- 获取完整数据
- 不依赖未公开API

#### 实现方式
1. 使用web-reader MCP工具读取页面内容
2. 解析HTML提取结构化数据
3. 本地缓存减少请求

#### 关键页面抓取URL
```
首页数据:     https://mcpmarket.com/zh
服务器列表:   https://mcpmarket.com/zh/server?page=1
服务器详情:   https://mcpmarket.com/zh/server/{slug}
分类列表:     https://mcpmarket.com/zh/categories
分类详情:     https://mcpmarket.com/zh/categories/{slug}
Skills列表:   https://mcpmarket.com/zh/tools/skills
Skill详情:    https://mcpmarket.com/zh/tools/skills/{slug}
搜索:         https://mcpmarket.com/zh/search?q={keyword}
```

### 方案二：使用web-reader工具

利用 `mcp__web_reader__webReader` 工具读取页面Markdown内容：

```typescript
// 读取服务器详情页示例
const content = await webReader({
  url: 'https://mcpmarket.com/zh/server/superpowers',
  return_format: 'markdown'
});

// 解析返回的Markdown内容提取数据
```

### 方案三：HTML解析服务

创建专门的数据获取服务：

```typescript
// src/services/mcpMarketService.ts

export class McpMarketService {
  private baseUrl = 'https://mcpmarket.com/zh'

  /**
   * 获取首页推荐数据
   */
  async getHomeData(): Promise<HomeData> {
    const content = await this.fetchPage('/')
    return this.parseHomeData(content)
  }

  /**
   * 获取服务器列表
   */
  async getServers(page: number = 1): Promise<PaginatedResult<McpServerCard>> {
    const content = await this.fetchPage(`/server?page=${page}`)
    return this.parseServerList(content)
  }

  /**
   * 获取服务器详情
   */
  async getServerDetail(slug: string): Promise<McpServerDetail> {
    const content = await this.fetchPage(`/server/${slug}`)
    return this.parseServerDetail(content)
  }

  /**
   * 搜索服务器
   */
  async searchServers(query: string, category?: string): Promise<McpServerCard[]> {
    const params = new URLSearchParams({ q: query })
    if (category) params.append('category', category)
    const content = await this.fetchPage(`/search?${params}`)
    return this.parseSearchResults(content)
  }

  /**
   * 获取所有分类
   */
  async getCategories(): Promise<Category[]> {
    const content = await this.fetchPage('/categories')
    return this.parseCategories(content)
  }

  /**
   * 获取Skills列表
   */
  async getSkills(page: number = 1): Promise<PaginatedResult<SkillCard>> {
    const content = await this.fetchPage(`/tools/skills?page=${page}`)
    return this.parseSkillList(content)
  }

  /**
   * 获取Skill详情
   */
  async getSkillDetail(slug: string): Promise<SkillDetail> {
    const content = await this.fetchPage(`/tools/skills/${slug}`)
    return this.parseSkillDetail(content)
  }

  private async fetchPage(path: string): Promise<string> {
    // 使用web-reader或fetch获取页面内容
    // ...
  }

  // 解析方法实现...
}
```

---

## 九、类型定义扩展

在现有 `src/types/marketplace.ts` 基础上，添加MCP Market网站特定类型：

```typescript
// MCP Market 服务器卡片
export interface McpMarketServerCard {
  slug: string
  name: string
  author: string
  description: string
  category: string
  stars: number
  github_url: string
  is_official: boolean
  is_featured: boolean
  is_sponsored: boolean
}

// MCP Market 服务器详情
export interface McpMarketServerDetail extends McpMarketServerCard {
  categories: string[]
  tabs: {
    about: string
    readme: string
    faq: string
  }
  features: string[]
  use_cases: UseCase[]
  related_skills: McpMarketSkillCard[]
  installation: {
    skillfish_command: string
  }
}

// MCP Market Skill卡片
export interface McpMarketSkillCard {
  slug: string
  name: string
  author: string
  description: string
  category: string
  stars: number
  github_url: string
  install_command: string
  download_url: string
}

// MCP Market 分类
export interface McpMarketCategory {
  slug: string
  name: string
  icon: string
  count: number
  description: string
}

// 使用案例
export interface UseCase {
  title: string
  description: string
}
```

---

## 十、注意事项

1. **数据缓存**: 建议实现本地缓存机制，避免频繁请求
2. **错误处理**: 网站可能更新结构，需要健壮的错误处理
3. **速率限制**: 尊重网站资源，控制请求频率
4. **数据时效性**: 定期更新缓存数据
5. **版权声明**: 集成时注明数据来源

---

## 十一、后续工作

1. [ ] 实现数据获取服务 `McpMarketService`
2. [ ] 创建数据解析工具函数
3. [ ] 实现本地缓存机制
4. [ ] 设计UI组件展示市场数据
5. [ ] 实现搜索和筛选功能
6. [ ] 添加安装集成功能

---

## 参考链接

- MCP Market 网站: https://mcpmarket.com
- MCP Market 中文版: https://mcpmarket.com/zh
- Skill.Fish CLI: https://skill.fish
