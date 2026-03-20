function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function readRawValue(value: Record<string, unknown> | undefined): string {
  if (!value) {
    return ''
  }

  return typeof value.raw === 'string' ? value.raw : ''
}

function mergeToolInputObjects(
  previous: Record<string, unknown> | undefined,
  next: Record<string, unknown>
): Record<string, unknown> {
  const previousRaw = readRawValue(previous)
  const nextRaw = readRawValue(next)
  const mergedRaw = `${previousRaw}${nextRaw}`

  const previousWithoutRaw = previous ? Object.fromEntries(
    Object.entries(previous).filter(([key]) => key !== 'raw')
  ) : {}
  const nextWithoutRaw = Object.fromEntries(
    Object.entries(next).filter(([key]) => key !== 'raw')
  )

  if (mergedRaw) {
    try {
      const parsed = JSON.parse(mergedRaw) as unknown
      if (isRecord(parsed)) {
        return {
          ...previousWithoutRaw,
          ...nextWithoutRaw,
          ...parsed
        }
      }
    } catch {
      return {
        ...previousWithoutRaw,
        ...nextWithoutRaw,
        raw: mergedRaw
      }
    }
  }

  return {
    ...previousWithoutRaw,
    ...nextWithoutRaw
  }
}

export function mergeToolInputArguments(
  previous: Record<string, unknown> | undefined,
  next: Record<string, unknown> | undefined
): Record<string, unknown> {
  if (!previous && !next) {
    return {}
  }

  if (!previous) {
    return next ? { ...next } : {}
  }

  if (!next) {
    return { ...previous }
  }

  return mergeToolInputObjects(previous, next)
}
