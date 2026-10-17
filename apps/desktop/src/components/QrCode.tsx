import { useMemo } from 'react';
import qrcode from '../vendor/qrcode/qrcode.js';

/** Scannable QR code drawn as SVG rects (no innerHTML). Always dark on white for scanners. */
export function QrCode({ value, size = 168, label }: { value: string; size?: number; label: string }) {
  const cells = useMemo(() => {
    const qr = qrcode(0, 'M');
    qr.addData(value);
    qr.make();
    const n = qr.getModuleCount();
    const dark: [number, number][] = [];
    for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) if (qr.isDark(r, c)) dark.push([c, r]);
    return { n, dark };
  }, [value]);
  const quiet = 4;
  const total = cells.n + quiet * 2;
  return (
    <svg
      className="qr"
      role="img"
      aria-label={label}
      width={size}
      height={size}
      viewBox={`0 0 ${total} ${total}`}
      shapeRendering="crispEdges"
    >
      <rect width={total} height={total} fill="#fff" />
      {cells.dark.map(([x, y]) => (
        <rect key={`${x}-${y}`} x={x + quiet} y={y + quiet} width={1} height={1} fill="#000" />
      ))}
    </svg>
  );
}
