import { useState } from "react";
import { PathSettings } from "../components/PathSettings";
import { AppSettings } from "../components/AppSettings";

type Tab = "paths" | "app";

const TABS: { id: Tab; label: string }[] = [
  { id: "paths", label: "Paths" },
  { id: "app", label: "App" },
];

export default function SettingsPage() {
  const [activeTab, setActiveTab] = useState<Tab>("paths");

  return (
    <div data-testid="page-settings" className="p-6 max-w-2xl">
      <h2 className="text-xl font-bold mb-6">Settings</h2>

      {/* Tab bar */}
      <div className="flex gap-1 border-b border-border mb-6">
        {TABS.map(({ id, label }) => (
          <button
            key={id}
            data-testid={`tab-${id}`}
            onClick={() => setActiveTab(id)}
            className={[
              "px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors",
              activeTab === id
                ? "border-primary text-foreground"
                : "border-transparent text-muted-foreground hover:text-foreground",
            ].join(" ")}
          >
            {label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div>
        {activeTab === "paths" && <PathSettings />}
        {activeTab === "app" && <AppSettings />}
      </div>
    </div>
  );
}
