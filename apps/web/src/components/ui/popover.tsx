import * as React from 'react';
import * as PopoverPrimitive from '@radix-ui/react-popover';

import { cn } from '@/lib/utils';

const Popover = PopoverPrimitive.Root;

const PopoverTrigger = PopoverPrimitive.Trigger;

const PopoverAnchor = PopoverPrimitive.Anchor;

interface PopoverContentProps extends React.ComponentPropsWithoutRef<
  typeof PopoverPrimitive.Content
> {
  /**
   * Render a small arrow pointing at the anchor. Defaults to `true` — pass
   * `false` for popovers where the arrow would collide with the layout
   * (e.g. when the content is nearly flush with its trigger or overflowing).
   */
  arrow?: boolean;
}

const PopoverContent = React.forwardRef<
  React.ComponentRef<typeof PopoverPrimitive.Content>,
  PopoverContentProps
>(({ className, align = 'center', sideOffset = 8, arrow = true, children, ...props }, ref) => (
  <PopoverPrimitive.Portal>
    <PopoverPrimitive.Content
      ref={ref}
      align={align}
      sideOffset={sideOffset}
      className={cn(
        'group relative z-50 w-72 rounded-md border border-border bg-popover p-4 text-popover-foreground shadow-md outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2',
        className,
      )}
      {...props}
    >
      {children}
      {arrow && (
        <span
          aria-hidden
          className="pointer-events-none absolute left-1/2 hidden -translate-x-1/2 translate-y-px group-data-[side=bottom]:bottom-full group-data-[side=bottom]:block"
        >
          <svg width="13" height="7" viewBox="0 0 13 7" className="block">
            <path d="M0 7 L6.5 0 L13 7 Z" className="fill-popover" />
            <path
              d="M0 7 L6.5 0 L13 7"
              className="fill-none stroke-border"
              strokeWidth={1}
              strokeLinejoin="round"
            />
          </svg>
        </span>
      )}
    </PopoverPrimitive.Content>
  </PopoverPrimitive.Portal>
));
PopoverContent.displayName = PopoverPrimitive.Content.displayName;

export { Popover, PopoverTrigger, PopoverContent, PopoverAnchor };
