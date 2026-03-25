import { cva } from 'class-variance-authority'

export const panelVariants = cva(
  'rounded-[var(--rm-radius-lg)] border border-[hsl(var(--rm-border))] shadow-[var(--rm-shadow-xs)]',
  {
    variants: {
      tone: {
        default: 'bg-[hsl(var(--rm-surface))]',
        muted: 'bg-[hsl(var(--rm-surface-muted))]',
        elevated: 'bg-[hsl(var(--rm-surface-elevated))] shadow-[var(--rm-shadow-sm)]',
      },
    },
    defaultVariants: {
      tone: 'default',
    },
  },
)

export const iconButtonVariants = cva(
  'inline-flex h-9 w-9 items-center justify-center rounded-[var(--rm-radius-md)] border text-[hsl(var(--rm-text-secondary))] transition-[background-color,border-color,color,box-shadow,transform,opacity] duration-120 hover:-translate-y-px focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--rm-accent))]/45',
  {
    variants: {
      variant: {
        ghost:
          'border-transparent bg-transparent hover:bg-[hsl(var(--rm-surface-muted))] hover:text-[hsl(var(--rm-text))]',
        surface:
          'border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))] hover:border-[hsl(var(--rm-border-strong))] hover:text-[hsl(var(--rm-text))] hover:shadow-[var(--rm-shadow-sm)]',
      },
    },
    defaultVariants: {
      variant: 'surface',
    },
  },
)

export const buttonVariants = cva(
  'inline-flex items-center gap-2 rounded-[var(--rm-radius-md)] border px-3 py-2 text-[0.78rem] font-semibold tracking-[0.02em] transition-[background-color,border-color,color,box-shadow,transform,opacity] duration-120 hover:-translate-y-px focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--rm-accent))]/50 disabled:pointer-events-none disabled:opacity-50',
  {
    variants: {
      variant: {
        surface:
          'border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))] text-[hsl(var(--rm-text-secondary))] hover:border-[hsl(var(--rm-border-strong))] hover:text-[hsl(var(--rm-text))] hover:shadow-[var(--rm-shadow-sm)]',
        accent:
          'border-transparent bg-[hsl(var(--rm-accent))] text-white shadow-[var(--rm-shadow-sm)] hover:brightness-105',
      },
    },
    defaultVariants: {
      variant: 'surface',
    },
  },
)

export const chipVariants = cva(
  'inline-flex max-w-full min-h-8 items-center justify-center overflow-hidden text-ellipsis whitespace-nowrap rounded-full border px-3 text-[0.72rem] font-semibold uppercase tracking-[0.08em] transition-[opacity,background-color,border-color,color] duration-120',
  {
    variants: {
      active: {
        true: 'opacity-100',
        false: 'opacity-45 hover:opacity-70',
      },
      tone: {
        neutral:
          'border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))] text-[hsl(var(--rm-text-secondary))]',
        kind:
          'border-transparent bg-[hsl(var(--kind-chip-bg))] text-[hsl(var(--kind-chip-text))]',
      },
    },
    defaultVariants: {
      active: true,
      tone: 'neutral',
    },
  },
)

export const kindThemeMap: Record<string, { text: string; softBg: string }> = {
  struct: {
    text: 'hsl(var(--rm-kind-struct))',
    softBg:
      'color-mix(in oklab, hsl(var(--rm-kind-struct)) 18%, hsl(var(--rm-surface)))',
  },
  enum: {
    text: 'hsl(var(--rm-kind-enum))',
    softBg:
      'color-mix(in oklab, hsl(var(--rm-kind-enum)) 17%, hsl(var(--rm-surface)))',
  },
  trait: {
    text: 'hsl(var(--rm-kind-trait))',
    softBg:
      'color-mix(in oklab, hsl(var(--rm-kind-trait)) 20%, hsl(var(--rm-surface)))',
  },
  fn: {
    text: 'hsl(var(--rm-kind-fn))',
    softBg:
      'color-mix(in oklab, hsl(var(--rm-kind-fn)) 17%, hsl(var(--rm-surface)))',
  },
  method: {
    text: 'hsl(var(--rm-kind-fn))',
    softBg:
      'color-mix(in oklab, hsl(var(--rm-kind-fn)) 24%, hsl(var(--rm-surface)))',
  },
  type: {
    text: 'hsl(var(--rm-kind-module))',
    softBg:
      'color-mix(in oklab, hsl(var(--rm-kind-module)) 17%, hsl(var(--rm-surface)))',
  },
  const: {
    text: 'hsl(var(--rm-kind-module))',
    softBg:
      'color-mix(in oklab, hsl(var(--rm-kind-module)) 17%, hsl(var(--rm-surface)))',
  },
  static: {
    text: 'hsl(var(--rm-kind-module))',
    softBg:
      'color-mix(in oklab, hsl(var(--rm-kind-module)) 17%, hsl(var(--rm-surface)))',
  },
}
