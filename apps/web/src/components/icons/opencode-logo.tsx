import type { SVGProps } from 'react';

export default function OpenCodeLogo(props: SVGProps<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg" {...props}>
      <g clipPath="url(#oc-clip)">
        {/* Terminal 窗框 */}
        <rect
          x="12"
          y="14"
          width="40"
          height="36"
          rx="4"
          stroke="currentColor"
          strokeWidth="2.5"
          fill="none"
        />
        {/* Title 酒吧线 */}
        <line x1="12" y1="22" x2="52" y2="22" stroke="currentColor" strokeWidth="2" />
        {/* Title 条形点 */}
        <circle cx="18" cy="18" r="1.5" fill="currentColor" />
        <circle cx="23" cy="18" r="1.5" fill="currentColor" />
        <circle cx="28" cy="18" r="1.5" fill="currentColor" />
        {/* Terminal 提示符 V 形 > */}
        <path
          d="M18 29L26 35L18 41"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          fill="none"
        />
        {/* Cursor 线 */}
        <line
          x1="30"
          y1="40"
          x2="44"
          y2="40"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
        />
      </g>
      <defs>
        <clipPath id="oc-clip">
          <rect width="40" height="40" fill="white" transform="translate(12 12)" />
        </clipPath>
      </defs>
    </svg>
  );
}
