import { computed, ref, watch } from 'vue'
import type { SoloAgentOption, SoloCreateFormState, SoloModelOption, SoloRunFormMode } from '../soloShared'

export interface SoloRunCreateDialogProps {
  visible: boolean
  mode: SoloRunFormMode
  form: SoloCreateFormState
  coordinatorOptions: SoloAgentOption[]
  expertOptions: SoloAgentOption[]
  modelOptions: SoloModelOption[]
  canCreate: boolean
}

export type SoloRunCreateDialogEmits = {
  (e: 'close'): void
  (e: 'browseExecutionPath'): void
  (e: 'createDraft'): void
  (e: 'createAndStart'): void
  (e: 'save'): void
  (e: 'update:form', patch: Partial<SoloCreateFormState>): void
}

/**
 * 管理 SOLO 创建弹窗的专家勾选、字段回写和折叠展示状态。
 */
export function useSoloRunCreateDialog(
  props: Readonly<SoloRunCreateDialogProps>,
  emit: SoloRunCreateDialogEmits
) {
  const showAllExperts = ref(false)
  const collapsedExpertLimit = 6

  const allSelected = computed(() =>
    props.expertOptions.length > 0
    && props.form.participantExpertIds.length === props.expertOptions.length
  )

  const hiddenExpertCount = computed(() =>
    Math.max(props.expertOptions.length - collapsedExpertLimit, 0)
  )

  const visibleExpertOptions = computed(() => (
    showAllExperts.value
      ? props.expertOptions
      : props.expertOptions.slice(0, collapsedExpertLimit)
  ))

  function updateField<K extends keyof SoloCreateFormState>(key: K, value: SoloCreateFormState[K]) {
    emit('update:form', { [key]: value })
  }

  function toggleExpert(expertId: string) {
    const nextIds = props.form.participantExpertIds.includes(expertId)
      ? props.form.participantExpertIds.filter((id) => id !== expertId)
      : [...props.form.participantExpertIds, expertId]

    updateField('participantExpertIds', nextIds)
  }

  function toggleAllExperts() {
    updateField(
      'participantExpertIds',
      allSelected.value ? [] : props.expertOptions.map((option) => option.value)
    )
  }

  function toggleExpertOverflow() {
    showAllExperts.value = !showAllExperts.value
  }

  watch(
    () => props.visible,
    (visible) => {
      if (!visible) {
        showAllExperts.value = false
      }
    }
  )

  return {
    allSelected,
    hiddenExpertCount,
    showAllExperts,
    toggleAllExperts,
    toggleExpert,
    toggleExpertOverflow,
    updateField,
    visibleExpertOptions,
  }
}
