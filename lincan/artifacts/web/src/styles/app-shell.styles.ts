import { cva } from 'class-variance-authority'

export const appShellTw = {
  root: 'h-dvh w-full overflow-hidden bg-[hsl(var(--rm-bg))] text-[hsl(var(--rm-text))] font-sans',
  main: 'h-[calc(100dvh-56px)] px-4 pb-4 pt-3',
  layoutBase:
    'flex h-full min-h-0 gap-4 rounded-[var(--rm-radius-xl)] border border-[hsl(var(--rm-border))]/65 bg-[hsl(var(--rm-surface-elevated))] p-3 shadow-[var(--rm-shadow-sm)]',
}

export const inspectorWidthVariants = cva('transition-[width,opacity,transform] duration-280 ease-apple', {
  variants: {
    open: {
      true: 'w-[360px] opacity-100 translate-x-0',
      false: 'w-0 opacity-0 translate-x-6 pointer-events-none',
    },
  },
  defaultVariants: {
    open: false,
  },
})
