import { Input } from "../../../shared/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "../../../shared/components/ui/dropdown-menu";
import { Button } from "../../../shared/components/ui/button";
import { useModManagerStore } from "../store";
import type { ModCategory } from "../../../shared/types/modTypes";

const categories: Array<{ value: ModCategory | null; label: string }> = [
  { value: null, label: "All Categories" },
  { value: "framework", label: "Framework" },
  { value: "map", label: "Map" },
  { value: "content", label: "Content" },
  { value: "overhaul", label: "Overhaul" },
];

export function ModFilters() {
  const availableSearch = useModManagerStore((s) => s.availableSearch);
  const setAvailableSearch = useModManagerStore((s) => s.setAvailableSearch);
  const categoryFilter = useModManagerStore((s) => s.categoryFilter);
  const setCategoryFilter = useModManagerStore((s) => s.setCategoryFilter);

  const currentLabel =
    categories.find((c) => c.value === categoryFilter)?.label ?? "All Categories";

  return (
    <div className="flex items-center gap-2 p-2">
      <Input
        data-testid="mod-search-available"
        placeholder="Search available..."
        value={availableSearch}
        onChange={(e) => setAvailableSearch(e.target.value)}
        className="h-7 text-xs"
      />
      <DropdownMenu>
        <DropdownMenuTrigger>
          <Button variant="outline" size="sm" className="whitespace-nowrap">
            {currentLabel}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          {categories.map((cat) => (
            <DropdownMenuItem
              key={cat.label}
              onClick={() => setCategoryFilter(cat.value)}
            >
              {cat.label}
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
