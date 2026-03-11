import { ref, readonly, type Ref } from 'vue'
import type { ConfirmDialogType } from '@/components/common/EaConfirmDialog.vue'

export interface ConfirmDialogOptions {
  type?: ConfirmDialogType
  title?: string
  message: string
  confirmLabel?: string
  cancelLabel?: string
  confirmButtonType?: 'primary' | 'danger'
}

interface ConfirmDialogState extends ConfirmDialogOptions {
  visible: boolean
  resolve: ((value: boolean) => void) | null
}

declare global {
  var __easyAgentConfirmDialogState__: Ref<ConfirmDialogState> | undefined
}

const defaultState = (): ConfirmDialogState => ({
  visible: false,
  type: 'warning',
  title: '',
  message: '',
  confirmLabel: '',
  cancelLabel: '',
  confirmButtonType: 'danger',
  resolve: null
})

const state = globalThis.__easyAgentConfirmDialogState__
  ?? (globalThis.__easyAgentConfirmDialogState__ = ref<ConfirmDialogState>(defaultState()))

function resetState() {
  state.value = defaultState()
}

/**
 * useConfirmDialog - 确认对话框 composable
 * 提供编程式 API 来显示确认对话框
 */
export function useConfirmDialog() {
  /**
   * 显示确认对话框
   * @param options 对话框配置选项
   * @returns Promise<boolean> - 用户确认返回 true，取消返回 false
   */
  function show(options: ConfirmDialogOptions): Promise<boolean> {
    return new Promise((resolve) => {
      state.value = {
        visible: true,
        type: options.type || 'warning',
        title: options.title || '',
        message: options.message,
        confirmLabel: options.confirmLabel || '',
        cancelLabel: options.cancelLabel || '',
        confirmButtonType: options.confirmButtonType || 'danger',
        resolve
      }
    })
  }

  /**
   * 显示警告确认对话框（黄色警告）
   */
  function warning(message: string, title?: string): Promise<boolean> {
    return show({
      type: 'warning',
      message,
      title
    })
  }

  /**
   * 显示危险操作确认对话框（红色警告）
   */
  function danger(message: string, title?: string): Promise<boolean> {
    return show({
      type: 'danger',
      message,
      title,
      confirmButtonType: 'danger'
    })
  }

  /**
   * 显示信息确认对话框（蓝色提示）
   */
  function info(message: string, title?: string): Promise<boolean> {
    return show({
      type: 'info',
      message,
      title,
      confirmButtonType: 'primary'
    })
  }

  /**
   * 处理用户确认
   */
  function handleConfirm() {
    const resolve = state.value.resolve
    resetState()
    queueMicrotask(() => resolve?.(true))
  }

  /**
   * 处理用户取消
   */
  function handleCancel() {
    const resolve = state.value.resolve
    resetState()
    queueMicrotask(() => resolve?.(false))
  }

  function handleVisibleChange(visible: boolean) {
    if (!visible && state.value.visible) {
      handleCancel()
    }
  }

  return {
    state: readonly(state),
    show,
    warning,
    danger,
    info,
    handleConfirm,
    handleCancel,
    handleVisibleChange
  }
}
