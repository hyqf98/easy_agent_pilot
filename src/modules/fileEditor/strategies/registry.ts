import { builtinLanguageStrategies } from './builtinStrategies'
import type { LanguageStrategy, LanguageStrategyContext } from './languageStrategy'

const customStrategies: LanguageStrategy[] = []

const normalizeExtension = (filePath: string): string => {
  const fileName = filePath.split(/[\\/]/).pop() ?? filePath
  const lowerName = fileName.toLowerCase()

  if (lowerName.endsWith('.d.ts')) {
    return 'd.ts'
  }

  const lastDot = lowerName.lastIndexOf('.')
  if (lastDot < 0 || lastDot === lowerName.length - 1) {
    return ''
  }

  return lowerName.slice(lastDot + 1)
}

const createContext = (filePath: string): LanguageStrategyContext => {
  const normalizedPath = filePath.replace(/\\/g, '/')
  const fileName = normalizedPath.split('/').pop() ?? normalizedPath
  return {
    filePath: normalizedPath,
    fileName,
    extension: normalizeExtension(fileName)
  }
}

export function registerLanguageStrategy(strategy: LanguageStrategy): void {
  const existingIndex = customStrategies.findIndex(item => item.id === strategy.id)
  if (existingIndex >= 0) {
    customStrategies.splice(existingIndex, 1, strategy)
    return
  }
  customStrategies.unshift(strategy)
}

export function getLanguageStrategy(filePath: string): LanguageStrategy {
  const ctx = createContext(filePath)
  const strategyPool = [...customStrategies, ...builtinLanguageStrategies]
  const strategy = strategyPool.find(item => item.match(ctx))
  return strategy ?? builtinLanguageStrategies[builtinLanguageStrategies.length - 1]
}

export function listLanguageStrategies(): LanguageStrategy[] {
  return [...customStrategies, ...builtinLanguageStrategies]
}
