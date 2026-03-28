import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "../../../shared/components/ui/button";
import { useProfileStore } from "../store";

interface MissingConfigBannerProps {
  configPath: string;
}

export function MissingConfigBanner({ configPath }: MissingConfigBannerProps) {
  const unlinkServerConfig = useProfileStore((s) => s.unlinkServerConfig);
  const persistProfile = useProfileStore((s) => s.persistProfile);
  const profiles = useProfileStore((s) => s.profiles);
  const activeProfileId = useProfileStore((s) => s.activeProfileId);
  const [browsing, setBrowsing] = useState(false);

  const fileName = configPath.split(/[\\/]/).pop() || "server.ini";

  const handleBrowse = async () => {
    setBrowsing(true);
    try {
      const selected = await open({
        title: "Locate server config file",
        filters: [{ name: "INI files", extensions: ["ini"] }],
      });
      if (selected) {
        // Update the profile's serverConfigPath
        const updated = profiles.map((p) =>
          p.id === activeProfileId ? { ...p, serverConfigPath: selected as string } : p
        );
        useProfileStore.setState({ profiles: updated });
        await persistProfile();
      }
    } finally {
      setBrowsing(false);
    }
  };

  return (
    <div className="mx-3 mt-2 px-4 py-3 border border-warning/50 bg-warning/10 rounded-lg flex items-center gap-3">
      <span className="text-warning text-lg shrink-0">&#9888;</span>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-foreground">
          {fileName} not found
        </p>
        <p className="text-xs text-muted-foreground truncate" title={configPath}>
          The linked server config no longer exists at the saved path.
        </p>
      </div>
      <div className="flex items-center gap-2 shrink-0">
        <Button variant="outline" size="sm" onClick={handleBrowse} disabled={browsing}>
          {browsing ? "Browsing..." : "Browse"}
        </Button>
        <Button variant="ghost" size="sm" onClick={unlinkServerConfig}>
          Unlink
        </Button>
      </div>
    </div>
  );
}
