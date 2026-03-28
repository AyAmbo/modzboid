import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { useModManagerStore } from "../store";
import { toast } from "../../../shared/components/ui/toaster";
import type { ModInfo } from "../../../shared/types/modTypes";

interface ModContextMenuProps {
  mod: ModInfo;
  isActive: boolean;
  onToggle?: () => void;
  children: React.ReactNode;
}

interface MenuPosition {
  x: number;
  y: number;
}

/**
 * Get the effective CSS zoom factor applied to the document.
 * CSS zoom causes clientX/Y to be in the unzoomed coordinate space,
 * but position:fixed renders in the zoomed space. Dividing by zoom corrects this.
 */
function getEffectiveZoom(): number {
  // Check CSS zoom on html and body
  const htmlZoom = parseFloat(getComputedStyle(document.documentElement).zoom) || 1;
  const bodyZoom = parseFloat(getComputedStyle(document.body).zoom) || 1;
  return htmlZoom * bodyZoom;
}

export function ModContextMenu({ mod, isActive, onToggle, children }: ModContextMenuProps) {
  const [pos, setPos] = useState<MenuPosition | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();

    // Correct for CSS zoom: clientX is in unzoomed viewport coords,
    // but fixed positioning renders in zoomed coords
    const zoom = getEffectiveZoom();
    setPos({ x: e.clientX / zoom, y: e.clientY / zoom });
  }, []);

  // After menu renders, clamp to viewport
  useEffect(() => {
    if (!pos || !menuRef.current) return;
    const menu = menuRef.current;
    const rect = menu.getBoundingClientRect();
    const zoom = getEffectiveZoom();
    const vw = window.innerWidth / zoom;
    const vh = window.innerHeight / zoom;

    let { x, y } = pos;
    if (x + rect.width / zoom > vw) x = vw - rect.width / zoom - 4;
    if (y + rect.height / zoom > vh) y = vh - rect.height / zoom - 4;
    if (x < 4) x = 4;
    if (y < 4) y = 4;

    if (x !== pos.x || y !== pos.y) {
      setPos({ x, y });
    }
  });

  // Close on click outside, right-click elsewhere, or Escape
  useEffect(() => {
    if (!pos) return;
    const handleClose = () => setPos(null);
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setPos(null);
    };
    document.addEventListener("click", handleClose);
    document.addEventListener("contextmenu", handleClose, true);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("click", handleClose);
      document.removeEventListener("contextmenu", handleClose, true);
      document.removeEventListener("keydown", handleKey);
    };
  }, [pos]);

  const openFolder = useCallback(async () => {
    try {
      if (mod.sourcePath) {
        await invoke("open_folder_cmd", { path: mod.sourcePath });
      }
    } catch (err) {
      console.error("Failed to open folder:", err);
    }
    setPos(null);
  }, [mod.sourcePath]);

  const copyModId = useCallback(() => {
    navigator.clipboard.writeText(mod.id);
    setPos(null);
  }, [mod.id]);

  const copyWorkshopUrl = useCallback(() => {
    if (mod.workshopId) {
      navigator.clipboard.writeText(`https://steamcommunity.com/sharedfiles/filedetails/?id=${mod.workshopId}`);
    }
    setPos(null);
  }, [mod.workshopId]);

  const selectMod = useModManagerStore((s) => s.selectMod);
  const handleSelect = useCallback(() => {
    selectMod(mod.id);
    setPos(null);
  }, [mod.id, selectMod]);

  const runCompatCheck = useCallback(async (forceFull: boolean) => {
    setPos(null);
    try {
      const report = await invoke<{
        modId: string;
        totalIssues: number;
        issues: { file: string; line: number; oldApi: string; message: string }[];
      }>("scan_mod_migration_cmd", { modId: mod.id, forceFull });

      if (report.totalIssues === 0) {
        toast({ title: "Compatible", description: `${mod.name} has no compatibility issues.` });
      } else {
        toast({
          title: `${report.totalIssues} issue${report.totalIssues !== 1 ? "s" : ""} found`,
          description: `${mod.name}: check Compatibility tab for details.`,
          variant: "destructive",
        });
      }
    } catch (err) {
      toast({
        title: "Scan failed",
        description: typeof err === "object" && err !== null && "message" in err ? (err as { message: string }).message : String(err),
        variant: "destructive",
      });
    }
  }, [mod.id, mod.name]);

  const handleAutoFix = useCallback(async () => {
    setPos(null);
    try {
      const report = await invoke<{
        modName: string;
        outputPath: string;
        fixesApplied: number;
        todosAdded: number;
        translationEntries: number;
      }>("auto_fix_mod_cmd", { modId: mod.id });
      toast({
        title: "Fixed copy created",
        description: `${report.fixesApplied} fixes applied, ${report.todosAdded} TODOs added. Saved to: ${report.outputPath}`,
      });
    } catch (err: unknown) {
      const msg = typeof err === "object" && err !== null && "message" in err
        ? (err as { message: string }).message
        : typeof err === "string" ? err : JSON.stringify(err);
      toast({
        title: "Auto-fix failed",
        description: msg,
        variant: "destructive",
      });
    }
  }, [mod.id]);

  const menuItems = [
    { label: isActive ? "Disable" : "Enable", action: () => { onToggle?.(); setPos(null); }, show: !!onToggle },
    { label: "View Details", action: handleSelect, show: true },
    { label: "---", action: () => {}, show: true },
    { label: "Check Compatibility", action: () => runCompatCheck(false), show: true },
    { label: "Full Check (all rules)", action: () => runCompatCheck(true), show: true },
    { label: "Auto-Fix (create local copy)", action: handleAutoFix, show: true },
    { label: "---", action: () => {}, show: true },
    { label: "Open Folder", action: openFolder, show: true },
    { label: "Copy Mod ID", action: copyModId, show: true },
    { label: "Copy Workshop URL", action: copyWorkshopUrl, show: !!mod.workshopId },
    { label: "---", action: () => {}, show: true },
    { label: `Source: ${mod.source}`, action: () => setPos(null), show: true, disabled: true },
    { label: mod.workshopId ? `Workshop: ${mod.workshopId}` : "Local mod", action: () => setPos(null), show: true, disabled: true },
  ];

  return (
    <div onContextMenu={handleContextMenu}>
      {children}
      {pos && createPortal(
        <div
          ref={menuRef}
          className="fixed z-50 min-w-[180px] rounded-md border border-border bg-card shadow-md py-1"
          style={{ left: pos.x, top: pos.y }}
          onClick={(e) => e.stopPropagation()}
        >
          {menuItems.filter((i) => i.show).map((item, idx) => {
            if (item.label === "---") {
              return <div key={`sep-${idx}`} className="my-1 h-px bg-border" />;
            }
            return (
              <button
                key={item.label}
                onClick={item.action}
                disabled={(item as { disabled?: boolean }).disabled}
                className="w-full text-left px-3 py-1.5 text-sm hover:bg-muted transition-colors disabled:opacity-50 disabled:cursor-default"
              >
                {item.label}
              </button>
            );
          })}
        </div>,
        document.body
      )}
    </div>
  );
}
