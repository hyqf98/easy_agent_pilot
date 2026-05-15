import { computed, ref, watch } from 'vue'
import type { SoloAgentOption, SoloCreateFormState, SoloRunFormMode } from '../soloShared'

export interface SoloRunCreateDialogProps {
  visible: boolean
  mode: SoloRunFormMode
  form: SoloCreateFormState
  coordinatorOptions: SoloAgentOption[]
  expertOptions: SoloAgentOption[]
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

  const orderedExpertOptions = computed(() => {
    const selectedIds = new Set(props.form.participantExpertIds)
    return [...props.expertOptions].sort((left, right) => {
      const leftSelected = selectedIds.has(left.value)
      const rightSelected = selectedIds.has(right.value)

      if (leftSelected === rightSelected) {
        return 0
      }

      return leftSelected ? -1 : 1
    })
  })

  const visibleExpertOptions = computed(() => (
    showAllExperts.value
      ? orderedExpertOptions.value
      : orderedExpertOptions.value.slice(0, collapsedExpertLimit)
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
