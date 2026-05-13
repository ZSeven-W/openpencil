import { useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { CodegenAssetFile } from '@/services/ai/codegen-assets';
import type { CodegenAssetManifestEntry } from '@/types/cloud';

interface CodeAssetPreviewStripProps {
  assets: CodegenAssetFile[];
  assetsManifest?: CodegenAssetManifestEntry[];
}

function bytesToObjectUrl(asset: CodegenAssetFile): string {
  if (typeof URL.createObjectURL !== 'function') return '';
  const bytes = Uint8Array.from(asset.bytes);
  return URL.createObjectURL(new Blob([bytes], { type: asset.mimeType }));
}

export default function CodeAssetPreviewStrip({
  assets,
  assetsManifest = [],
}: CodeAssetPreviewStripProps) {
  const { t } = useTranslation();
  const objectUrls = useMemo(
    () => assets.map((asset) => ({ asset, url: bytesToObjectUrl(asset) })),
    [assets],
  );
  const cloudAssets = assetsManifest.filter((asset) => asset.signedUrl);
  const count = objectUrls.length + cloudAssets.length;

  useEffect(() => {
    return () => {
      if (typeof URL.revokeObjectURL !== 'function') return;
      for (const { url } of objectUrls) {
        if (url) URL.revokeObjectURL(url);
      }
    };
  }, [objectUrls]);

  if (count === 0) return null;

  return (
    <div className="shrink-0 border-b border-border/50 bg-muted/20 px-2 py-2">
      <div className="mb-1.5 flex items-center justify-between text-[10px] font-medium uppercase tracking-[0.08em] text-muted-foreground">
        <span>{t('codePanel.assets')}</span>
        <span>{count}</span>
      </div>
      <div className="flex gap-1.5 overflow-x-auto">
        {objectUrls.map(({ asset, url }) => {
          return (
            <div
              key={asset.id}
              className="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded border border-border/60 bg-background"
              title={asset.relativePath}
            >
              {url ? (
                <img
                  src={url}
                  alt={asset.sourceNodeName ?? asset.relativePath}
                  className="h-full w-full object-cover"
                />
              ) : (
                <span className="px-1 text-center text-[9px] text-muted-foreground">IMG</span>
              )}
            </div>
          );
        })}
        {cloudAssets.map((asset) => (
          <img
            key={asset.id}
            src={asset.signedUrl}
            alt={asset.sourceNodeName ?? asset.relativePath}
            className="h-12 w-12 shrink-0 rounded border border-border/60 bg-background object-cover"
          />
        ))}
      </div>
    </div>
  );
}
