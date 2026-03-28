import * as React from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "../../../shared/components/ui/dialog";
import { Button } from "../../../shared/components/ui/button";
import { cn } from "../../../shared/lib/utils";
import { toast } from "../../../shared/components/ui/toaster";

type ExportFormat = "json" | "csv" | "text";

interface ExportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  profileId: string | null;
}

export function ExportDialog({
  open,
  onOpenChange,
  profileId,
}: ExportDialogProps) {
  const [format, setFormat] = React.useState<ExportFormat>("json");
  const [content, setContent] = React.useState("");
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const requestIdRef = React.useRef(0);

  const fetchExport = React.useCallback(
    async (fmt: ExportFormat) => {
      if (!profileId) return;
      const requestId = ++requestIdRef.current;
      setLoading(true);
      setError(null);
      try {
        const result = await invoke<string>("export_mod_list_cmd", {
          profileId,
          format: fmt,
        });
        if (requestId !== requestIdRef.current) return; // stale response
        setContent(result);
      } catch (err) {
        if (requestId !== requestIdRef.current) return;
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        setContent("");
      } finally {
        if (requestId === requestIdRef.current) {
          setLoading(false);
        }
      }
    },
    [profileId]
  );

  React.useEffect(() => {
    if (open && profileId) {
      fetchExport(format);
    }
    if (!open) {
      setContent("");
      setError(null);
    }
  }, [open, profileId, format, fetchExport]);

  const handleFormatChange = (fmt: ExportFormat) => {
    setFormat(fmt);
  };

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(content);
      toast({ title: "Copied", description: "Mod list copied to clipboard" });
    } catch {
      toast({
        title: "Error",
        description: "Failed to copy to clipboard",
        variant: "destructive",
      });
    }
  };

  const formats: ExportFormat[] = ["json", "csv", "text"];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-testid="dialog-export" className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Export Mod List</DialogTitle>
          <DialogDescription>
            Export your mod list to share with others.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Format tabs */}
          <div className="flex gap-2">
            {formats.map((fmt) => (
              <button
                key={fmt}
                type="button"
                onClick={() => handleFormatChange(fmt)}
                className={cn(
                  "flex-1 py-2 px-3 rounded-md text-sm font-medium border transition-colors uppercase",
                  format === fmt
                    ? "bg-primary text-primary-foreground border-primary"
                    : "bg-transparent border-border text-muted-foreground hover:bg-muted"
                )}
              >
                {fmt}
              </button>
            ))}
          </div>

          {/* Preview */}
          {error && <p className="text-sm text-destructive">{error}</p>}

          {loading ? (
            <div className="flex items-center justify-center h-[300px] text-muted-foreground text-sm">
              Loading...
            </div>
          ) : (
            <pre className="bg-muted border border-border rounded-md p-3 text-xs text-foreground overflow-auto max-h-[300px] whitespace-pre-wrap break-words">
              {content || "No content to display."}
            </pre>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Close
          </Button>
          <Button onClick={handleCopy} disabled={!content || loading}>
            Copy to Clipboard
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
