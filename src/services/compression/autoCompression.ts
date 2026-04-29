export interface AutoCompressionCheckInput {
  autoCompressionEnabled: boolean
  meaningfulMessageCount: number
  usagePercentage: number
  threshold: number
}

export function shouldAutoCompressByThreshold(input: AutoCompressionCheckInput): boolean {
  return input.autoCompressionEnabled
    && input.meaningfulMessageCount > 0
    && input.usagePercentage >= input.threshold
}
