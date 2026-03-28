import { useCallback } from "react";
import type { DragEndEvent } from "@dnd-kit/core";
import { arrayMove } from "@dnd-kit/sortable";
import { useProfileStore, useActiveProfile } from "../../profiles/store";

export function useModDragDrop() {
  const activeProfile = useActiveProfile();
  const reorderMods = useProfileStore((s) => s.reorderMods);

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;
      if (!over || !activeProfile) return;
      if (active.id === over.id) return;

      const oldIndex = activeProfile.loadOrder.indexOf(String(active.id));
      const newIndex = activeProfile.loadOrder.indexOf(String(over.id));

      if (oldIndex === -1 || newIndex === -1) return;

      const newOrder = arrayMove(activeProfile.loadOrder, oldIndex, newIndex);
      reorderMods(newOrder);
    },
    [activeProfile, reorderMods]
  );

  return { handleDragEnd };
}
