// The circle stays low-left. Do not recenter it. Fills swap in CSS for dark paper.
export const AmberLogo = ({ className = "h-8 w-8" }: { className?: string }) => (
  <svg
    viewBox="20 20 216 216"
    xmlns="http://www.w3.org/2000/svg"
    className={`amber-mark shrink-0 select-none ${className}`}
    aria-hidden="true"
  >
    <path className="field" d="M 20 20 H 236 V 236 H 20 Z" />
    <path
      className="dot"
      d="M 110 180 A 34 34 0 1 0 42 180 A 34 34 0 1 0 110 180 Z"
    />
  </svg>
);
