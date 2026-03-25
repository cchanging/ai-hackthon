import { MoonStar, SunMedium, Upload, LocateFixed, RefreshCcw, CircleX, ExternalLink } from 'lucide-react'
import { useRef } from 'react'
import { buttonVariants, iconButtonVariants } from '../styles/ui.styles'
import { topBarTw } from '../styles/top-bar.styles'
import { cn } from '../lib/cn'
import type { ThemeMode } from '../app/types'

interface TopBarProps {
  theme: ThemeMode
  hasArtifact: boolean
  hasSelection: boolean
  onToggleTheme: () => void
  onUploadFile: (file: File) => Promise<void>
  onFitView: () => void
  onResetLayout: () => void
  onClearSelection: () => void
}

export function TopBar({
  theme,
  hasArtifact,
  hasSelection,
  onToggleTheme,
  onUploadFile,
  onFitView,
  onResetLayout,
  onClearSelection,
}: TopBarProps) {
  const fileInputRef = useRef<HTMLInputElement | null>(null)

  const handleUploadClick = () => {
    fileInputRef.current?.click()
  }

  const handleFileChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    if (!file) {
      return
    }

    await onUploadFile(file)
    event.target.value = ''
  }

  return (
    <header className={topBarTw.root}>
      <div className={topBarTw.brandWrap}>
        <div className={topBarTw.brandIcon}>
          <span className="text-[1rem] font-bold">R</span>
        </div>
        <div>
          <p className={topBarTw.brandTitle}>RustMap</p>
          <p className={topBarTw.brandSubtitle}>Rust static structure workspace</p>
        </div>
      </div>

      <div className={topBarTw.actions}>
        <input
          ref={fileInputRef}
          type="file"
          className="hidden"
          accept=".json,application/json"
          onChange={handleFileChange}
        />

        <button
          type="button"
          className={cn(buttonVariants({ variant: 'surface' }))}
          onClick={handleUploadClick}
        >
          <Upload className="h-4 w-4" />
          Upload JSON
        </button>

        <button
          type="button"
          className={cn(iconButtonVariants({ variant: 'surface' }))}
          onClick={onFitView}
          disabled={!hasArtifact}
          title="Fit view"
          aria-label="Fit view"
        >
          <LocateFixed className="h-4 w-4" />
        </button>

        <button
          type="button"
          className={cn(iconButtonVariants({ variant: 'surface' }))}
          onClick={onResetLayout}
          disabled={!hasArtifact}
          title="Reset layout"
          aria-label="Reset layout"
        >
          <RefreshCcw className="h-4 w-4" />
        </button>

        <button
          type="button"
          className={cn(iconButtonVariants({ variant: 'surface' }))}
          onClick={onClearSelection}
          disabled={!hasSelection}
          title="Clear selection"
          aria-label="Clear selection"
        >
          <CircleX className="h-4 w-4" />
        </button>

        <a
          href="https://github.com/Linermao/rustmap"
          target="_blank"
          rel="noreferrer noopener"
          className={cn(buttonVariants({ variant: 'surface' }), 'h-9 px-2.5 py-1')}
          title="Open GitHub repository"
          aria-label="Open GitHub repository"
        >
          <ExternalLink className="h-3.5 w-3.5" />
          GitHub
        </a>

        <button
          type="button"
          className={cn(iconButtonVariants({ variant: 'surface' }))}
          onClick={onToggleTheme}
          title="Toggle theme"
          aria-label="Toggle theme"
        >
          {theme === 'light' ? (
            <MoonStar className="h-4 w-4" />
          ) : (
            <SunMedium className="h-4 w-4" />
          )}
        </button>
      </div>
    </header>
  )
}
