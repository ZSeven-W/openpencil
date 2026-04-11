import { useEffect, useMemo, useState } from 'react';
import { resolveRuntimeAssetSource } from '@/utils/document-assets';

interface ImageAssetWarning {
  message: string;
  assetPath: string;
}

export function useImageAssetState(
  assetPath: string | null | undefined,
  documentPath: string | null | undefined,
) {
  const [previewLoadFailed, setPreviewLoadFailed] = useState(false);

  const runtimeSource = useMemo(
    () => resolveRuntimeAssetSource(assetPath, documentPath),
    [assetPath, documentPath],
  );

  const previewSrc = runtimeSource.runtimeUrl ?? undefined;

  useEffect(() => {
    if (!previewSrc) {
      setPreviewLoadFailed(runtimeSource.unresolved);
      return;
    }

    let disposed = false;
    const img = new Image();
    img.onload = () => {
      if (!disposed) setPreviewLoadFailed(false);
    };
    img.onerror = () => {
      if (!disposed) setPreviewLoadFailed(true);
    };
    img.src = previewSrc;

    return () => {
      disposed = true;
    };
  }, [previewSrc, runtimeSource.unresolved]);

  const hasImage =
    !!assetPath && !assetPath.startsWith('__') && !runtimeSource.unresolved && !previewLoadFailed;

  const warning =
    runtimeSource.isLocal && assetPath && (runtimeSource.unresolved || previewLoadFailed)
      ? ({
          message: runtimeSource.unresolved
            ? 'Relative image path cannot be resolved yet'
            : 'Image file is missing',
          assetPath,
        } satisfies ImageAssetWarning)
      : null;

  return {
    runtimeSource,
    previewSrc,
    hasImage,
    warning,
  };
}
