import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useNotificationStore } from '@/stores/notification'
import type { CliType } from '@/stores/providerProfile'
import { getErrorMessage } from '@/utils/api'

export interface DefaultCliConfigEditorFile {
  cliType: CliType
  path: string
  content: string
  fileType: 'json' | 'toml'
}

export interface DefaultCliConfigLocateTarget {
  query: string
  label?: string
  matchCase?: boolean
}

interface UseDefaultCliConfigEditorOptions {
  onAfterSave?: (cliType: CliType) => Promise<void> | void
}

/**
 * 统一封装默认 CLI 配置文件的读取、格式化、保存和定位状态。
 * 适用于设置页和 Skill 配置页，避免重复维护同一套编辑器状态机。
 */
export function useDefaultCliConfigEditor(options: UseDefaultCliConfigEditorOptions = {}) {
  const { t } = useI18n()
  const notificationStore = useNotificationStore()

  const showConfigEditor = ref(false)
  const isConfigEditorLoading = ref(false)
  const isConfigEditorSaving = ref(false)
  const configEditorFile = ref<DefaultCliConfigEditorFile | null>(null)
  const configEditorContent = ref('')
  const configEditorLocateTarget = ref<DefaultCliConfigLocateTarget | null>(null)

  const isConfigEditorDirty = computed(() => (
    configEditorFile.value !== null && configEditorContent.value !== configEditorFile.value.content
  ))

  async function readDefaultCliEditorFile(cliType: CliType) {
    return invoke<DefaultCliConfigEditorFile>('read_default_cli_config_file', { cliType })
  }

  /**
   * 载入默认配置文件，并可选择记录本次打开需要定位的目标内容。
   */
  async function loadDefaultConfigFile(cliType: CliType, locateTarget?: DefaultCliConfigLocateTarget | null) {
    isConfigEditorLoading.value = true

    try {
      const file = await readDefaultCliEditorFile(cliType)
      configEditorFile.value = file
      configEditorContent.value = file.content
      configEditorLocateTarget.value = locateTarget?.query.trim() ? locateTarget : null
    } catch (error) {
      console.error('Failed to load default config file:', error)
      notificationStore.error(t('common.error'), getErrorMessage(error))
      throw error
    } finally {
      isConfigEditorLoading.value = false
    }
  }

  /**
   * 打开默认配置文件编辑器，并在加载完成后将目标内容定位到首个匹配项。
   */
  async function openConfigEditor(cliType: CliType, locateTarget?: DefaultCliConfigLocateTarget | null) {
    showConfigEditor.value = true
    await loadDefaultConfigFile(cliType, locateTarget)
  }

  async function reloadConfigEditor(cliType: CliType) {
    await loadDefaultConfigFile(cliType, configEditorLocateTarget.value)
  }

  async function formatConfigEditor() {
    if (!configEditorFile.value) {
      return
    }

    try {
      configEditorContent.value = await invoke<string>('format_default_cli_config_content', {
        cliType: configEditorFile.value.cliType,
        content: configEditorContent.value
      })
      notificationStore.success(t('settings.providerSwitch.messages.formatSuccess'))
    } catch (error) {
      console.error('Failed to format config file:', error)
      notificationStore.error(t('common.error'), getErrorMessage(error))
    }
  }

  async function saveConfigEditor() {
    if (!configEditorFile.value) {
      return
    }

    isConfigEditorSaving.value = true
    try {
      const file = await invoke<DefaultCliConfigEditorFile>('write_default_cli_config_file', {
        cliType: configEditorFile.value.cliType,
        content: configEditorContent.value
      })
      configEditorFile.value = file
      configEditorContent.value = file.content
      await options.onAfterSave?.(file.cliType)
      notificationStore.success(t('settings.providerSwitch.messages.saveConfigSuccess'))
    } catch (error) {
      console.error('Failed to save config file:', error)
      notificationStore.error(t('common.error'), getErrorMessage(error))
    } finally {
      isConfigEditorSaving.value = false
    }
  }

  function resetConfigEditor() {
    showConfigEditor.value = false
    isConfigEditorLoading.value = false
    isConfigEditorSaving.value = false
    configEditorFile.value = null
    configEditorContent.value = ''
    configEditorLocateTarget.value = null
  }

  return {
    configEditorContent,
    configEditorFile,
    configEditorLocateTarget,
    formatConfigEditor,
    isConfigEditorDirty,
    isConfigEditorLoading,
    isConfigEditorSaving,
    loadDefaultConfigFile,
    openConfigEditor,
    reloadConfigEditor,
    resetConfigEditor,
    saveConfigEditor,
    showConfigEditor
  }
}
