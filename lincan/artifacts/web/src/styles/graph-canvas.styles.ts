export const graphCanvasTw = {
  root: 'relative min-w-0 flex-1 overflow-hidden rounded-[var(--rm-radius-lg)] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))] shadow-[var(--rm-shadow-xs)]',
  searchWrap:
    'absolute left-3 top-3 z-40 w-[min(360px,calc(100%-1.5rem))]',
  searchInputWrap:
    'flex items-center gap-2 rounded-[10px] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))]/95 px-2.5 py-2 shadow-[var(--rm-shadow-sm)] backdrop-blur-[8px]',
  searchIcon: 'h-4 w-4 shrink-0 text-[hsl(var(--rm-text-tertiary))]',
  searchInput:
    'w-full min-w-0 bg-transparent text-[0.75rem] text-[hsl(var(--rm-text))] outline-none placeholder:text-[hsl(var(--rm-text-tertiary))]',
  searchClear:
    'inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-[7px] border border-transparent text-[hsl(var(--rm-text-tertiary))] transition-colors duration-120 hover:border-[hsl(var(--rm-border))] hover:text-[hsl(var(--rm-text))]',
  searchResultPanel:
    'mt-2 max-h-[260px] overflow-y-auto rounded-[10px] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))]/96 p-1.5 shadow-[var(--rm-shadow-sm)] backdrop-blur-[8px]',
  searchResultItem:
    'w-full min-w-0 rounded-[8px] border border-transparent px-2 py-1.5 text-left transition-[background-color,border-color,color] duration-120 hover:border-[hsl(var(--rm-border-strong))] hover:bg-[hsl(var(--rm-surface-muted))]',
  searchResultTop: 'flex items-center justify-between gap-2',
  searchResultLabel:
    'overflow-hidden text-ellipsis whitespace-nowrap text-[0.72rem] font-semibold text-[hsl(var(--rm-text))]',
  searchResultKind:
    'shrink-0 rounded-full border border-[hsl(var(--rm-accent))]/35 bg-[hsl(var(--rm-accent-soft))]/55 px-1.5 py-0.5 text-[0.6rem] font-semibold uppercase tracking-[0.08em] text-[hsl(var(--rm-accent))]',
  searchResultId:
    'mt-1 overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.66rem] text-[hsl(var(--rm-text-secondary))]',
  searchEmpty:
    'px-2 py-1.5 text-[0.7rem] text-[hsl(var(--rm-text-tertiary))]',
  emptyState:
    'absolute inset-0 z-20 grid place-items-center bg-[linear-gradient(160deg,hsl(var(--rm-surface-elevated)),hsl(var(--rm-surface)))]',
  emptyInner: 'max-w-[420px] px-6 text-center',
  emptyTitle: 'text-[1.15rem] font-semibold text-[hsl(var(--rm-text))]',
  emptyBody: 'mt-2 text-[0.85rem] leading-6 text-[hsl(var(--rm-text-secondary))]',
  edgeTooltip:
    'pointer-events-none absolute z-30 max-w-[260px] overflow-hidden rounded-[10px] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))]/95 px-2.5 py-2 shadow-[var(--rm-shadow-sm)] backdrop-blur-[8px]',
  edgeTooltipTitle:
    'overflow-hidden text-ellipsis whitespace-nowrap text-[0.66rem] font-semibold uppercase tracking-[0.08em] text-[hsl(var(--rm-text))]',
  edgeTooltipBody:
    'mt-1 overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.68rem] text-[hsl(var(--rm-text-secondary))]',
}
