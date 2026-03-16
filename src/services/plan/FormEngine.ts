/**
 * 动态表单引擎
 * 负责 Schema 解析、表单渲染、验证逻辑
 */

import type {
  DynamicFormSchema,
  FormField,
  FormTemplate,
  FormRequest,
  FormResponse
} from '@/types/plan'

// 预定义模板库
export const FORM_TEMPLATES: FormTemplate[] = [
  {
    id: 'feature-request',
    name: '功能需求收集',
    description: '收集新功能的需求信息',
    category: 'requirement',
    schema: {
      formId: 'feature-request',
      title: '功能需求收集',
      description: '请填写以下信息以帮助我们更好地理解您的需求',
      fields: [
        {
          name: 'featureName',
          label: '功能名称',
          type: 'text',
          required: true,
          placeholder: '请输入功能名称'
        },
        {
          name: 'description',
          label: '功能描述',
          type: 'textarea',
          required: true,
          placeholder: '请详细描述功能需求'
        },
        {
          name: 'priority',
          label: '优先级',
          type: 'select',
          options: [
            { label: '低', value: 'low' },
            { label: '中', value: 'medium' },
            { label: '高', value: 'high' }
          ],
          default: 'medium'
        },
        {
          name: 'deadline',
          label: '期望完成日期',
          type: 'date'
        },
        {
          name: 'attachments',
          label: '相关附件',
          type: 'file'
        }
      ],
      submitText: '提交需求'
    }
  },
  {
    id: 'bug-report',
    name: '问题报告',
    description: '报告问题或 Bug',
    category: 'requirement',
    schema: {
      formId: 'bug-report',
      title: '问题报告',
      description: '请描述您遇到的问题',
      fields: [
        {
          name: 'title',
          label: '问题标题',
          type: 'text',
          required: true,
          placeholder: '简短描述问题'
        },
        {
          name: 'description',
          label: '问题描述',
          type: 'textarea',
          required: true,
          placeholder: '请详细描述问题，包括复现步骤'
        },
        {
          name: 'severity',
          label: '严重程度',
          type: 'select',
          options: [
            { label: '低', value: 'low' },
            { label: '中', value: 'medium' },
            { label: '高', value: 'high' },
            { label: '紧急', value: 'critical' }
          ],
          default: 'medium'
        },
        {
          name: 'environment',
          label: '环境信息',
          type: 'textarea',
          placeholder: '操作系统、浏览器版本等'
        }
      ],
      submitText: '提交报告'
    }
  },
  {
    id: 'database-config',
    name: '数据库配置',
    description: '配置数据库连接信息',
    category: 'config',
    schema: {
      formId: 'database-config',
      title: '数据库配置',
      fields: [
        {
          name: 'dbType',
          label: '数据库类型',
          type: 'select',
          required: true,
          options: [
            { label: 'MySQL', value: 'mysql' },
            { label: 'PostgreSQL', value: 'postgresql' },
            { label: 'SQLite', value: 'sqlite' },
            { label: 'MongoDB', value: 'mongodb' }
          ]
        },
        {
          name: 'host',
          label: '主机地址',
          type: 'text',
          required: true,
          placeholder: 'localhost'
        },
        {
          name: 'port',
          label: '端口',
          type: 'number',
          default: 3306
        },
        {
          name: 'database',
          label: '数据库名',
          type: 'text',
          required: true
        },
        {
          name: 'username',
          label: '用户名',
          type: 'text',
          required: true
        },
        {
          name: 'password',
          label: '密码',
          type: 'text',
          required: true
        }
      ],
      submitText: '保存配置'
    }
  },
  {
    id: 'api-config',
    name: 'API 配置',
    description: '配置 API 连接信息',
    category: 'config',
    schema: {
      formId: 'api-config',
      title: 'API 配置',
      fields: [
        {
          name: 'apiName',
          label: 'API 名称',
          type: 'text',
          required: true
        },
        {
          name: 'baseUrl',
          label: 'Base URL',
          type: 'text',
          required: true,
          placeholder: 'https://api.example.com'
        },
        {
          name: 'authType',
          label: '认证方式',
          type: 'select',
          options: [
            { label: '无', value: 'none' },
            { label: 'API Key', value: 'api_key' },
            { label: 'Bearer Token', value: 'bearer' },
            { label: 'Basic Auth', value: 'basic' }
          ],
          default: 'none'
        },
        {
          name: 'apiKey',
          label: 'API Key',
          type: 'text',
          condition: { field: 'authType', value: 'api_key' }
        },
        {
          name: 'token',
          label: 'Token',
          type: 'text',
          condition: { field: 'authType', value: 'bearer' }
        }
      ],
      submitText: '保存配置'
    }
  },
  {
    id: 'code-review',
    name: '代码审查',
    description: '代码审查检查清单',
    category: 'review',
    schema: {
      formId: 'code-review',
      title: '代码审查',
      fields: [
        {
          name: 'reviewScope',
          label: '审查范围',
          type: 'multiselect',
          required: true,
          options: [
            { label: '代码逻辑', value: 'logic' },
            { label: '性能优化', value: 'performance' },
            { label: '安全检查', value: 'security' },
            { label: '代码风格', value: 'style' },
            { label: '测试覆盖', value: 'testing' }
          ]
        },
        {
          name: 'focusAreas',
          label: '重点关注',
          type: 'textarea',
          placeholder: '请描述需要特别关注的方面'
        },
        {
          name: 'strictness',
          label: '审查严格程度',
          type: 'slider',
          default: 50,
          validation: { min: 0, max: 100 }
        }
      ],
      submitText: '开始审查'
    }
  },
  {
    id: 'deploy-config',
    name: '部署配置',
    description: '配置部署参数',
    category: 'deploy',
    schema: {
      formId: 'deploy-config',
      title: '部署配置',
      fields: [
        {
          name: 'environment',
          label: '部署环境',
          type: 'radio',
          required: true,
          options: [
            { label: '开发环境', value: 'development' },
            { label: '测试环境', value: 'staging' },
            { label: '生产环境', value: 'production' }
          ],
          default: 'development'
        },
        {
          name: 'deployType',
          label: '部署方式',
          type: 'select',
          required: true,
          options: [
            { label: 'Docker', value: 'docker' },
            { label: 'Kubernetes', value: 'k8s' },
            { label: '传统部署', value: 'traditional' }
          ]
        },
        {
          name: 'replicas',
          label: '副本数量',
          type: 'number',
          default: 1,
          validation: { min: 1, max: 10 }
        },
        {
          name: 'enableMonitoring',
          label: '启用监控',
          type: 'checkbox',
          default: true
        }
      ],
      submitText: '开始部署'
    }
  }
]

/**
 * 表单引擎类
 */
export class FormEngine {
  private templates: Map<string, FormTemplate>

  constructor() {
    this.templates = new Map()
    // 加载预定义模板
    FORM_TEMPLATES.forEach(template => {
      this.templates.set(template.id, template)
    })
  }

  /**
   * 解析 AI 输出的表单请求
   */
  parseFormRequest(request: FormRequest): DynamicFormSchema | null {
    if (request.mode === 'schema' && request.schema) {
      // 直接使用 Schema
      return this.validateSchema(request.schema)
    }

    if (request.mode === 'template' && request.templateId) {
      // 加载模板并合并默认值
      const template = this.templates.get(request.templateId)
      if (!template) {
        console.error(`Template not found: ${request.templateId}`)
        return null
      }

      // 如果有默认值，合并到 schema
      if (request.defaultValues) {
        return this.mergeDefaultValues(template.schema, request.defaultValues)
      }

      return template.schema
    }

    return null
  }

  /**
   * 从 AI 消息中提取表单请求
   */
  extractFormRequest(message: string): FormRequest | null {
    try {
      // 尝试解析 JSON 格式的表单请求
      const jsonMatch = message.match(/```json\n([\s\S]*?)\n```/)
      if (jsonMatch) {
        const parsed = JSON.parse(jsonMatch[1])
        if (parsed.type === 'form_request') {
          return parsed as FormRequest
        }
      }

      // 尝试直接解析整个消息
      const parsed = JSON.parse(message)
      if (parsed.type === 'form_request') {
        return parsed as FormRequest
      }
    } catch {
      // 不是 JSON 格式，尝试其他格式
    }

    // 尝试解析 XML 格式
    const xmlMatch = message.match(/<form_request[^>]*>([\s\S]*?)<\/form_request>/)
    if (xmlMatch) {
      return this.parseXmlFormRequest(xmlMatch[1])
    }

    return null
  }

  /**
   * 验证 Schema 有效性
   */
  validateSchema(schema: DynamicFormSchema): DynamicFormSchema | null {
    if (!schema.formId || !schema.title || !Array.isArray(schema.fields)) {
      return null
    }

    // 验证每个字段
    for (const field of schema.fields as FormField[]) {
      if (!field.name || !field.label || !field.type) {
        return null
      }
    }

    return schema
  }

  /**
   * 合并默认值到 Schema
   */
  mergeDefaultValues(
    schema: DynamicFormSchema,
    defaultValues: Record<string, any>
  ): DynamicFormSchema {
    const merged = JSON.parse(JSON.stringify(schema))

    merged.fields.forEach((field: FormField) => {
      if (defaultValues[field.name] !== undefined) {
        field.default = defaultValues[field.name]
      }
    })

    return merged
  }

  /**
   * 获取所有模板
   */
  getTemplates(): FormTemplate[] {
    return Array.from(this.templates.values())
  }

  /**
   * 按分类获取模板
   */
  getTemplatesByCategory(category: FormTemplate['category']): FormTemplate[] {
    return this.getTemplates().filter(t => t.category === category)
  }

  /**
   * 获取单个模板
   */
  getTemplate(id: string): FormTemplate | undefined {
    return this.templates.get(id)
  }

  /**
   * 注册自定义模板
   */
  registerTemplate(template: FormTemplate): void {
    this.templates.set(template.id, template)
  }

  /**
   * 验证表单数据
   */
  validateFormData(schema: DynamicFormSchema, values: Record<string, any>): {
    valid: boolean
    errors: Record<string, string>
  } {
    const errors: Record<string, string> = {}

    schema.fields.forEach(field => {
      // 条件字段隐藏时不参与校验，否则表单在动态切换字段时会被“看不见的必填项”卡住。
      if (field.condition && values[field.condition.field] !== field.condition.value) {
        return
      }

      const value = values[field.name]
      const isEmptyArray = Array.isArray(value) && value.length === 0

      // 必填验证
      if (field.required && (value === undefined || value === null || value === '' || isEmptyArray)) {
        errors[field.name] = `${field.label} 是必填项`
        return
      }

      // 如果值为空且非必填，跳过其他验证
      if (value === undefined || value === null || value === '') {
        return
      }

      // 类型特定验证
      if (field.validation) {
        // 数字范围验证
        if (field.type === 'number' || field.type === 'slider') {
          const numValue = Number(value)
          if (field.validation.min !== undefined && numValue < field.validation.min) {
            errors[field.name] = field.validation.message || `${field.label} 不能小于 ${field.validation.min}`
          }
          if (field.validation.max !== undefined && numValue > field.validation.max) {
            errors[field.name] = field.validation.message || `${field.label} 不能大于 ${field.validation.max}`
          }
        }

        // 文本长度验证
        if (field.type === 'text' || field.type === 'textarea') {
          const strValue = String(value)
          if (field.validation.min !== undefined && strValue.length < field.validation.min) {
            errors[field.name] = field.validation.message || `${field.label} 长度不能少于 ${field.validation.min} 个字符`
          }
          if (field.validation.max !== undefined && strValue.length > field.validation.max) {
            errors[field.name] = field.validation.message || `${field.label} 长度不能超过 ${field.validation.max} 个字符`
          }
        }

        // 正则表达式验证
        if (field.validation.pattern) {
          const regex = new RegExp(field.validation.pattern)
          if (!regex.test(String(value))) {
            errors[field.name] = field.validation.message || `${field.label} 格式不正确`
          }
        }
      }
    })

    return {
      valid: Object.keys(errors).length === 0,
      errors
    }
  }

  /**
   * 创建表单响应
   */
  createFormResponse(formId: string, values: Record<string, any>): FormResponse {
    return {
      type: 'form_response',
      formId,
      values
    }
  }

  /**
   * 解析 XML 格式的表单请求
   */
  private parseXmlFormRequest(content: string): FormRequest | null {
    // 简化的 XML 解析
    const modeMatch = content.match(/mode="([^"]*)"/)
    const templateIdMatch = content.match(/template_id="([^"]*)"/)

    if (modeMatch) {
      const request: FormRequest = {
        type: 'form_request',
        mode: modeMatch[1] as 'schema' | 'template'
      }

      if (templateIdMatch) {
        request.templateId = templateIdMatch[1]
      }

      return request
    }

    return null
  }
}

// 导出单例实例
export const formEngine = new FormEngine()
