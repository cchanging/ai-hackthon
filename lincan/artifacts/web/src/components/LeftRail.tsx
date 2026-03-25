import { AlertTriangle, CheckCircle2, Filter, Check, Ban } from 'lucide-react'
import { leftRailTw } from '../styles/left-rail.styles'
import { panelVariants, kindThemeMap } from '../styles/ui.styles'
import { cn } from '../lib/cn'

interface KindFilterItem {
  kind: string
  total: number
  active: boolean
}

interface CrateFilterItem {
  crateName: string
  total: number
  active: boolean
}

interface LeftRailProps {
  fileName: string | null
  nodeCount: number
  crateVisibleNodeCount: number
  edgeCount: number
  warningCount: number
  crateItems: CrateFilterItem[]
  onToggleCrate: (crateName: string) => void
  onSetAllCrates: (enabled: boolean) => void
  onInvertCrates: () => void
  onIsolateCrate: (crateName: string) => void
  kindItems: KindFilterItem[]
  onToggleKind: (kind: string) => void
  onSetAllKinds: (enabled: boolean) => void
  onInvertKinds: () => void
  onIsolateKind: (kind: string) => void
  contractError: string | null
  warnings: string[]
}

function resolveKindTheme(kind: string) {
  return kindThemeMap[kind] ?? {
    text: 'hsl(var(--rm-kind-crate))',
    softBg: 'color-mix(in oklab, hsl(var(--rm-kind-crate)) 16%, white)',
  }
}

function resolveCrateTheme(crateName: string) {
  let hash = 0
  for (const char of crateName) {
    hash = (hash * 31 + char.charCodeAt(0)) >>> 0
  }
  const hue = hash % 360
  return {
    text: `hsl(${hue} 64% 42%)`,
    softBg: `hsl(${hue} 88% 95%)`,
  }
}

export function LeftRail({
  fileName,
  nodeCount,
  crateVisibleNodeCount,
  edgeCount,
  warningCount,
  crateItems,
  onToggleCrate,
  onSetAllCrates,
  onInvertCrates,
  onIsolateCrate,
  kindItems,
  onToggleKind,
  onSetAllKinds,
  onInvertKinds,
  onIsolateKind,
  contractError,
  warnings,
}: LeftRailProps) {
  const activeCrateCount = crateItems.filter((item) => item.active).length
  const activeKindCount = kindItems.filter((item) => item.active).length

  return (
    <aside className={leftRailTw.root}>
      <section className={cn(panelVariants({ tone: 'elevated' }), 'px-3.5 py-3')}>
        <p className={leftRailTw.sectionTitle}>Overview</p>
        <div className={leftRailTw.statGrid}>
          <div className={leftRailTw.statItem}>
            <p className={leftRailTw.statLabel}>Nodes</p>
            <p className={leftRailTw.statValue}>{nodeCount}</p>
          </div>
          <div className={leftRailTw.statItem}>
            <p className={leftRailTw.statLabel}>Edges</p>
            <p className={leftRailTw.statValue}>{edgeCount}</p>
          </div>
          <div className={leftRailTw.statItem}>
            <p className={leftRailTw.statLabel}>Warnings</p>
            <p className={leftRailTw.statValue}>{warningCount}</p>
          </div>
        </div>

        <p className="mt-3 truncate text-[0.7rem] text-[hsl(var(--rm-text-tertiary))]">
          {fileName ? `Loaded: ${fileName}` : 'No artifact loaded'}
        </p>
      </section>

      <section className={cn(panelVariants({ tone: 'default' }), 'px-3.5 py-3')}>
        <div className="mb-3 flex items-center justify-between">
          <p className={leftRailTw.sectionTitle}>Crate filter</p>
          <Filter className="h-3.5 w-3.5 text-[hsl(var(--rm-text-tertiary))]" />
        </div>
        <p className={leftRailTw.filterMeta}>
          Active {activeCrateCount}/{crateItems.length} · Visible {crateVisibleNodeCount} nodes
        </p>
        <div className={leftRailTw.filterToolbar}>
          <button
            type="button"
            className={leftRailTw.filterActionButton}
            onClick={() => onSetAllCrates(true)}
          >
            All
          </button>
          <button
            type="button"
            className={leftRailTw.filterActionButton}
            onClick={onInvertCrates}
          >
            Invert
          </button>
          <button
            type="button"
            className={leftRailTw.filterActionButton}
            onClick={() => onSetAllCrates(false)}
          >
            None
          </button>
        </div>

        <div className={leftRailTw.filterList}>
          {crateItems.length === 0 ? (
            <p className="text-[0.75rem] text-[hsl(var(--rm-text-tertiary))]">Upload a JSON artifact first.</p>
          ) : (
            crateItems.map((item) => {
              const active = item.active
              const theme = resolveCrateTheme(item.crateName)
              return (
                <div
                  key={item.crateName}
                  className={cn(
                    leftRailTw.filterRow,
                    active ? leftRailTw.filterRowActive : leftRailTw.filterRowInactive,
                  )}
                  style={{
                    borderColor: active ? theme.text : undefined,
                    backgroundColor: active ? theme.softBg : undefined,
                  }}
                >
                  <button
                    type="button"
                    className={leftRailTw.filterRowMain}
                    title={item.crateName}
                    onClick={() => onToggleCrate(item.crateName)}
                  >
                    <span
                      className={leftRailTw.filterSwatch}
                      style={{
                        backgroundColor: theme.text,
                      }}
                    />
                    <span className={leftRailTw.filterKindText}>{item.crateName}</span>
                    <span className={leftRailTw.filterCountText}>{item.total}</span>
                    {active ? (
                      <Check className="h-3.5 w-3.5 shrink-0 text-[hsl(var(--rm-text-secondary))]" />
                    ) : (
                      <Ban className="h-3.5 w-3.5 shrink-0 text-[hsl(var(--rm-text-tertiary))]" />
                    )}
                  </button>
                  <button
                    type="button"
                    className={leftRailTw.filterOnlyButton}
                    title={`Only ${item.crateName}`}
                    onClick={(event) => {
                      event.stopPropagation()
                      onIsolateCrate(item.crateName)
                    }}
                  >
                    Only
                  </button>
                </div>
              )
            })
          )}
        </div>
      </section>

      <section className={cn(panelVariants({ tone: 'default' }), 'px-3.5 py-3')}>
        <div className="mb-3 flex items-center justify-between">
          <p className={leftRailTw.sectionTitle}>Node kind filter</p>
          <Filter className="h-3.5 w-3.5 text-[hsl(var(--rm-text-tertiary))]" />
        </div>
        <p className={leftRailTw.filterMeta}>
          Active {activeKindCount}/{kindItems.length} · Visible {nodeCount} nodes
        </p>
        <div className={leftRailTw.filterToolbar}>
          <button
            type="button"
            className={leftRailTw.filterActionButton}
            onClick={() => onSetAllKinds(true)}
          >
            All
          </button>
          <button
            type="button"
            className={leftRailTw.filterActionButton}
            onClick={onInvertKinds}
          >
            Invert
          </button>
          <button
            type="button"
            className={leftRailTw.filterActionButton}
            onClick={() => onSetAllKinds(false)}
          >
            None
          </button>
        </div>

        <div className={leftRailTw.filterList}>
          {kindItems.length === 0 ? (
            <p className="text-[0.75rem] text-[hsl(var(--rm-text-tertiary))]">Upload a JSON artifact first.</p>
          ) : (
            kindItems.map((item) => {
              const active = item.active
              const theme = resolveKindTheme(item.kind)
              return (
                <div
                  key={item.kind}
                  className={cn(
                    leftRailTw.filterRow,
                    active ? leftRailTw.filterRowActive : leftRailTw.filterRowInactive,
                  )}
                  style={{
                    borderColor: active ? theme.text : undefined,
                    backgroundColor: active ? theme.softBg : undefined,
                  }}
                >
                  <button
                    type="button"
                    className={leftRailTw.filterRowMain}
                    title={item.kind}
                    onClick={() => onToggleKind(item.kind)}
                  >
                    <span
                      className={leftRailTw.filterSwatch}
                      style={{
                        backgroundColor: theme.text,
                      }}
                    />
                    <span className={leftRailTw.filterKindText}>{item.kind}</span>
                    <span className={leftRailTw.filterCountText}>{item.total}</span>
                    {active ? (
                      <Check className="h-3.5 w-3.5 shrink-0 text-[hsl(var(--rm-text-secondary))]" />
                    ) : (
                      <Ban className="h-3.5 w-3.5 shrink-0 text-[hsl(var(--rm-text-tertiary))]" />
                    )}
                  </button>
                  <button
                    type="button"
                    className={leftRailTw.filterOnlyButton}
                    title={`Only ${item.kind}`}
                    onClick={(event) => {
                      event.stopPropagation()
                      onIsolateKind(item.kind)
                    }}
                  >
                    Only
                  </button>
                </div>
              )
            })
          )}
        </div>
      </section>

      {contractError ? (
        <section className={cn(panelVariants({ tone: 'default' }), 'px-3.5 py-3')}>
          <div className="mb-2 flex items-center gap-2 text-[hsl(var(--rm-kind-trait))]">
            <AlertTriangle className="h-4 w-4" />
            <p className={leftRailTw.sectionTitle}>Upload Error</p>
          </div>
          <pre className="whitespace-pre-wrap text-[0.73rem] leading-5 text-[hsl(var(--rm-text-secondary))]">
            {contractError}
          </pre>
        </section>
      ) : (
        <section className={cn(panelVariants({ tone: 'default' }), 'px-3.5 py-3')}>
          <div className="mb-2 flex items-center gap-2 text-[hsl(var(--rm-kind-fn))]">
            <CheckCircle2 className="h-4 w-4" />
            <p className={leftRailTw.sectionTitle}>Validation</p>
          </div>
          <p className="text-[0.75rem] leading-5 text-[hsl(var(--rm-text-secondary))]">
            JSON upload validation is strict and checks top-level + graph contract fields.
          </p>
        </section>
      )}

      <section className={cn(panelVariants({ tone: 'default' }), 'px-3.5 py-3')}>
        <p className={leftRailTw.sectionTitle}>Warnings</p>
        {warnings.length === 0 ? (
          <p className="text-[0.75rem] text-[hsl(var(--rm-text-tertiary))]">No warning entries.</p>
        ) : (
          <div className={leftRailTw.warningList}>
            {warnings.map((warning, index) => (
              <article
                className={leftRailTw.warningItem}
                key={`${warning}-${index}`}
                title={warning}
              >
                {warning}
              </article>
            ))}
          </div>
        )}
      </section>
    </aside>
  )
}
