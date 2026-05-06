// Re-导出组件需要的 Radix 原语
export { Tooltip, TooltipTrigger, TooltipContent, TooltipProvider } from '@radix-ui/react-tooltip';
export { Popover, PopoverTrigger, PopoverContent } from '@radix-ui/react-popover';
export { Separator } from '@radix-ui/react-separator';
export {
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
} from '@radix-ui/react-select';
export { Slider } from '@radix-ui/react-slider';
export { Switch } from '@radix-ui/react-switch';
export { Toggle } from '@radix-ui/react-toggle';

// cn() 实用程序 — pen-react 拥有自己的副本（2 行，不从 @/lib/utils 导入）
import { twMerge } from 'tailwind-merge';
import { clsx, type ClassValue } from 'clsx';
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
