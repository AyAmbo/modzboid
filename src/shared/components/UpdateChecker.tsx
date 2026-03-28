import { useState, useEffect } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { useAppStore } from "../stores/appStore";
import { ConfirmDialog } from "./ui/confirm-dialog";

export function UpdateChecker() {
  const config = useAppStore((s) => s.config);
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [updateVersion, setUpdateVersion] = useState("");
  const [updateBody, setUpdateBody] = useState("");
  const [installing, setInstalling] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);

  useEffect(() => {
    // Only check if user has opted in
    if (!config?.checkUpdates) return;

    const checkForUpdate = async () => {
      try {
        const update = await check();
        if (update) {
          setUpdateVersion(update.version);
          setUpdateBody(update.body ?? "");
          setUpdateAvailable(true);
          setDialogOpen(true);
        }
      } catch (err) {
        // Silently fail — update check is optional
        console.debug("Update check failed:", err);
      }
    };

    // Check after 5 seconds (don't block startup)
    const timer = setTimeout(checkForUpdate, 5000);
    return () => clearTimeout(timer);
  }, [config?.checkUpdates]);

  const handleInstall = async () => {
    setInstalling(true);
    try {
      const update = await check();
      if (update) {
        // Download and install — Tauri verifies the Ed25519 signature
        // before applying. If verification fails, the update is rejected.
        await update.downloadAndInstall();
        // Tauri will prompt for restart
      }
    } catch (err) {
      console.error("Update installation failed:", err);
    } finally {
      setInstalling(false);
    }
  };

  if (!updateAvailable) return null;

  return (
    <ConfirmDialog
      open={dialogOpen}
      onOpenChange={setDialogOpen}
      title={`Update Available: v${updateVersion}`}
      description="A new version of Project Modzboid is available. The update will be verified with a cryptographic signature before installation."
      actions={[
        {
          label: "Not Now",
          variant: "outline",
          onClick: () => setDialogOpen(false),
        },
        {
          label: installing ? "Installing..." : "Download & Install",
          onClick: handleInstall,
        },
      ]}
    >
      {updateBody && (
        <div className="py-2">
          <div className="text-sm font-medium mb-1">What's new:</div>
          <div className="text-xs text-muted-foreground whitespace-pre-line bg-muted p-3 rounded max-h-40 overflow-auto">
            {updateBody}
          </div>
        </div>
      )}
      <div className="text-xs text-muted-foreground/60 mt-1">
        Updates are cryptographically signed (Ed25519). Invalid signatures are
        automatically rejected.
      </div>
    </ConfirmDialog>
  );
}
