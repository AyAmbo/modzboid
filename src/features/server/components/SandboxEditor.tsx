import { useMemo, useState, useEffect } from "react";
import { SettingField } from "./SettingField";
import { Button } from "../../../shared/components/ui/button";
import { useServerStore } from "../store";

export function SandboxEditor() {
  const sandboxVars = useServerStore((s) => s.sandboxVars);
  const dirtySandbox = useServerStore((s) => s.dirtySandbox);
  const updateSandboxSetting = useServerStore((s) => s.updateSandboxSetting);
  const saveSandboxVars = useServerStore((s) => s.saveSandboxVars);
  const reloadSandbox = useServerStore((s) => s.reloadSandbox);
  const undoSandboxChanges = useServerStore((s) => s.undoSandboxChanges);
  const activeConfigPath = useServerStore((s) => s.activeConfigPath);
  const [activeTab, setActiveTab] = useState<string>("General");
  const [saving, setSaving] = useState(false);
  const [reloading, setReloading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Reset activeTab when sandboxVars changes (different config selected)
  useEffect(() => {
    setActiveTab("General");
    setError(null);
  }, [sandboxVars]);

  const tabs = useMemo(() => {
    if (!sandboxVars) return [];
    try {
      const result: Array<{ name: string; isCategory: boolean }> = [];
      const topLevel = sandboxVars.topLevel ?? [];
      const categories = sandboxVars.categories ?? [];
      if (topLevel.length > 0) {
        result.push({ name: "General", isCategory: false });
      }
      for (const cat of categories) {
        if (cat?.name) {
          result.push({ name: cat.name, isCategory: true });
        }
      }
      return result;
    } catch (e) {
      setError(`Failed to parse sandbox tabs: ${e}`);
      return [];
    }
  }, [sandboxVars]);

  const activeSettings = useMemo(() => {
    if (!sandboxVars) return [];
    try {
      if (activeTab === "General") return sandboxVars.topLevel ?? [];
      const categories = sandboxVars.categories ?? [];
      const cat = categories.find((c) => c.name === activeTab);
      return cat?.settings ?? [];
    } catch (e) {
      setError(`Failed to load settings for ${activeTab}: ${e}`);
      return [];
    }
  }, [sandboxVars, activeTab]);

  const activeCategoryName = activeTab === "General" ? null : activeTab;

  const handleSave = async () => {
    setSaving(true);
    try {
      await saveSandboxVars();
    } catch (e) {
      setError(`Failed to save: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleReload = async () => {
    setReloading(true);
    await reloadSandbox();
    setReloading(false);
  };

  if (!sandboxVars) {
    const activeConfig = activeConfigPath;
    const expectedFile = activeConfig
      ? activeConfig.replace(/\.ini$/, "_SandboxVars.lua").split(/[\\/]/).pop()
      : null;
    return (
      <div className="p-4 space-y-2">
        <p className="text-sm text-muted-foreground">
          {activeConfig
            ? "No sandbox variables file found for this server config."
            : "Select a server config to view sandbox variables."}
        </p>
        {expectedFile && (
          <p className="text-xs text-muted-foreground/70">
            Expected file: <code className="bg-muted px-1 py-0.5 rounded">{expectedFile}</code>
          </p>
        )}
        {activeConfig && (
          <p className="text-xs text-muted-foreground/70">
            Sandbox variables are created when you first start a server with this config.
          </p>
        )}
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-4 space-y-3">
        <div className="text-sm text-destructive">{error}</div>
        <Button variant="outline" size="sm" onClick={() => setError(null)}>Dismiss</Button>
      </div>
    );
  }

  return (
    <div className="flex h-full">
      {/* Category sidebar */}
      <div className="w-44 border-r border-border overflow-auto shrink-0">
        {tabs.map((tab) => (
          <button
            key={tab.name}
            onClick={() => setActiveTab(tab.name)}
            className={`w-full text-left px-3 py-2 text-sm transition-colors ${
              activeTab === tab.name
                ? "bg-primary/10 text-primary font-medium"
                : "text-muted-foreground hover:bg-muted"
            }`}
          >
            {tab.name}
          </button>
        ))}
      </div>

      {/* Settings */}
      <div className="flex-1 flex flex-col min-h-0">
        {/* Toolbar */}
        <div className="flex items-center gap-2 px-4 py-2 border-b border-border bg-card shrink-0">
          <Button onClick={handleSave} disabled={saving || dirtySandbox.size === 0} size="sm">
            {saving ? "Saving..." : "Save"}
          </Button>
          <Button variant="outline" onClick={handleReload} disabled={reloading} size="sm" title="Reload from disk">
            {reloading ? "Reloading..." : "Reload"}
          </Button>
          <Button variant="outline" onClick={undoSandboxChanges} disabled={dirtySandbox.size === 0} size="sm" title="Discard all unsaved changes">
            Undo
          </Button>
          {dirtySandbox.size > 0 && (
            <span className="text-xs text-warning ml-1">
              {dirtySandbox.size} unsaved change{dirtySandbox.size !== 1 ? "s" : ""}
            </span>
          )}
        </div>

        <div className="flex-1 overflow-auto">
          {activeSettings.length === 0 ? (
            <div className="p-4 text-sm text-muted-foreground">
              No settings found for "{activeTab}".
            </div>
          ) : (
            <div className="p-4 space-y-1">
              {activeSettings.map((setting) => {
                if (!setting?.key) return null;
                const mapKey = activeCategoryName
                  ? `${activeCategoryName}.${setting.key}`
                  : setting.key;
                const currentValue = dirtySandbox.get(mapKey) ?? String(setting.value ?? "");
                const isDirty = dirtySandbox.has(mapKey);
                return (
                  <SettingField
                    key={setting.key}
                    label={setting.key}
                    value={currentValue}
                    description={setting.description ?? null}
                    settingType={setting.settingType ?? "string"}
                    min={setting.min ?? null}
                    max={setting.max ?? null}
                    defaultValue={setting.defaultValue ?? null}
                    enumOptions={setting.enumOptions}
                    isDirty={isDirty}
                    onChange={(v) => updateSandboxSetting(activeCategoryName, setting.key, v)}
                  />
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
