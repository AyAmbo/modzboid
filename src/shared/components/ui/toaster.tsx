import { create } from "zustand";
import { cn } from "../../lib/utils";

interface Toast {
  id: string;
  title?: string;
  description?: string;
  variant?: "default" | "destructive";
}

interface ToastStore {
  toasts: Toast[];
  toast: (t: Omit<Toast, "id">) => void;
  dismiss: (id: string) => void;
}

const MAX_VISIBLE = 3;

export const useToastStore = create<ToastStore>((set, get) => ({
  toasts: [],
  toast: (t) => {
    const { toasts } = get();
    // Deduplicate: skip if an identical message is already showing
    const key = `${t.title ?? ""}|${t.description ?? ""}`;
    if (toasts.some((x) => `${x.title ?? ""}|${x.description ?? ""}` === key)) {
      return;
    }
    const id = Math.random().toString(36).slice(2);
    set((state) => ({
      // Keep only the newest MAX_VISIBLE toasts
      toasts: [...state.toasts, { ...t, id }].slice(-MAX_VISIBLE),
    }));
    setTimeout(() => {
      set((state) => ({ toasts: state.toasts.filter((x) => x.id !== id) }));
    }, 4000);
  },
  dismiss: (id) => {
    set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) }));
  },
}));

/** Call from anywhere (hooks, stores, event handlers) */
export function toast(t: Omit<Toast, "id">) {
  useToastStore.getState().toast(t);
}

/** For React components that prefer the hook pattern */
export function useToast() {
  return useToastStore();
}

export function Toaster() {
  const { toasts, dismiss } = useToastStore();

  if (toasts.length === 0) return null;

  return (
    <div data-testid="toast" className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
      {toasts.map((t) => (
        <div
          key={t.id}
          className={cn(
            "pointer-events-auto",
            "flex items-start gap-3 rounded-lg border px-4 py-3 shadow-lg min-w-[300px] max-w-[420px] bg-card text-foreground border-border",
            t.variant === "destructive" && "border-destructive bg-destructive text-white"
          )}
        >
          <div className="flex-1">
            {t.title && <p className="text-sm font-semibold">{t.title}</p>}
            {t.description && (
              <p className="text-xs text-muted-foreground mt-0.5">{t.description}</p>
            )}
          </div>
          <button
            onClick={() => dismiss(t.id)}
            className="text-muted-foreground hover:text-foreground text-xs mt-0.5"
          >
            ✕
          </button>
        </div>
      ))}
    </div>
  );
}
