import { useState, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import type { ImageNode, ImageFitMode } from '@/types/pen';
import SectionHeader from '@/components/shared/section-header';
import { Image as ImageIcon, Search, Sparkles } from 'lucide-react';
import ImageFillPopover from './image-fill-popover';
import ImageSearchPopover from './image-search-popover';
import ImageGeneratePopover from './image-generate-popover';
import { Button } from '@/components/ui/button';
import { useDocumentStore } from '@/stores/document-store';
import { toStoredAssetPath } from '@/utils/document-assets';
import LocalImageWarning from './local-image-warning';
import { useImageAssetState } from './use-image-asset-state';

interface ImageSectionProps {
  node: ImageNode;
  onUpdate: (updates: Partial<ImageNode>) => void;
}

export default function ImageSection({ node, onUpdate }: ImageSectionProps) {
  const { t } = useTranslation();
  const documentPath = useDocumentStore((s) => s.filePath);
  const [triggerRect, setTriggerRect] = useState<DOMRect | null>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const fitMode = node.objectFit ?? 'fill';
  const { previewSrc, hasImage, warning } = useImageAssetState(node.src, documentPath);

  const handleClose = useCallback(() => setTriggerRect(null), []);

  const handleToggle = () => {
    if (triggerRect) {
      setTriggerRect(null);
    } else if (triggerRef.current) {
      setTriggerRect(triggerRef.current.getBoundingClientRect());
    }
  };

  const handleRelink = useCallback(async () => {
    if (!window.electronAPI?.openImageFile) return;
    const result = await window.electronAPI.openImageFile();
    if (!result) return;
    onUpdate({
      src: toStoredAssetPath(result.filePath, documentPath),
    });
  }, [documentPath, onUpdate]);

  return (
    <div className="space-y-1.5">
      <SectionHeader title={t('image.title')} />

      <button
        ref={triggerRef}
        type="button"
        onClick={handleToggle}
        className="w-full flex items-center gap-2 h-8 px-1.5 rounded border border-border hover:bg-accent/50 transition-colors cursor-pointer"
      >
        <div className="w-6 h-6 rounded border border-border shrink-0 bg-muted overflow-hidden flex items-center justify-center">
          {hasImage ? (
            <img src={previewSrc} alt="" className="w-full h-full object-cover" />
          ) : (
            <ImageIcon className="w-3 h-3 text-muted-foreground" />
          )}
        </div>
        <span className="text-[11px] text-foreground flex-1 text-left truncate">
          {t(`image.${fitMode === 'fit' ? 'fitMode' : fitMode}`)}
        </span>
      </button>

      {warning && (
        <LocalImageWarning
          message={warning.message}
          assetPath={warning.assetPath}
          onRelink={window.electronAPI?.openImageFile ? handleRelink : undefined}
        />
      )}

      <div className="flex gap-1 mt-1.5">
        <ImageSearchPopover
          initialQuery={node.imageSearchQuery ?? node.name ?? ''}
          onSelect={(url: string) => onUpdate({ src: url })}
        >
          <Button size="sm" variant="outline" className="flex-1 h-7 text-xs">
            <Search className="h-3 w-3 mr-1" />
            Search
          </Button>
        </ImageSearchPopover>

        <ImageGeneratePopover
          initialPrompt={node.imagePrompt ?? node.name ?? ''}
          onGenerated={(url: string) => onUpdate({ src: url })}
          width={typeof node.width === 'number' ? node.width : undefined}
          height={typeof node.height === 'number' ? node.height : undefined}
        >
          <Button size="sm" variant="outline" className="flex-1 h-7 text-xs">
            <Sparkles className="h-3 w-3 mr-1" />
            Generate
          </Button>
        </ImageGeneratePopover>
      </div>

      {triggerRect && (
        <ImageFillPopover
          imageSrc={previewSrc}
          fitMode={fitMode}
          documentPath={documentPath}
          triggerRect={triggerRect}
          adjustments={{
            exposure: node.exposure,
            contrast: node.contrast,
            saturation: node.saturation,
            temperature: node.temperature,
            tint: node.tint,
            highlights: node.highlights,
            shadows: node.shadows,
          }}
          onFitModeChange={(mode) => onUpdate({ objectFit: mode as ImageFitMode })}
          onAdjustmentChange={(key, value) => onUpdate({ [key]: value } as Partial<ImageNode>)}
          onResetAdjustments={() =>
            onUpdate({
              exposure: 0,
              contrast: 0,
              saturation: 0,
              temperature: 0,
              tint: 0,
              highlights: 0,
              shadows: 0,
            } as Partial<ImageNode>)
          }
          onImageChange={(dataUrl) => onUpdate({ src: dataUrl })}
          onClose={handleClose}
        />
      )}
    </div>
  );
}
