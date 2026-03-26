export const inspectorTw = {
  root: 'h-full overflow-hidden rounded-[var(--rm-radius-lg)] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))] shadow-[var(--rm-shadow-xs)]',
  header:
    'flex h-14 min-w-0 items-center justify-between gap-2 border-b border-[hsl(var(--rm-border))] px-4',
  headerActions: 'flex shrink-0 items-center gap-1',
  titleWrap: 'min-w-0',
  title: 'truncate text-[1.02rem] font-semibold text-[hsl(var(--rm-text))]',
  subtitle: 'mt-0.5 truncate text-[0.7rem] uppercase tracking-[0.08em] text-[hsl(var(--rm-text-secondary))]',
  body: 'h-[calc(100%-56px)] overflow-y-auto px-4 py-4',
  section: 'mb-5 min-w-0 overflow-hidden',
  sectionTitle:
    'mb-2 overflow-hidden text-ellipsis whitespace-nowrap text-[0.72rem] font-semibold uppercase tracking-[0.08em] text-[hsl(var(--rm-text-secondary))]',
  monoBlock:
    'min-w-0 overflow-hidden rounded-[10px] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface-muted))] p-3 font-mono text-[0.73rem] leading-5 text-[hsl(var(--rm-text-secondary))]',
  monoLine:
    'overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.72rem] leading-5 text-[hsl(var(--rm-text-secondary))]',
  list: 'space-y-2',
  listItem:
    'min-w-0 overflow-hidden rounded-[10px] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface-elevated))] px-3 py-2 text-[0.75rem] text-[hsl(var(--rm-text-secondary))]',
  relationCard:
    'group relative block w-full min-w-0 overflow-hidden rounded-[10px] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface-elevated))] px-3 py-2 text-left text-[0.75rem] text-[hsl(var(--rm-text-secondary))] transition-[border-color,background-color,box-shadow,transform,opacity] duration-120 hover:z-10 hover:-translate-y-px hover:border-[hsl(var(--rm-accent))]/45 hover:bg-[hsl(var(--rm-accent-soft))]/30 hover:shadow-[var(--rm-shadow-sm)] focus-visible:z-10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--rm-accent))]/45 disabled:pointer-events-none disabled:opacity-55',
  relationCardTop: 'mb-1.5 flex items-center justify-between gap-2',
  relationJumpHint:
    'inline-flex items-center gap-1 rounded-full border border-[hsl(var(--rm-accent))]/35 bg-[hsl(var(--rm-accent-soft))]/60 px-2 py-0.5 text-[0.62rem] font-semibold uppercase tracking-[0.08em] text-[hsl(var(--rm-accent))] transition-opacity duration-120 group-hover:opacity-100 opacity-90',
  relationTitle:
    'overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.7rem] text-[hsl(var(--rm-text))]',
  signatureGroup: 'space-y-2',
  signatureRow:
    'min-w-0 overflow-hidden rounded-[10px] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface-muted))] px-3 py-2',
  signatureKey:
    'overflow-hidden text-ellipsis whitespace-nowrap text-[0.66rem] uppercase tracking-[0.08em] text-[hsl(var(--rm-text-tertiary))]',
  signatureValue:
    'mt-1 overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.72rem] text-[hsl(var(--rm-text))]',
  signatureMeta:
    'mt-1 overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.68rem] text-[hsl(var(--rm-text-secondary))]',
}
