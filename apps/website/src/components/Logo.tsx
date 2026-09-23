export const AmberLogo = ({ className = 'w-8 h-8' }: { className?: string }) => (
  <svg
    viewBox="0 0 32 32"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    className={`${className} shrink-0 select-none`}
    aria-label="Amber Logo"
  >
    <defs>
      <linearGradient id="amber-grad" x1="4" y1="4" x2="28" y2="28" gradientUnits="userSpaceOnUse">
        <stop offset="0%" stopColor="#FDE047" />
        <stop offset="40%" stopColor="#F59E0B" />
        <stop offset="100%" stopColor="#D97706" />
      </linearGradient>
      <linearGradient id="amber-facets" x1="16" y1="2" x2="16" y2="30" gradientUnits="userSpaceOnUse">
        <stop offset="0%" stopColor="#FFFBEB" stopOpacity="0.85" />
        <stop offset="100%" stopColor="#92400E" stopOpacity="0.4" />
      </linearGradient>
    </defs>
    {/* Amber crystal cut */}
    <polygon
      points="10,2 22,2 30,10 30,22 22,30 10,30 2,22 2,10"
      fill="url(#amber-grad)"
      stroke="#B45309"
      strokeWidth="1.2"
    />
    {/* Inner facets */}
    <polygon
      points="12,7 20,7 25,12 25,20 20,25 12,25 7,20 7,12"
      fill="none"
      stroke="url(#amber-facets)"
      strokeWidth="1"
    />
    <polygon
      points="14,11 18,11 21,14 21,18 18,21 14,21 11,18 11,14"
      fill="#FEF3C7"
      fillOpacity="0.35"
      stroke="#FEF3C7"
      strokeWidth="0.8"
    />
  </svg>
)
