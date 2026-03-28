import { useMemo } from "react";
import { ConfirmDialog } from "../../../shared/components/ui/confirm-dialog";
import { useModManagerStore } from "../store";

interface DisableWarningDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  modName: string;
  dependentIds: string[];
  onDisableAll: () => void;
  onDisableOnly: () => void;
}

export function DisableWarningDialog({
  open,
  onOpenChange,
  modName,
  dependentIds,
  onDisableAll,
  onDisableOnly,
}: DisableWarningDialogProps) {
  const allMods = useModManagerStore((s) => s.allMods);

  const dependentNames = useMemo(() => {
    const nameMap = new Map(allMods.map((m) => [m.id, m.name]));
    return dependentIds.map((id) => nameMap.get(id) ?? id);
  }, [allMods, dependentIds]);

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Dependent Mods Warning"
      description={`${modName} is required by the following enabled mods:`}
      actions={[
        { label: "Cancel", variant: "outline", onClick: () => onOpenChange(false) },
        { label: `Disable Only ${modName}`, variant: "ghost", onClick: () => { onDisableOnly(); onOpenChange(false); } },
        { label: `Disable All (${dependentIds.length + 1})`, variant: "destructive", onClick: () => { onDisableAll(); onOpenChange(false); } },
      ]}
    >
      <div className="py-2 space-y-1">
        {dependentNames.map((name, i) => (
          <div key={dependentIds[i]} className="text-sm px-2 py-1 bg-muted rounded flex items-center gap-2">
            <span className="text-destructive text-xs">-</span>
            {name}
          </div>
        ))}
      </div>
    </ConfirmDialog>
  );
}
