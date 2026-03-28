import * as React from "react";
import { useProfileStore, useActiveProfile } from "../store";
import { ProfileCreateDialog } from "./ProfileCreateDialog";
import { cn } from "../../../shared/lib/utils";

export function ProfileSwitcher() {
  const { profiles, switchProfile } = useProfileStore();
  const activeProfile = useActiveProfile();
  const [dropdownOpen, setDropdownOpen] = React.useState(false);
  const [createDialogOpen, setCreateDialogOpen] = React.useState(false);
  const containerRef = React.useRef<HTMLDivElement>(null);

  // Close dropdown when clicking outside
  React.useEffect(() => {
    if (!dropdownOpen) return;
    const handler = (e: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        setDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [dropdownOpen]);

  const handleSwitch = async (id: string) => {
    setDropdownOpen(false);
    await switchProfile(id);
  };

  return (
    <>
      <div ref={containerRef} className="relative flex items-center gap-1">
        {/* Dropdown trigger */}
        <button
          onClick={() => setDropdownOpen((v) => !v)}
          className={cn(
            "flex-1 flex items-center gap-1 px-2 py-1.5 rounded text-sm text-left",
            "hover:bg-muted transition-colors",
            dropdownOpen && "bg-muted"
          )}
        >
          <span className="flex-1 truncate text-foreground">
            {activeProfile?.name ?? "No Profile"}
          </span>
          <span className="text-muted-foreground text-xs shrink-0">
            {dropdownOpen ? "▲" : "▼"}
          </span>
        </button>

        {/* Create profile button */}
        <button
          onClick={() => setCreateDialogOpen(true)}
          title="Create new profile"
          className="h-6 w-6 flex items-center justify-center rounded text-muted-foreground hover:text-foreground hover:bg-muted transition-colors text-sm shrink-0"
        >
          +
        </button>

        {/* Dropdown menu */}
        {dropdownOpen && (
          <div className="absolute bottom-full mb-1 left-0 right-0 bg-card border border-border rounded-md shadow-lg py-1 z-50 min-w-[10rem]">
            {profiles.length === 0 ? (
              <div className="px-3 py-2 text-xs text-muted-foreground">
                No profiles
              </div>
            ) : (
              profiles.map((profile) => {
                const isActive = profile.id === activeProfile?.id;
                const typeLabel =
                  profile.type === "singleplayer" ? "SP" : "SV";
                return (
                  <button
                    key={profile.id}
                    onClick={() => handleSwitch(profile.id)}
                    className={cn(
                      "w-full flex items-center gap-2 px-3 py-1.5 text-sm text-left transition-colors",
                      isActive
                        ? "text-primary bg-primary/10"
                        : "text-foreground hover:bg-muted"
                    )}
                  >
                    <span className="flex-1 truncate">{profile.name}</span>
                    <span
                      className={cn(
                        "text-xs px-1 rounded shrink-0",
                        profile.type === "singleplayer"
                          ? "text-blue-400"
                          : "text-orange-400"
                      )}
                    >
                      {typeLabel}
                    </span>
                    {isActive && (
                      <span className="text-primary text-xs shrink-0">✓</span>
                    )}
                  </button>
                );
              })
            )}
          </div>
        )}
      </div>

      <ProfileCreateDialog
        open={createDialogOpen}
        onOpenChange={setCreateDialogOpen}
      />
    </>
  );
}
