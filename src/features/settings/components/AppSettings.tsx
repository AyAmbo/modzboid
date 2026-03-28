import { useAppStore } from "../../../shared/stores/appStore";
import { Switch } from "../../../shared/components/ui/switch";
import type { AppConfig } from "../../../shared/types/app";

type Theme = "dark" | "light" | "system";

const THEMES: { value: Theme; label: string }[] = [
  { value: "dark", label: "Dark" },
  { value: "light", label: "Light" },
  { value: "system", label: "System" },
];

const UI_SCALES = [
  { value: 50, label: "50%" },
  { value: 60, label: "60%" },
  { value: 70, label: "70%" },
  { value: 75, label: "75%" },
  { value: 80, label: "80%" },
  { value: 90, label: "90%" },
  { value: 100, label: "100%" },
  { value: 110, label: "110%" },
  { value: 120, label: "120%" },
  { value: 130, label: "130%" },
  { value: 150, label: "150%" },
  { value: 175, label: "175%" },
  { value: 200, label: "200%" },
  { value: 250, label: "250%" },
  { value: 300, label: "300%" },
];

const FONT_SIZES = [
  { value: 10, label: "Tiny (10px)" },
  { value: 12, label: "Small (12px)" },
  { value: 13, label: "13px" },
  { value: 14, label: "Medium (14px)" },
  { value: 16, label: "Large (16px)" },
  { value: 18, label: "18px" },
  { value: 20, label: "20px" },
  { value: 24, label: "24px" },
  { value: 28, label: "28px" },
  { value: 30, label: "30px" },
];

export function AppSettings() {
  const { config, saveConfig } = useAppStore();
  const currentTheme = (config?.theme as Theme) ?? "dark";
  const checkUpdates = config?.checkUpdates ?? true;
  const uiScale = config?.uiScale ?? 100;
  const fontSize = config?.fontSize ?? 14;

  const handleTheme = async (theme: Theme) => {
    if (!config) return;
    const updated: AppConfig = { ...config, theme };
    await saveConfig(updated);
  };

  const handleCheckUpdates = async (enabled: boolean) => {
    if (!config) return;
    const updated: AppConfig = { ...config, checkUpdates: enabled };
    await saveConfig(updated);
  };

  const handleUiScale = async (scale: number) => {
    if (!config) return;
    const updated: AppConfig = { ...config, uiScale: scale };
    await saveConfig(updated);
  };

  const handleFontSize = async (size: number) => {
    if (!config) return;
    const updated: AppConfig = { ...config, fontSize: size };
    await saveConfig(updated);
  };

  return (
    <div className="space-y-6">
      {/* Theme */}
      <div>
        <h3 className="text-sm font-medium mb-3">Theme</h3>
        <div className="flex gap-2">
          {THEMES.map(({ value, label }) => (
            <button
              key={value}
              onClick={() => handleTheme(value)}
              className={[
                "px-4 py-2 text-sm rounded border transition-colors",
                currentTheme === value
                  ? "bg-primary text-primary-foreground border-primary"
                  : "border-border hover:bg-muted text-muted-foreground",
              ].join(" ")}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {/* UI Scale */}
      <div>
        <h3 className="text-sm font-medium mb-3">UI Scale</h3>
        <select
          value={uiScale}
          onChange={(e) => handleUiScale(Number(e.target.value))}
          className="px-3 py-2 text-sm rounded border border-border bg-muted text-foreground"
        >
          {UI_SCALES.map(({ value, label }) => (
            <option key={value} value={value}>
              {label}
            </option>
          ))}
        </select>
      </div>

      {/* Font Size */}
      <div>
        <h3 className="text-sm font-medium mb-3">Font Size</h3>
        <select
          value={fontSize}
          onChange={(e) => handleFontSize(Number(e.target.value))}
          className="px-3 py-2 text-sm rounded border border-border bg-muted text-foreground"
        >
          {FONT_SIZES.map(({ value, label }) => (
            <option key={value} value={value}>
              {label}
            </option>
          ))}
        </select>
      </div>

      {/* Updates */}
      <div>
        <h3 className="text-sm font-medium mb-3">Updates</h3>
        <div className="flex items-center justify-between">
          <div>
            <div className="text-sm">Check for updates</div>
            <div className="text-xs text-muted-foreground">
              Updates are verified with Ed25519 cryptographic signatures
            </div>
          </div>
          <Switch
            checked={checkUpdates}
            onCheckedChange={handleCheckUpdates}
          />
        </div>
      </div>

      {/* About */}
      <div>
        <h3 className="text-sm font-medium mb-3">About</h3>
        <div className="bg-muted rounded p-4 space-y-2 text-sm">
          <div className="flex justify-between">
            <span className="text-muted-foreground">Version</span>
            <span className="font-medium">v0.1.0</span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted-foreground">App</span>
            <span className="font-medium">Project Modzboid</span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted-foreground">Description</span>
            <span className="font-medium">Mod manager for Project Zomboid</span>
          </div>
        </div>
      </div>
    </div>
  );
}
