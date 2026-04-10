import { AlertTriangle } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface LocalImageWarningProps {
  message: string;
  assetPath: string;
  onRelink?: () => void;
}

export default function LocalImageWarning({
  message,
  assetPath,
  onRelink,
}: LocalImageWarningProps) {
  return (
    <div className="rounded-md border border-orange-500/40 bg-orange-500/10 px-2 py-1.5">
      <div className="flex items-start gap-2">
        <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-orange-400" />
        <div className="min-w-0 flex-1">
          <div className="text-[11px] font-medium text-orange-200">{message}</div>
          <div className="mt-0.5 text-[10px] text-orange-100/80 break-all">{assetPath}</div>
        </div>
        {onRelink && (
          <Button
            size="sm"
            variant="outline"
            className="h-6 shrink-0 border-orange-400/40 bg-transparent px-2 text-[10px] text-orange-100 hover:bg-orange-500/10"
            onClick={() => void onRelink()}
          >
            Relink
          </Button>
        )}
      </div>
    </div>
  );
}
