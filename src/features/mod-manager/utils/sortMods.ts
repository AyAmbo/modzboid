import type { ModInfo } from "../../../shared/types/modTypes";
import type { SortField } from "../store";

export function sortMods(mods: ModInfo[], field: SortField, dir: "asc" | "desc"): ModInfo[] {
  return [...mods].sort((a, b) => {
    let cmp = 0;
    switch (field) {
      case "name": cmp = a.name.localeCompare(b.name); break;
      case "version": cmp = (a.modVersion ?? "").localeCompare(b.modVersion ?? ""); break;
      case "author": cmp = (a.authors[0] ?? "").localeCompare(b.authors[0] ?? ""); break;
      case "size": cmp = (a.sizeBytes ?? 0) - (b.sizeBytes ?? 0); break;
      case "lastModified": cmp = (a.lastModified ?? "").localeCompare(b.lastModified ?? ""); break;
      case "source": cmp = a.source.localeCompare(b.source); break;
      case "workshopId": cmp = (a.workshopId ?? "").localeCompare(b.workshopId ?? ""); break;
      case "category": cmp = (a.detectedCategory ?? a.category ?? "").localeCompare(b.detectedCategory ?? b.category ?? ""); break;
      case "id": cmp = a.id.localeCompare(b.id); break;
    }
    return dir === "asc" ? cmp : -cmp;
  });
}
