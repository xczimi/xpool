import type { ReactNode } from 'react'

/** A small section heading grouping related settings rows. */
export function SegHeading({ children }: { children: ReactNode }) {
  return <div className="seg-heading">{children}</div>
}

/** A labelled settings row: a visible text label beside its control. */
export function SegRow({
  label,
  children,
}: {
  label: string
  children: ReactNode
}) {
  return (
    <div className="seg-row">
      <span className="seg-label">{label}</span>
      {children}
    </div>
  )
}

interface SegToggleProps<T extends string> {
  /** Visible row label, also the radiogroup's accessible name by default. */
  label: string
  options: readonly T[]
  value: T
  onChange: (next: T) => void
  /** Visible text on each segment. */
  renderOption: (option: T) => ReactNode
  /** Per-segment accessible name, when the visible text is an abbreviation. */
  optionAriaLabel?: (option: T) => string
  /** Per-segment disabled predicate (e.g. a combination that renders empty). */
  isDisabled?: (option: T) => boolean
}

/**
 * A labelled segmented toggle: a row label plus a `radiogroup` of buttons in the
 * shared `.seg-toggle` style. Used for the language, display and theme-mode
 * pickers so each reads as a self-explanatory settings row.
 */
export function SegToggle<T extends string>({
  label,
  options,
  value,
  onChange,
  renderOption,
  optionAriaLabel,
  isDisabled,
}: SegToggleProps<T>) {
  return (
    <SegRow label={label}>
      <div className="seg-toggle" role="radiogroup" aria-label={label}>
        {options.map((option) => (
          <button
            key={option}
            type="button"
            role="radio"
            aria-checked={option === value}
            aria-label={optionAriaLabel?.(option)}
            title={optionAriaLabel?.(option)}
            disabled={isDisabled?.(option) ?? false}
            className={`seg-option${option === value ? ' is-active' : ''}`}
            onClick={() => onChange(option)}
          >
            {renderOption(option)}
          </button>
        ))}
      </div>
    </SegRow>
  )
}
