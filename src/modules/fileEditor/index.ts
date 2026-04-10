export { useFileEditorStore } from './stores/fileEditor'
export { default as FileEditorWorkspace } from './components/fileEditorWorkspace/FileEditorWorkspace.vue'
export { getLanguageStrategy, registerLanguageStrategy, listLanguageStrategies } from './strategies/registry'
export type { LanguageStrategy } from './strategies/languageStrategy'
