/**
 * Accent colour presets and dark/light mode — pure logic (no DOM, no React).
 * Mirrors the split used by `lib/displayMode.ts`: the testable core lives here;
 * the React provider that applies it to the document is separate.
 */

/** Selectable accent hue. `amber` is the default / brand baseline. */
export type Accent = 'amber' | 'green' | 'cyan' | 'magenta' | 'violet' | 'mono'

/** What the user picks for mode. `system` follows the OS preference. */
export type ThemeMode = 'system' | 'dark' | 'light'

/** A mode with `system` already resolved to a concrete palette. */
export type ResolvedTheme = 'dark' | 'light'

/** Accent options, in display order. */
export const ACCENTS: readonly Accent[] = [
  'amber',
  'green',
  'cyan',
  'magenta',
  'violet',
  'mono',
]

/** Mode options, in display order. */
export const THEME_MODES: readonly ThemeMode[] = ['system', 'dark', 'light']

export const DEFAULT_ACCENT: Accent = 'amber'
export const DEFAULT_MODE: ThemeMode = 'system'

export const ACCENT_STORAGE_KEY = 'xpool.accent'
export const MODE_STORAGE_KEY = 'xpool.themeMode'

const ACCENT_SET: ReadonlySet<string> = new Set(ACCENTS)
const MODE_SET: ReadonlySet<string> = new Set(THEME_MODES)

/** Coerce an untrusted value to a valid Accent, else the default. */
export function coerceAccent(value: unknown): Accent {
  return typeof value === 'string' && ACCENT_SET.has(value)
    ? (value as Accent)
    : DEFAULT_ACCENT
}

/** Coerce an untrusted value to a valid ThemeMode, else the default. */
export function coerceThemeMode(value: unknown): ThemeMode {
  return typeof value === 'string' && MODE_SET.has(value)
    ? (value as ThemeMode)
    : DEFAULT_MODE
}

/** Resolve a mode to a concrete palette, consulting the OS only for `system`. */
export function resolveTheme(
  mode: ThemeMode,
  systemPrefersDark: boolean,
): ResolvedTheme {
  if (mode === 'system') {
    return systemPrefersDark ? 'dark' : 'light'
  }
  return mode
}
