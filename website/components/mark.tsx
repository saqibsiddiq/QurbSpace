import { useId } from 'react'

/**
 * The mark (decision 0091): two circles almost touching, one white and one
 * Qurb light. The same drawing as the window’s
 * `#qurb-mark`, packaging/qurb.svg and the Android launcher icon.
 */
export function Mark({ size = 28, className }: { size?: number; className?: string }) {
  // Each mark on a page needs its own gradient id, or the second one would
  // borrow the first’s, and lose it if the first is hidden.
  const tile = `qurb-tile-${useId().replace(/:/g, '')}`
  return (
    <svg className={className} width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
      <defs>
        <linearGradient id={tile} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#3c806a" />
          <stop offset="1" stopColor="#22503f" />
        </linearGradient>
      </defs>
      <rect x="1" y="1" width="30" height="30" rx="9" fill={`url(#${tile})`} />
      <circle cx="10.73" cy="16" r="4.92" fill="#fff" />
      <circle cx="21.27" cy="16" r="4.92" fill="#bde7d6" />
    </svg>
  )
}
