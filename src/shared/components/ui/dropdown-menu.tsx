import * as React from "react";
import { cn } from "../../lib/utils";

interface DropdownMenuProps {
  children: React.ReactNode;
}

interface DropdownMenuTriggerProps {
  children: React.ReactNode;
  asChild?: boolean;
}

interface DropdownMenuContentProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
  align?: "start" | "center" | "end";
}

interface DropdownMenuItemProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  children: React.ReactNode;
  inset?: boolean;
}

const DropdownMenuContext = React.createContext<{
  open: boolean;
  setOpen: (v: boolean) => void;
}>({ open: false, setOpen: () => {} });

function DropdownMenu({ children }: DropdownMenuProps) {
  const [open, setOpen] = React.useState(false);
  return (
    <DropdownMenuContext.Provider value={{ open, setOpen }}>
      <div className="relative inline-block">{children}</div>
    </DropdownMenuContext.Provider>
  );
}

function DropdownMenuTrigger({ children }: DropdownMenuTriggerProps) {
  const { setOpen, open } = React.useContext(DropdownMenuContext);
  return (
    <div onClick={() => setOpen(!open)} className="cursor-pointer">
      {children}
    </div>
  );
}

function DropdownMenuContent({ className, children, align = "start", ...props }: DropdownMenuContentProps) {
  const { open, setOpen } = React.useContext(DropdownMenuContext);

  React.useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      // Don't close if clicking inside the dropdown content
      const target = e.target as HTMLElement;
      if (target.closest?.("[data-dropdown-content]")) return;
      setOpen(false);
    };
    // Use timeout so the current click cycle completes before we start listening
    const id = setTimeout(() => {
      document.addEventListener("click", handler);
    }, 0);
    return () => {
      clearTimeout(id);
      document.removeEventListener("click", handler);
    };
  }, [open, setOpen]);

  if (!open) return null;

  const alignClasses = {
    start: "left-0",
    center: "left-1/2 -translate-x-1/2",
    end: "right-0",
  };

  return (
    <div
      data-dropdown-content
      className={cn(
        "absolute top-full mt-1 z-50 min-w-[8rem] bg-card border border-border rounded-md shadow-lg py-1",
        alignClasses[align],
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
}

function DropdownMenuItem({ className, children, inset, onClick, ...props }: DropdownMenuItemProps) {
  const { setOpen } = React.useContext(DropdownMenuContext);
  return (
    <button
      className={cn(
        "w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-muted focus:bg-muted outline-none cursor-pointer",
        inset && "pl-8",
        className
      )}
      onClick={(e) => {
        e.stopPropagation();
        onClick?.(e);
        setOpen(false);
      }}
      {...props}
    >
      {children}
    </button>
  );
}

function DropdownMenuSeparator({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn("h-px bg-border my-1", className)}
      {...props}
    />
  );
}

function DropdownMenuLabel({ className, inset, ...props }: React.HTMLAttributes<HTMLDivElement> & { inset?: boolean }) {
  return (
    <div
      className={cn(
        "px-3 py-1.5 text-xs font-semibold text-muted-foreground",
        inset && "pl-8",
        className
      )}
      {...props}
    />
  );
}

export {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuLabel,
};
