import { useState } from 'react';
import { SectionHeader } from '../section-header.js';

const SCALE_OPTIONS = [
  { value: '1', label: '1x' },
  { value: '2', label: '2x' },
  { value: '3', label: '3x' },
];

const FORMAT_OPTIONS = [
  { value: 'png', label: 'PNG' },
  { value: 'jpeg', label: 'JPEG' },
  { value: 'webp', label: 'WEBP' },
];

interface ExportSectionProps {
  nodeId: string;
  nodeName: string;
  /** Optional callback for export action — app-level implementation. */
  onExport?: (options: { nodeId: string; nodeName: string; scale: number; format: string }) => void;
}

export function ExportSection({ nodeId, nodeName, onExport }: ExportSectionProps) {
  const [scale, setScale] = useState('1');
  const [format, setFormat] = useState('png');

  const handleExport = () => {
    onExport?.({ nodeId, nodeName, scale: Number(scale), format });
  };

  return (
    <div className="space-y-1.5">
      <SectionHeader title="Export" />
      <div className="flex gap-1.5">
        <select
          value={scale}
          onChange={(e) => setScale(e.target.value)}
          className="flex-1 h-6 text-[11px] bg-secondary border border-transparent rounded px-1 focus:outline-none focus:border-ring text-foreground"
        >
          {SCALE_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        <select
          value={format}
          onChange={(e) => setFormat(e.target.value)}
          className="flex-1 h-6 text-[11px] bg-secondary border border-transparent rounded px-1 focus:outline-none focus:border-ring text-foreground"
        >
          {FORMAT_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </div>
      <button
        type="button"
        onClick={handleExport}
        className="w-full h-7 text-xs border border-border rounded bg-transparent text-foreground hover:bg-accent/50 transition-colors"
      >
        Export Layer
      </button>
    </div>
  );
}
