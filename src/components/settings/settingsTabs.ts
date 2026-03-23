import { markRaw, type Component } from 'vue'
import type { SettingsTab } from '@/stores/ui'
import GeneralSettings from './tabs/GeneralSettings.vue'
import AgentSettings from './tabs/AgentSettings.vue'
import SkillConfigPage from '@/components/skill-config/SkillConfigPage.vue'
import ProviderSwitch from './tabs/ProviderSwitch.vue'
import ThemeSettings from './tabs/ThemeSettings.vue'
import DataSettings from './tabs/DataSettings.vue'
import LogSettings from './tabs/LogSettings.vue'
import LspSettings from './tabs/LspSettings.vue'
import SessionManagementSettings from './tabs/SessionManagementSettings.vue'
import AppUpdateSettings from './tabs/AppUpdateSettings.vue'
import MarketplacePage from '@/components/marketplace/MarketplacePage.vue'

export interface SettingsTabDescriptor {
  id: SettingsTab
  labelKey: string
  icon: string
  component: Component
  layout: 'default' | 'full'
}

export const SETTINGS_TAB_DESCRIPTORS: SettingsTabDescriptor[] = [
  {
    id: 'general',
    labelKey: 'settings.nav.general',
    icon: 'settings',
    component: markRaw(GeneralSettings),
    layout: 'default'
  },
  {
    id: 'agents',
    labelKey: 'settings.nav.agents',
    icon: 'bot',
    component: markRaw(AgentSettings),
    layout: 'default'
  },
  {
    id: 'agentConfig',
    labelKey: 'settings.nav.agentConfig',
    icon: 'settings-2',
    component: markRaw(SkillConfigPage),
    layout: 'full'
  },
  {
    id: 'marketplace',
    labelKey: 'settings.nav.marketplace',
    icon: 'store',
    component: markRaw(MarketplacePage),
    layout: 'full'
  },
  {
    id: 'providerSwitch',
    labelKey: 'settings.nav.providerSwitch',
    icon: 'repeat',
    component: markRaw(ProviderSwitch),
    layout: 'default'
  },
  {
    id: 'appUpdate',
    labelKey: 'settings.nav.appUpdate',
    icon: 'download',
    component: markRaw(AppUpdateSettings),
    layout: 'default'
  },
  {
    id: 'sessions',
    labelKey: 'settings.nav.sessions',
    icon: 'history',
    component: markRaw(SessionManagementSettings),
    layout: 'default'
  },
  {
    id: 'theme',
    labelKey: 'settings.nav.theme',
    icon: 'palette',
    component: markRaw(ThemeSettings),
    layout: 'default'
  },
  {
    id: 'lsp',
    labelKey: 'settings.nav.lsp',
    icon: 'languages',
    component: markRaw(LspSettings),
    layout: 'default'
  },
  {
    id: 'data',
    labelKey: 'settings.nav.data',
    icon: 'database',
    component: markRaw(DataSettings),
    layout: 'default'
  },
  {
    id: 'logs',
    labelKey: 'settings.nav.logs',
    icon: 'scroll-text',
    component: markRaw(LogSettings),
    layout: 'full'
  }
]

const settingsTabDescriptorMap = new Map(
  SETTINGS_TAB_DESCRIPTORS.map((descriptor) => [descriptor.id, descriptor])
)

export function getSettingsTabDescriptor(tab: SettingsTab): SettingsTabDescriptor {
  return settingsTabDescriptorMap.get(tab) ?? SETTINGS_TAB_DESCRIPTORS[0]
}
