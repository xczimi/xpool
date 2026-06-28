import type { CumulativePoint } from '../lib/cumulativePoints'

/** One overlaid line: a label, a stroke colour, and its cumulative points. */
export interface TimelineSeries {
  label: string
  color: string
  points: CumulativePoint[]
}

const W = 480
const H = 200
const PAD_L = 36
const PAD_R = 12
const PAD_T = 12
const PAD_B = 28

/**
 * Hand-rolled SVG line chart — no charting library, mirroring BrandIcon's
 * pure-SVG approach. Plots cumulative points per round, one <polyline> per
 * series, so it supports head-to-head overlay. Every input is pre-computed and
 * already localised (`xLabels`), so the component is pure and clock-free.
 */
export function PointsTimelineChart({
  series,
  xLabels,
  title,
}: {
  series: TimelineSeries[]
  xLabels: string[]
  title?: string
}) {
  const n = xLabels.length
  const maxY = Math.max(
    1,
    ...series.flatMap((s) => s.points.map((p) => p.cumulative)),
  )
  const innerW = W - PAD_L - PAD_R
  const innerH = H - PAD_T - PAD_B
  const x = (i: number) => PAD_L + (n <= 1 ? 0 : (innerW * i) / (n - 1))
  const y = (v: number) => PAD_T + innerH - (innerH * v) / maxY

  return (
    <figure className="points-timeline">
      {title && <figcaption>{title}</figcaption>}
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
            points={s.points.map((p, i) => `${x(i)},${y(p.cumulative)}`).join(' ')}
          />
        ))}
        {series.flatMap((s) =>
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
        {xLabels.map((lbl, i) => (
          <text
            key={`${lbl}-${i}`}
            className="pt-xlabel"
            x={x(i)}
            y={H - 8}
            textAnchor="middle"
          >
            {lbl}
          </text>
        ))}
      </svg>
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
