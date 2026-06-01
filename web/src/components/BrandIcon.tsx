/**
 * xPool brand badge — LED-X scoreboard logo (Variant C).
 *
 * 13 amber LED dots arranged as an X on a near-black dot-matrix disc, with a
 * single green "match live" status indicator. Renders crisply at all sizes
 * (favicon → header → OG image) because every primitive is a vector circle.
 *
 * Pure SVG, no props beyond an optional className for sizing — keeps the
 * component cacheable and trivially inlinable.
 */
export function BrandIcon({ className = 'brand-icon' }: { className?: string }) {
  return (
    <svg
      className={className}
      viewBox="0 0 48 48"
      xmlns="http://www.w3.org/2000/svg"
      shapeRendering="crispEdges"
      aria-label="xPool"
      role="img"
    >
      <defs>
        <clipPath id="xpool-disc">
          <circle cx="24" cy="24" r="22" />
        </clipPath>
        <radialGradient id="xpool-led" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0%" stopColor="#ffd76a" />
          <stop offset="55%" stopColor="#ff8c00" />
          <stop offset="100%" stopColor="#a04400" />
        </radialGradient>
        <filter id="xpool-glow" x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur stdDeviation="0.8" />
        </filter>
      </defs>

      {/* near-black scoreboard disc */}
      <circle cx="24" cy="24" r="22" fill="#0a0a14" />

      {/* faint amber dot-matrix grid (unlit pixels) */}
      <g clipPath="url(#xpool-disc)" fill="#2a1a08">
        {[6, 12, 18, 24, 30, 36, 42].flatMap((y) =>
          [6, 12, 18, 24, 30, 36, 42].map((x) => (
            <rect key={`${x}-${y}`} x={x} y={y} width="1.5" height="1.5" />
          )),
        )}
      </g>

      {/* faint amber rim (scoreboard bezel) */}
      <circle cx="24" cy="24" r="22" fill="none" stroke="#a04400" strokeWidth="1.2" />

      {/* the X — 13 glowing amber LEDs, brightest at the center */}
      <g fill="url(#xpool-led)" filter="url(#xpool-glow)">
        <circle cx="13" cy="13" r="2.0" />
        <circle cx="17" cy="17" r="2.0" />
        <circle cx="21" cy="21" r="2.2" />
        <circle cx="24" cy="24" r="2.6" />
        <circle cx="27" cy="27" r="2.2" />
        <circle cx="31" cy="31" r="2.0" />
        <circle cx="35" cy="35" r="2.0" />
        <circle cx="35" cy="13" r="2.0" />
        <circle cx="31" cy="17" r="2.0" />
        <circle cx="27" cy="21" r="2.2" />
        <circle cx="21" cy="27" r="2.2" />
        <circle cx="17" cy="31" r="2.0" />
        <circle cx="13" cy="35" r="2.0" />
      </g>

      {/* "match live" status indicator */}
      <circle cx="24" cy="40" r="1.4" fill="#33ff66" />
    </svg>
  )
}
