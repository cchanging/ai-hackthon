import { cva } from 'class-variance-authority'

export const nodeCardTw = {
  base: 'relative flex min-w-[170px] max-w-[220px] select-none flex-col gap-2 overflow-hidden rounded-[12px] border border-transparent px-3 py-2.5 transition-[transform,opacity,border-color,box-shadow] duration-160 ease-apple',
  kindBadge:
    'inline-flex max-w-full w-fit self-start overflow-hidden text-ellipsis whitespace-nowrap rounded-full border border-transparent px-2.5 py-1 text-[0.66rem] font-semibold uppercase tracking-[0.09em]',
  label: 'overflow-hidden text-ellipsis whitespace-nowrap text-[0.83rem] font-semibold text-[hsl(var(--rm-text))]',
  id: 'overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.66rem] text-[hsl(var(--rm-text-secondary))]',
}

export const nodeCardVariants = cva(nodeCardTw.base, {
  variants: {
    state: {
      default:
        'border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))] shadow-[var(--rm-shadow-xs)] hover:-translate-y-px hover:border-[hsl(var(--rm-border-strong))] hover:shadow-[var(--rm-shadow-sm)]',
      selected:
        'border-[hsl(var(--rm-accent))]/65 bg-[hsl(var(--rm-surface))] shadow-[var(--rm-shadow-md)] ring-2 ring-[hsl(var(--rm-accent))]/20',
      dimmed:
        'border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))] opacity-28 shadow-none',
    },
  },
  defaultVariants: {
    state: 'default',
  },
})
