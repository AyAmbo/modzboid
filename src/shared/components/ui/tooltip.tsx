import * as React from "react";
import { cn } from "../../lib/utils";

interface TooltipProps {
  children: React.ReactNode;
  content: React.ReactNode;
  className?: string;
}

function Tooltip({ children, content, className }: TooltipProps) {
  return (
    <div className="relative group inline-flex">
      {children}
      <div
        className={cn(
          "absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-2 py-1 text-xs rounded bg-card border border-border text-foreground whitespace-nowrap opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none z-50",
          className
        )}
      >
        {content}
      </div>
    </div>
  );
}

export { Tooltip };
