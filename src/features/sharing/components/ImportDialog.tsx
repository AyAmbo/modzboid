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
import { Input } from "../../../shared/components/ui/input";
import { toast } from "../../../shared/components/ui/toaster";
import type { ImportPreview } from "../types";

interface ImportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImported: () => void;
}

export function ImportDialog({
  open,
  onOpenChange,
  onImported,
}: ImportDialogProps) {
  const [content, setContent] = React.useState("");
  const [profileName, setProfileName] = React.useState("");
  const [preview, setPreview] = React.useState<ImportPreview | null>(null);
  const [parsing, setParsing] = React.useState(false);
  const [importing, setImporting] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const debounceRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);

  // Reset state when dialog closes
  React.useEffect(() => {
    if (!open) {
      setContent("");
      setProfileName("");
      setPreview(null);
      setError(null);
      setParsing(false);
    }
  }, [open]);

  // Debounced parse on content change
  React.useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    if (!content.trim()) {
      setPreview(null);
      setError(null);
      return;
    }

    setParsing(true);
    debounceRef.current = setTimeout(async () => {
      try {
        const result = await invoke<ImportPreview>(
          "parse_mod_list_import_cmd",
          { content }
        );
        setPreview(result);
        setError(null);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        setPreview(null);
      } finally {
        setParsing(false);
      }
    }, 500);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [content]);

  const handleImport = async () => {
    if (!preview || !profileName.trim()) return;

    setImporting(true);
    try {
      await invoke("apply_mod_list_import_cmd", {
        profileName: profileName.trim(),
        modIds: preview.found,
      });
      toast({
        title: "Imported",
        description: `Created profile "${profileName.trim()}" with ${preview.found.length} mods`,
      });
      onImported();
      onOpenChange(false);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      toast({
        title: "Error",
        description: msg,
        variant: "destructive",
      });
    } finally {
      setImporting(false);
    }
  };

  const canImport =
    preview && preview.found.length > 0 && profileName.trim().length > 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-testid="dialog-import" className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Import Mod List</DialogTitle>
          <DialogDescription>
            Paste a mod list (JSON, CSV, or plain text) to import it as a new
            profile.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Paste area */}
          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">
              Mod List Content
            </label>
            <textarea
              className="flex w-full rounded-md border border-border bg-muted px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary resize-none"
              rows={8}
              placeholder="Paste your mod list here (JSON, CSV, or plain text)..."
              value={content}
              onChange={(e) => setContent(e.target.value)}
            />
          </div>

          {/* Preview */}
          {parsing && (
            <p className="text-sm text-muted-foreground">Parsing...</p>
          )}

          {error && <p className="text-sm text-destructive">{error}</p>}

          {preview && !parsing && (
            <div className="space-y-2 rounded-md border border-border bg-muted p-3">
              <p className="text-sm font-medium text-foreground">
                Detected format:{" "}
                <span className="text-muted-foreground">
                  {preview.detectedFormat}
                </span>
              </p>

              <p className="text-sm text-green-500">
                Found: {preview.found.length} mod
                {preview.found.length !== 1 ? "s" : ""}
              </p>

              {preview.missing.length > 0 && (
                <div>
                  <p className="text-sm text-orange-400">
                    Missing: {preview.missing.length} mod
                    {preview.missing.length !== 1 ? "s" : ""}
                  </p>
                  <ul className="mt-1 max-h-[120px] overflow-auto text-xs text-muted-foreground space-y-0.5">
                    {preview.missing.map((mod) => (
                      <li key={mod.id}>
                        {mod.name ? `${mod.name} (${mod.id})` : mod.id}
                        {mod.workshopId && ` [Workshop: ${mod.workshopId}]`}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}

          {/* Profile name */}
          {preview && preview.found.length > 0 && (
            <div className="space-y-1.5">
              <label className="text-sm font-medium text-foreground">
                Profile Name
              </label>
              <Input
                placeholder="Imported Profile"
                value={profileName}
                onChange={(e) => setProfileName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && canImport) handleImport();
                }}
              />
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleImport} disabled={!canImport || importing}>
            {importing ? "Importing..." : "Import"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
