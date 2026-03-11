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
import ProviderDeleteDialog from '@/components/settings/provider-switch/ProviderDeleteDialog.vue'
import ProviderProfilesSection from '@/components/settings/provider-switch/ProviderProfilesSection.vue'
import ProviderSwitchTabs from '@/components/settings/provider-switch/ProviderSwitchTabs.vue'
import ProviderProfileForm from './ProviderProfileForm.vue'

const { t } = useI18n()
const store = useProviderProfileStore()

const currentCliType = ref<CliType>('claude')
const showFormModal = ref(false)
const editingProfile = ref<ProviderProfile | null>(null)
const showDeleteConfirm = ref(false)
const deletingProfile = ref<ProviderProfile | null>(null)
const switchingId = ref<string | null>(null)
const showApiKey = ref(false)

const currentProfiles = computed(() =>
  currentCliType.value === 'claude' ? store.claudeProfiles : store.codexProfiles
)

const currentActiveProfile = computed(() =>
  currentCliType.value === 'claude' ? store.activeClaudeProfile : store.activeCodexProfile
)

const currentConnection = computed(() =>
  currentCliType.value === 'claude' ? store.claudeConnection : store.codexConnection
)

function handleCliTypeChange(type: CliType) {
  currentCliType.value = type
  store.currentCliType = type
  showApiKey.value = false
}

function handleAdd() {
  editingProfile.value = null
  showFormModal.value = true
}

function handleEdit(profile: ProviderProfile) {
  editingProfile.value = { ...profile }
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
    if (editingProfile.value) {
      await store.updateProfile(editingProfile.value.id, input as UpdateProviderProfileInput)
      showSuccess(t('settings.providerSwitch.messages.updateSuccess'))
    } else {
      await store.createProfile(input as CreateProviderProfileInput)
      showSuccess(t('settings.providerSwitch.messages.createSuccess'))
    }
    showFormModal.value = false
    editingProfile.value = null
  } catch (error) {
    console.error('Save failed:', error)
    showError(editingProfile.value
      ? t('settings.providerSwitch.messages.updateFailed')
      : t('settings.providerSwitch.messages.createFailed'))
    throw error
  }
}

function showSuccess(_message: string) {
}

function showError(message: string) {
  console.error('Error:', message)
}

function handleDeleteDialogVisibleChange(visible: boolean) {
  showDeleteConfirm.value = visible
  if (!visible) {
    deletingProfile.value = null
  }
}

onMounted(async () => {
  await store.loadProfiles()
  await store.readAllCliConnections()
})

watch(currentCliType, async (type) => {
  await store.loadActiveProfile(type)
  await store.readCurrentConfig(type)
  await store.readCliConnectionInfo(type)
}, { immediate: true })
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
    />

    <ProviderProfilesSection
      :loading="store.isLoading"
      :profiles="currentProfiles"
      :active-profile="currentActiveProfile"
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
