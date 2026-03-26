export const leftRailTw = {
  root: 'flex h-full w-[264px] shrink-0 flex-col gap-4 overflow-y-auto rounded-[var(--rm-radius-lg)] bg-[hsl(var(--rm-surface))] p-4 shadow-[var(--rm-shadow-xs)]',
  sectionTitle:
    'mb-2 overflow-hidden text-ellipsis whitespace-nowrap text-[0.78rem] font-semibold uppercase tracking-[0.08em] text-[hsl(var(--rm-text-secondary))]',
  statGrid: 'grid grid-cols-3 gap-2',
  statItem:
    'min-w-0 overflow-hidden rounded-[10px] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface-elevated))] px-2.5 py-2',
  statLabel:
    'overflow-hidden text-ellipsis whitespace-nowrap text-[0.64rem] uppercase tracking-[0.08em] text-[hsl(var(--rm-text-tertiary))]',
  statValue:
    'mt-1 overflow-hidden text-ellipsis whitespace-nowrap text-[0.95rem] font-semibold text-[hsl(var(--rm-text))]',
  filterGroup: 'flex flex-wrap gap-2',
  filterMeta: 'mb-3 text-[0.7rem] text-[hsl(var(--rm-text-tertiary))]',
  filterToolbar: 'mb-3 flex gap-2',
  filterActionButton:
    'inline-flex min-w-0 items-center justify-center rounded-[9px] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface-elevated))] px-2.5 py-1.5 text-[0.66rem] font-semibold uppercase tracking-[0.08em] text-[hsl(var(--rm-text-secondary))] transition-[border-color,color,background-color,transform] duration-120 hover:-translate-y-px hover:border-[hsl(var(--rm-border-strong))] hover:text-[hsl(var(--rm-text))]',
  filterList: 'space-y-2',
  filterRow:
    'flex min-w-0 items-center gap-2 rounded-[10px] border px-2.5 py-2 transition-[border-color,background-color,box-shadow,transform,opacity] duration-120',
  filterRowActive:
    'border-[hsl(var(--rm-border-strong))] bg-[hsl(var(--rm-surface-elevated))] shadow-[var(--rm-shadow-xs)]',
  filterRowInactive:
    'border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))] opacity-65 hover:opacity-90',
  filterRowMain:
    'flex min-w-0 flex-1 items-center gap-2 overflow-hidden text-left',
  filterSwatch: 'h-2.5 w-2.5 shrink-0 rounded-full',
  filterKindText:
    'truncate text-[0.75rem] font-semibold uppercase tracking-[0.08em] text-[hsl(var(--rm-text))]',
  filterCountText:
    'ml-auto shrink-0 rounded-full border border-[hsl(var(--rm-border))] px-1.5 py-0.5 text-[0.64rem] font-semibold text-[hsl(var(--rm-text-secondary))]',
  filterOnlyButton:
    'inline-flex shrink-0 items-center justify-center rounded-[8px] border border-transparent px-2 py-1 text-[0.62rem] font-semibold uppercase tracking-[0.08em] text-[hsl(var(--rm-text-tertiary))] transition-[color,border-color,background-color] duration-120 hover:border-[hsl(var(--rm-border-strong))] hover:bg-[hsl(var(--rm-surface-muted))] hover:text-[hsl(var(--rm-text))]',
  warningList: 'flex max-h-[180px] flex-col gap-2 overflow-y-auto pr-1',
  warningItem:
    'overflow-hidden text-ellipsis whitespace-nowrap rounded-[10px] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface-muted))] px-3 py-2 text-[0.75rem] leading-5 text-[hsl(var(--rm-text-secondary))]',
}
