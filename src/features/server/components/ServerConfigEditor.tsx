import { useMemo, useState } from "react";
import { SettingField } from "./SettingField";
import { Button } from "../../../shared/components/ui/button";
import { Input } from "../../../shared/components/ui/input";
import { useServerStore } from "../store";
import type { ServerSetting } from "../types";

export function ServerConfigEditor() {
  const serverConfig = useServerStore((s) => s.serverConfig);
  const dirtySettings = useServerStore((s) => s.dirtySettings);
  const updateSetting = useServerStore((s) => s.updateSetting);
  const saveServerConfig = useServerStore((s) => s.saveServerConfig);
  const reloadConfig = useServerStore((s) => s.reloadConfig);
  const undoConfigChanges = useServerStore((s) => s.undoConfigChanges);
  const [activeCategory, setActiveCategory] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [reloading, setReloading] = useState(false);
  const [searchText, setSearchText] = useState("");

  const categories = useMemo(() => {
    if (!serverConfig) return [];
    const catMap = new Map<string, ServerSetting[]>();
    for (const s of serverConfig.settings) {
      const list = catMap.get(s.category) || [];
      list.push(s);
      catMap.set(s.category, list);
    }
    return Array.from(catMap.entries()).map(([name, settings]) => ({ name, settings }));
  }, [serverConfig]);

  // When searching, filter all settings across all categories
  const searchResults = useMemo(() => {
    if (!searchText.trim() || !serverConfig) return null;
    const q = searchText.toLowerCase();
    return serverConfig.settings.filter(
      (s) =>
        s.key.toLowerCase().includes(q) ||
        s.value.toLowerCase().includes(q) ||
        (s.description && s.description.toLowerCase().includes(q))
    );
  }, [searchText, serverConfig]);

  const isSearching = searchResults !== null;

  const activeCat = activeCategory ?? categories[0]?.name ?? null;
  const activeSettings = isSearching
    ? searchResults
    : categories.find((c) => c.name === activeCat)?.settings ?? [];

  const handleSave = async () => {
    setSaving(true);
    await saveServerConfig();
    setSaving(false);
  };

  const handleReload = async () => {
    setReloading(true);
    await reloadConfig();
    setReloading(false);
  };

  if (!serverConfig) {
    return <div className="p-4 text-sm text-muted-foreground">Select a server config to edit.</div>;
  }

  return (
    <div className="flex h-full">
      {/* Category sidebar */}
      <div className="w-40 border-r border-border overflow-auto shrink-0 flex flex-col">
        {/* Search input */}
        <div className="p-2 border-b border-border">
          <Input
            placeholder="Search settings..."
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            className="h-7 text-xs"
          />
        </div>
        {categories.map((cat) => (
          <button
            key={cat.name}
            onClick={() => { setActiveCategory(cat.name); setSearchText(""); }}
            className={`w-full text-left px-3 py-2 text-sm transition-colors ${
              !isSearching && activeCat === cat.name
                ? "bg-primary/10 text-primary font-medium"
                : "text-muted-foreground hover:bg-muted"
            }`}
          >
            {cat.name}
            <span className="ml-1 text-xs text-muted-foreground/60">({cat.settings.length})</span>
          </button>
        ))}
      </div>

      {/* Settings */}
      <div className="flex-1 flex flex-col min-h-0">
        {/* Toolbar */}
        <div className="flex items-center gap-2 px-4 py-2 border-b border-border bg-card shrink-0">
          <Button onClick={handleSave} disabled={saving || dirtySettings.size === 0} size="sm">
            {saving ? "Saving..." : "Save"}
          </Button>
          <Button variant="outline" onClick={handleReload} disabled={reloading} size="sm" title="Reload from disk">
            {reloading ? "Reloading..." : "Reload"}
          </Button>
          <Button variant="outline" onClick={undoConfigChanges} disabled={dirtySettings.size === 0} size="sm" title="Discard all unsaved changes">
            Undo
          </Button>
          {dirtySettings.size > 0 && (
            <span className="text-xs text-warning ml-1">
              {dirtySettings.size} unsaved change{dirtySettings.size !== 1 ? "s" : ""}
            </span>
          )}
        </div>

        <div className="flex-1 overflow-auto">
          {isSearching && (
            <div className="px-4 pt-3 pb-1 text-xs text-muted-foreground">
              {searchResults.length} result{searchResults.length !== 1 ? "s" : ""} for "{searchText}"
            </div>
          )}
          <div className="p-4 space-y-1">
            {activeSettings.map((setting) => {
              const currentValue = dirtySettings.get(setting.key) ?? setting.value;
              const isDirty = dirtySettings.has(setting.key);
              return (
                <SettingField
                  key={setting.key}
                  label={setting.key}
                  value={currentValue}
                  description={setting.description}
                  settingType={setting.settingType}
                  min={setting.min}
                  max={setting.max}
                  defaultValue={setting.defaultValue}
                  isDirty={isDirty}
                  onChange={(v) => updateSetting(setting.key, v)}
                />
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
