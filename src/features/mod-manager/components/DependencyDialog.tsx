import { useMemo } from "react";
import { ConfirmDialog } from "../../../shared/components/ui/confirm-dialog";
import { useModManagerStore } from "../store";
import type { DepResolution } from "../../../shared/types/deps";

interface DependencyDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  modName: string;
  resolution: DepResolution;
  onConfirm: () => void;
}

export function DependencyDialog({
  open,
  onOpenChange,
  modName,
  resolution,
  onConfirm,
}: DependencyDialogProps) {
  const allMods = useModManagerStore((s) => s.allMods);

  const modNames = useMemo(() => {
    const nameMap = new Map(allMods.map((m) => [m.id, m.name]));
    return {
      toEnable: resolution.toEnable.map((id) => nameMap.get(id) ?? id),
      notInstalled: resolution.notInstalled,
    };
  }, [allMods, resolution]);

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Dependencies Required"
      description={`${modName} requires the following mods:`}
      actions={[
        { label: "Cancel", variant: "outline", onClick: () => onOpenChange(false) },
        {
          label: `Enable All (${resolution.toEnable.length + 1})`,
          onClick: () => { onConfirm(); onOpenChange(false); },
        },
      ]}
    >
      <div className="space-y-3 py-2">
        {modNames.toEnable.length > 0 && (
          <div>
            <div className="text-sm font-medium mb-1.5">Will be enabled:</div>
            <div className="space-y-1">
              {modNames.toEnable.map((name, i) => (
                <div key={resolution.toEnable[i]} className="text-sm px-2 py-1 bg-muted rounded flex items-center gap-2">
                  <span className="text-green-500 text-xs">+</span>
                  {name}
                </div>
              ))}
            </div>
          </div>
        )}
        {modNames.notInstalled.length > 0 && (
          <div>
            <div className="text-sm font-medium text-destructive mb-1.5">Not installed:</div>
            <div className="space-y-1">
              {modNames.notInstalled.map((id) => (
                <div key={id} className="text-sm px-2 py-1 bg-muted rounded text-muted-foreground">{id}</div>
              ))}
            </div>
          </div>
        )}
      </div>
    </ConfirmDialog>
  );
}
