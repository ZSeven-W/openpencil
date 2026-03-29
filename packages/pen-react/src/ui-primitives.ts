// Re-export Radix primitives that components need
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

// cn() utility — pen-react owns its own copy (2 lines, no import from @/lib/utils)
import { twMerge } from 'tailwind-merge';
import { clsx, type ClassValue } from 'clsx';
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
