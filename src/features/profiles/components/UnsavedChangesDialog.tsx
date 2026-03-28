import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "../../../shared/components/ui/dialog";
import { Button } from "../../../shared/components/ui/button";

interface UnsavedChangesDialogProps {
  open: boolean;
  onSaveAndSwitch: () => void;
  onDiscard: () => void;
  onCancel: () => void;
  changeCount: number;
  profileName: string;
}

export function UnsavedChangesDialog({
  open,
  onSaveAndSwitch,
  onDiscard,
  onCancel,
  changeCount,
  profileName,
}: UnsavedChangesDialogProps) {
  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) onCancel(); }}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Unsaved changes</DialogTitle>
          <DialogDescription>
            "{profileName}" has {changeCount} unsaved change{changeCount !== 1 ? "s" : ""} that haven't been saved to the server.ini file.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter className="gap-2 sm:gap-0">
          <Button variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="ghost" onClick={onDiscard}>
            Discard
          </Button>
          <Button variant="default" onClick={onSaveAndSwitch}>
            Save & Switch
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
