export interface SourceLocation {
  file: string
  line: number
  col: number
}

function normalizeFilePath(file: string): string {
  const trimmed = file.trim()
  if (!trimmed) {
    return '/'
  }

  const windowsAbsolutePathPattern = /^[a-zA-Z]:[\\/]/u
  if (windowsAbsolutePathPattern.test(trimmed)) {
    return `/${trimmed.replaceAll('\\', '/')}`
  }

  if (trimmed.startsWith('/')) {
    return trimmed
  }

  return `/${trimmed}`
}

export function buildVscodeFileUri(location: SourceLocation): string {
  const normalizedPath = normalizeFilePath(location.file)
  const encodedPath = encodeURI(normalizedPath)
  return `vscode://file${encodedPath}:${location.line}:${location.col}`
}

export function locationToClipboardText(location: SourceLocation): string {
  return `${location.file}:${location.line}:${location.col}`
}

export async function copyLocation(location: SourceLocation): Promise<boolean> {
  const copyText = locationToClipboardText(location)

  if (typeof navigator !== 'undefined' && navigator.clipboard) {
    try {
      await navigator.clipboard.writeText(copyText)
      return true
    } catch {
      // Fall through to the legacy copy path.
    }
  }

  if (typeof document === 'undefined') {
    return false
  }

  const textarea = document.createElement('textarea')
  textarea.value = copyText
  textarea.setAttribute('readonly', 'true')
  textarea.style.position = 'fixed'
  textarea.style.opacity = '0'
  textarea.style.pointerEvents = 'none'
  textarea.style.left = '-9999px'

  document.body.appendChild(textarea)
  textarea.focus()
  textarea.select()
  textarea.setSelectionRange(0, textarea.value.length)

  const copied = document.execCommand('copy')
  textarea.remove()

  return copied
}
