export function Logo({ size = 32 }: { size?: number }) {
  return (
    <svg className="logo" width={size} height={size} viewBox="0 0 44 44" fill="none" aria-hidden="true">
      <path
        d="M3 7.5C3 6.1 4.1 5 5.5 5H11.8C12.8 5 13.7 5.6 14.1 6.5L24 27.4L19.6 37L3.3 8.7C3.1 8.33 3 7.92 3 7.5Z"
        fill="url(#vsn-a)"
      />
      <path
        d="M41 7.5C41 6.1 39.9 5 38.5 5H32.2C31.2 5 30.3 5.6 29.9 6.5L19.6 37C20.4 38.6 21.9 39 23.2 39C24.5 39 25.8 38.4 26.5 37L40.7 8.7C40.9 8.3 41 7.9 41 7.5Z"
        fill="url(#vsn-b)"
      />
      <circle cx="36" cy="33" r="4" fill="#5AC8D6" />
      <defs>
        <linearGradient id="vsn-a" x1="4" y1="6" x2="22" y2="38" gradientUnits="userSpaceOnUse">
          <stop stopColor="#5AC8D6" />
          <stop offset="1" stopColor="#6188C6" />
        </linearGradient>
        <linearGradient id="vsn-b" x1="22" y1="38" x2="40" y2="6" gradientUnits="userSpaceOnUse">
          <stop stopColor="#6188C6" />
          <stop offset="1" stopColor="#625BA8" />
        </linearGradient>
      </defs>
    </svg>
  );
}
