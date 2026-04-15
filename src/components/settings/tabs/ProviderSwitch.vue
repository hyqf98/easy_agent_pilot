<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  useProviderProfileStore,
  type ProviderProfile,
  type CliType,
  type CreateProviderProfileInput,
  type UpdateProviderProfileInput
} from '@/stores/providerProfile'
import ProviderConnectionInfoCard from '@/components/settings/provider-switch/ProviderConnectionInfoCard.vue'
import ProviderConfigEditorModal from '@/components/settings/provider-switch/ProviderConfigEditorModal.vue'
import ProviderDeleteDialog from '@/components/settings/provider-switch/ProviderDeleteDialog.vue'
import ProviderProfilesSection from '@/components/settings/provider-switch/ProviderProfilesSection.vue'
import ProviderSwitchTabs from '@/components/settings/provider-switch/ProviderSwitchTabs.vue'
import { useNotificationStore } from '@/stores/notification'
import {
  useDefaultCliConfigEditor,
  type DefaultCliConfigLocateTarget
} from '@/composables/useDefaultCliConfigEditor'
import ProviderProfileForm from './ProviderProfileForm.vue'

const { t } = useI18n()
const store = useProviderProfileStore()
const notificationStore = useNotificationStore()

const currentCliType = ref<CliType>('claude')
const showFormModal = ref(false)
const editingProfile = ref<ProviderProfile | null>(null)
const showDeleteConfirm = ref(false)
const deletingProfile = ref<ProviderProfile | null>(null)
const switchingId = ref<string | null>(null)
const showApiKey = ref(false)
const isEditingCurrentConfig = ref(false)
const {
  configEditorContent,
  configEditorFile,
  configEditorLocateTarget,
  formatConfigEditor: handleFormatConfigEditor,
  isConfigEditorDirty,
  isConfigEditorLoading,
  isConfigEditorSaving,
  openConfigEditor,
  reloadConfigEditor,
  saveConfigEditor: handleSaveConfigEditor,
  showConfigEditor
} = useDefaultCliConfigEditor({
  onAfterSave: async (cliType) => {
    await store.refreshCliTypeState(cliType)
  }
})

const currentProfiles = computed(() => {
  if (currentCliType.value === 'claude') return store.claudeProfiles
  if (currentCliType.value === 'codex') return store.codexProfiles
  return store.opencodeProfiles
})

const currentActiveProfile = computed(() => {
  if (currentCliType.value === 'claude') return store.activeClaudeProfile
  if (currentCliType.value === 'codex') return store.activeCodexProfile
  return store.activeOpencodeProfile
})

const currentConnection = computed(() => {
  if (currentCliType.value === 'claude') return store.claudeConnection
  if (currentCliType.value === 'codex') return store.codexConnection
  return store.opencodeConnection
})

const currentDefaultProfile = computed<ProviderProfile | null>(() => {
  if (!store.currentConfig || store.currentConfig.cliType !== currentCliType.value) {
    return null
  }

  return {
    ...store.currentConfig,
    name: t('settings.providerSwitch.defaultConfigName')
  }
})

function handleCliTypeChange(type: CliType) {
  currentCliType.value = type
  store.currentCliType = type
  showApiKey.value = false
}

function handleAdd() {
  editingProfile.value = null
  isEditingCurrentConfig.value = false
  showFormModal.value = true
}

function handleEdit(profile: ProviderProfile) {
  editingProfile.value = { ...profile }
  isEditingCurrentConfig.value = !profile.id
  showFormModal.value = true
}

async function handleSwitch(profile: ProviderProfile) {
  switchingId.value = profile.id
  try {
    await store.switchProfile(profile.id)
    showSuccess(t('settings.providerSwitch.messages.switchSuccess'))
  } catch (error) {
    console.error('Switch failed:', error)
    showError(t('settings.providerSwitch.messages.switchFailed'))
  } finally {
    switchingId.value = null
  }
}

function handleDeleteConfirm(profile: ProviderProfile) {
  deletingProfile.value = profile
  showDeleteConfirm.value = true
}

async function handleDelete() {
  if (!deletingProfile.value) {
    return
  }

  try {
    await store.deleteProfile(deletingProfile.value.id)
    showDeleteConfirm.value = false
    deletingProfile.value = null
    showSuccess(t('settings.providerSwitch.messages.deleteSuccess'))
  } catch (error) {
    console.error('Delete failed:', error)
    showError(t('settings.providerSwitch.messages.deleteFailed'))
  }
}

async function handleSave(input: CreateProviderProfileInput | UpdateProviderProfileInput) {
  try {
    if (isEditingCurrentConfig.value) {
      await store.updateCurrentConfig(currentCliType.value, input as UpdateProviderProfileInput)
    } else if (editingProfile.value) {
      await store.updateProfile(editingProfile.value.id, input as UpdateProviderProfileInput)
    } else {
      await store.createProfile(input as CreateProviderProfileInput)
    }

    await store.refreshCliTypeState(currentCliType.value, { reloadProfiles: true })

    showSuccess((editingProfile.value || isEditingCurrentConfig.value)
      ? t('settings.providerSwitch.messages.updateSuccess')
      : t('settings.providerSwitch.messages.createSuccess'))
    showFormModal.value = false
    editingProfile.value = null
    isEditingCurrentConfig.value = false
  } catch (error) {
    console.error('Save failed:', error)
    showError((editingProfile.value || isEditingCurrentConfig.value)
      ? t('settings.providerSwitch.messages.updateFailed')
      : t('settings.providerSwitch.messages.createFailed'))
    throw error
  }
}

function showSuccess(message: string) {
  notificationStore.success(message)
}

function showError(message: string) {
  notificationStore.error(t('common.error'), message)
}

async function handleOpenConfigEditor(target?: DefaultCliConfigLocateTarget) {
  await openConfigEditor(currentCliType.value, target)
}

async function handleReloadConfigEditor() {
  await reloadConfigEditor(currentCliType.value)
}

function handleDeleteDialogVisibleChange(visible: boolean) {
  showDeleteConfirm.value = visible
  if (!visible) {
    deletingProfile.value = null
  }
}

onMounted(async () => {
  await store.loadProfiles()
  await store.refreshCliTypeState(currentCliType.value)
})

watch(currentCliType, async (type) => {
  await store.refreshCliTypeState(type)
})

watch(showFormModal, (visible) => {
  if (!visible) {
    editingProfile.value = null
    isEditingCurrentConfig.value = false
  }
})
</script>

<template>
  <div class="provider-switch">
    <div class="header">
      <h2 class="title">
        {{ t('settings.providerSwitch.title') }}
      </h2>
      <p class="description">
        {{ t('settings.providerSwitch.description') }}
      </p>
    </div>

    <ProviderSwitchTabs
      :current-cli-type="currentCliType"
      @change="handleCliTypeChange"
    />

    <ProviderConnectionInfoCard
      :loading="store.isLoadingConnections"
      :connection="currentConnection"
      :show-api-key="showApiKey"
      @toggle-api-key="showApiKey = !showApiKey"
      @open-config-editor="handleOpenConfigEditor"
    />

    <ProviderProfilesSection
      :loading="store.isLoading"
      :profiles="currentProfiles"
      :active-profile="currentActiveProfile"
      :default-profile="currentActiveProfile ? null : currentDefaultProfile"
      :switching-id="switchingId"
      @add="handleAdd"
      @edit="handleEdit"
      @switch="handleSwitch"
      @delete="handleDeleteConfirm"
    />

    <ProviderProfileForm
      v-model:visible="showFormModal"
      :profile="editingProfile"
      :cli-type="currentCliType"
      @save="handleSave"
    />

    <ProviderDeleteDialog
      :visible="showDeleteConfirm"
      :profile-name="deletingProfile?.name"
      @update:visible="handleDeleteDialogVisibleChange"
      @confirm="handleDelete"
    />

    <ProviderConfigEditorModal
      v-model:visible="showConfigEditor"
      :loading="isConfigEditorLoading"
      :saving="isConfigEditorSaving"
      :file="configEditorFile"
      :content="configEditorContent"
      :dirty="isConfigEditorDirty"
      :locate-target="configEditorLocateTarget"
      @update:content="configEditorContent = $event"
      @reload="handleReloadConfigEditor"
      @format="handleFormatConfigEditor"
      @save="handleSaveConfigEditor"
    />
  </div>
</template>

<style scoped>
.provider-switch {
  padding: 16px;
}

.header {
  margin-bottom: 24px;
}

.title {
  margin: 0 0 8px;
  color: var(--color-text-primary, #1a1a1a);
  font-size: 20px;
  font-weight: 600;
}

.description {
  color: var(--color-text-secondary, #666);
  font-size: 14px;
}
</style>
