import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Button } from "../../../shared/components/ui/button";
import { ConfirmDialog } from "../../../shared/components/ui/confirm-dialog";
import { toast } from "../../../shared/components/ui/toaster";
import type { ExtensionInfo } from "../types";

export default function ExtensionsPage() {
  const [extensions, setExtensions] = useState<ExtensionInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [installing, setInstalling] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  const loadExtensions = useCallback(async () => {
    try {
      const list = await invoke<ExtensionInfo[]>("list_extensions_cmd");
      setExtensions(list);
    } catch (err) {
      console.error("Failed to load extensions:", err);
      toast({ title: "Error", description: "Failed to load extensions", variant: "destructive" });
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadExtensions();
  }, [loadExtensions]);

  const handleInstallFolder = async () => {
    const selected = await open({ directory: true });
    if (!selected) return;
    await doInstall(selected as string);
  };

  const handleInstallZip = async () => {
    const selected = await open({
      directory: false,
      filters: [
        { name: "Extension Archive", extensions: ["zip", "tar.gz", "tgz", "gz"] },
        { name: "All Files", extensions: ["*"] },
      ],
    });
    if (!selected) return;
    await doInstall(selected as string);
  };

  const doInstall = async (sourcePath: string) => {
    setInstalling(true);
    try {
      await invoke("install_extension_cmd", { sourcePath });
      await loadExtensions();
      toast({ title: "Extension Installed", description: "Extension installed successfully" });
    } catch (err) {
      const msg = typeof err === "object" && err !== null && "message" in err ? (err as { message: string }).message : String(err);
      toast({ title: "Install failed", description: msg, variant: "destructive" });
    } finally {
      setInstalling(false);
    }
  };

  const handleToggle = async (ext: ExtensionInfo) => {
    try {
      await invoke("toggle_extension_cmd", { extensionId: ext.id, enabled: !ext.enabled });
      await loadExtensions();
      toast({
        title: ext.enabled ? "Extension Disabled" : "Extension Enabled",
        description: `${ext.name} has been ${ext.enabled ? "disabled" : "enabled"}`,
      });
    } catch (err) {
      console.error("Failed to toggle extension:", err);
      toast({ title: "Error", description: "Failed to toggle extension", variant: "destructive" });
    }
  };

  const handleDelete = async (extensionId: string) => {
    try {
      await invoke("uninstall_extension_cmd", { extensionId });
      setConfirmDelete(null);
      await loadExtensions();
      toast({ title: "Extension Removed" });
    } catch (err) {
      console.error("Failed to remove extension:", err);
      toast({ title: "Error", description: "Failed to remove extension", variant: "destructive" });
    }
  };

  const handleExport = async (ext: ExtensionInfo) => {
    const dest = await save({
      defaultPath: `${ext.id}.zip`,
      filters: [{ name: "ZIP Archive", extensions: ["zip"] }],
    });
    if (!dest) return;
    try {
      await invoke("export_extension_cmd", { extensionId: ext.id, outputPath: dest });
      toast({ title: "Extension Exported", description: `Saved to ${dest}` });
    } catch (err) {
      const msg = typeof err === "object" && err !== null && "message" in err ? (err as { message: string }).message : String(err);
      toast({ title: "Export failed", description: msg, variant: "destructive" });
    }
  };

  const formatType = (type: string) => {
    switch (type) {
      case "rule-pack":
        return "Rule Pack";
      case "theme":
        return "Theme";
      case "docs":
        return "Documentation";
      default:
        return type;
    }
  };

  return (
    <div data-testid="page-extensions" className="p-6 max-w-3xl flex flex-col gap-6 h-full">
      {/* Header */}
      <div>
        <h1 className="text-xl font-semibold">Extensions</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Install and manage rule packs, themes, and other extensions.
        </p>
      </div>

      {/* Install buttons */}
      <div className="flex items-center gap-3">
        <Button data-testid="btn-install-extension" onClick={handleInstallFolder} disabled={installing}>
          {installing ? "Installing..." : "Install from Folder"}
        </Button>
        <Button variant="outline" onClick={handleInstallZip} disabled={installing}>
          Install from Archive
        </Button>
      </div>

      {/* Extension list */}
      {isLoading ? (
        <div className="text-sm text-muted-foreground">Loading...</div>
      ) : extensions.length === 0 ? (
        <div data-testid="extensions-empty" className="flex-1 flex items-center justify-center">
          <div className="text-center space-y-2">
            <p className="text-muted-foreground text-sm">No extensions installed.</p>
            <p className="text-muted-foreground/60 text-xs">
              Install rule packs or themes to extend Project Modzboid's functionality.
            </p>
          </div>
        </div>
      ) : (
        <div data-testid="extension-list" className="space-y-2">
          {extensions.map((ext) => (
            <div
              key={ext.id}
              data-testid={`extension-card-${ext.id}`}
              className="flex items-center gap-4 p-3 bg-card border border-border rounded-lg"
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-sm">{ext.name}</span>
                  <span className="text-xs text-muted-foreground">v{ext.version}</span>
                  <span className="inline-flex items-center px-1.5 py-0.5 text-xs rounded bg-muted text-muted-foreground">
                    {formatType(ext.extensionType)}
                  </span>
                </div>
                <div className="text-xs text-muted-foreground mt-0.5">
                  by {ext.author}
                </div>
                {ext.description && (
                  <p className="text-xs text-muted-foreground/80 mt-1 line-clamp-2">
                    {ext.description}
                  </p>
                )}
              </div>
              <div className="flex items-center gap-1.5">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => handleToggle(ext)}
                >
                  {ext.enabled ? "Disable" : "Enable"}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => handleExport(ext)}
                >
                  Export
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-destructive hover:text-destructive"
                  onClick={() => setConfirmDelete(ext.id)}
                >
                  Delete
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Confirm delete dialog */}
      <ConfirmDialog
        open={confirmDelete !== null}
        onOpenChange={() => setConfirmDelete(null)}
        title="Remove Extension"
        description="This extension will be permanently removed."
        actions={[
          { label: "Cancel", variant: "outline", onClick: () => setConfirmDelete(null) },
          {
            label: "Delete",
            variant: "destructive",
            onClick: () => confirmDelete && handleDelete(confirmDelete),
          },
        ]}
      />
    </div>
  );
}
