import { SectionHeader } from '../section-header.js';
import { Image as ImageIcon } from 'lucide-react';
import type { PenNode } from '@zseven-w/pen-types';

type ImageFitMode = 'fill' | 'fit' | 'crop' | 'tile';
type ImageNode = PenNode & {
  src?: string;
  objectFit?: ImageFitMode;
  imageSearchQuery?: string;
  imagePrompt?: string;
  width?: number | string;
  height?: number | string;
  exposure?: number;
  contrast?: number;
  saturation?: number;
  temperature?: number;
  tint?: number;
  highlights?: number;
  shadows?: number;
};

interface ImageSectionProps {
  node: ImageNode;
  onUpdate: (updates: Partial<ImageNode>) => void;
}

const FIT_MODE_OPTIONS: { value: ImageFitMode; label: string }[] = [
  { value: 'fill', label: 'Fill' },
  { value: 'fit', label: 'Fit' },
  { value: 'crop', label: 'Crop' },
  { value: 'tile', label: 'Tile' },
];

export function ImageSection({ node, onUpdate }: ImageSectionProps) {
  const fitMode = node.objectFit ?? 'fill';
  const hasImage = node.src && !node.src.startsWith('__');

  return (
    <div className="space-y-1.5">
      <SectionHeader title="Image" />

      <div className="flex items-center gap-2 h-8 px-1.5 rounded border border-border bg-secondary/30">
        <div className="w-6 h-6 rounded border border-border shrink-0 bg-muted overflow-hidden flex items-center justify-center">
          {hasImage ? (
            <img src={node.src} alt="" className="w-full h-full object-cover" />
          ) : (
            <ImageIcon className="w-3 h-3 text-muted-foreground" />
          )}
        </div>
        <select
          value={fitMode}
          onChange={(e) => onUpdate({ objectFit: e.target.value as ImageFitMode })}
          className="flex-1 bg-transparent text-[11px] text-foreground focus:outline-none cursor-pointer"
        >
          {FIT_MODE_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}
