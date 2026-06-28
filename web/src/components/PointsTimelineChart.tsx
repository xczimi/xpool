import { useLayoutEffect, useRef, useState } from 'react'
import type { TimelineSeries } from '../lib/timeline'
import { pickTickIndices } from '../lib/timeline'

export type { TimelineSeries }

// The viewBox WIDTH tracks the container's pixel width (measured below) so the
// chart fills its column with no distortion: rendered 1:1, the aspect ratio
// always matches, so the svg height stays pinned at H (~200px) at every width
// instead of ballooning on wide screens (the `meet` + height:auto trap).
const DEFAULT_W = 480
const MIN_W = 320
const H = 200
const PAD_L = 36
const PAD_R = 12
const PAD_T = 12
const PAD_B = 28

/** Short date label for an x tick (kickoff day), localised, clock-free. */
function tickLabel(iso: string, locale: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return ''
  return d.toLocaleDateString(locale, { month: 'short', day: 'numeric' })
}

/**
 * Hand-rolled SVG line chart — no charting library, mirroring BrandIcon's
 * pure-SVG approach. The x-axis is GAME BY GAME (chronological game order); one
 * <polyline> per series so it overlays N players. x ticks are sparse, labelled
 * by each game's kickoff date. Every input is pre-computed data (no Date.now),
 * so the component is pure.
 */
export function PointsTimelineChart({
  series,
  locale,
  title,
  emptyLabel,
}: {
  series: TimelineSeries[]
  locale: string
  title?: string
  emptyLabel?: string
}) {
  // Measure the container width and feed it back as the viewBox width so the
  // plot stretches to fill the column while the height stays bounded at H.
  const ref = useRef<HTMLElement>(null)
  const [W, setW] = useState(DEFAULT_W)
  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    const measure = () => {
      const cw = el.clientWidth
      if (cw > 0) setW(Math.max(MIN_W, Math.round(cw)))
    }
    measure()
    if (typeof ResizeObserver === 'undefined') return
    const ro = new ResizeObserver(measure)
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  // All series share the same game x-axis; take the longest to be safe.
  const axis = series.reduce(
    (longest, s) => (s.points.length > longest.length ? s.points : longest),
    [] as TimelineSeries['points'],
  )
  const n = axis.length
  const maxY = Math.max(
    1,
    ...series.flatMap((s) => s.points.map((p) => p.cumulative)),
  )
  const innerW = W - PAD_L - PAD_R
  const innerH = H - PAD_T - PAD_B
  const x = (i: number) => PAD_L + (n <= 1 ? innerW / 2 : (innerW * i) / (n - 1))
  const y = (v: number) => PAD_T + innerH - (innerH * v) / maxY
  const ticks = pickTickIndices(n)

  return (
    <figure className="points-timeline" ref={ref}>
      {title && <figcaption>{title}</figcaption>}
      {n === 0 ? (
        <p className="pt-empty">{emptyLabel ?? ''}</p>
      ) : (
        <svg
          viewBox={`0 0 ${W} ${H}`}
          role="img"
          aria-label={title ?? 'points timeline'}
          preserveAspectRatio="xMidYMid meet"
        >
          <line
            className="pt-axis"
            x1={PAD_L}
            y1={PAD_T + innerH}
            x2={W - PAD_R}
            y2={PAD_T + innerH}
          />
          {series.map((s) => (
            <polyline
              key={s.label}
              className="pt-line"
              fill="none"
              stroke={s.color}
              points={s.points
                .map((p, i) => `${x(i)},${y(p.cumulative)}`)
                .join(' ')}
            />
          ))}
          {/* Markers only when sparse enough to read (single-line / short axis). */}
          {n <= 1 &&
            series.flatMap((s) =>
              s.points.map((p, i) => (
                <circle
                  key={`${s.label}-${i}`}
                  cx={x(i)}
                  cy={y(p.cumulative)}
                  r={2.5}
                  fill={s.color}
                />
              )),
            )}
          {ticks.map((i) => (
            <text
              key={`tick-${i}`}
              className="pt-xlabel"
              x={x(i)}
              y={H - 8}
              textAnchor="middle"
            >
              {tickLabel(axis[i].kickoff, locale)}
            </text>
          ))}
        </svg>
      )}
      {series.length > 1 && (
        <ul className="pt-legend">
          {series.map((s) => (
            <li key={s.label} className="pt-legend-item">
              <span
                className="pt-swatch"
                style={{ background: s.color }}
                aria-hidden="true"
              />
              {s.label}
            </li>
          ))}
        </ul>
      )}
    </figure>
  )
}
