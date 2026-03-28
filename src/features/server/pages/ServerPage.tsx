import { useEffect, useState } from "react";
import { useServerStore } from "../store";
import { ServerSelector } from "../components/ServerSelector";
import { ServerConfigEditor } from "../components/ServerConfigEditor";
import { SandboxEditor } from "../components/SandboxEditor";
import { RconTerminal } from "../components/RconTerminal";

type Tab = "config" | "sandbox" | "rcon";

export default function ServerPage() {
  const { configs, activeConfigPath, isLoading, loadConfigs, selectConfig, sandboxVars } =
    useServerStore();
  const [tab, setTab] = useState<Tab>("config");

  useEffect(() => {
    loadConfigs();
  }, [loadConfigs]);

  // Auto-select first config if none selected
  useEffect(() => {
    if (configs.length > 0 && !activeConfigPath) {
      selectConfig(configs[0].path);
    }
  }, [configs, activeConfigPath, selectConfig]);

  return (
    <div data-testid="page-server" className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center gap-4 px-6 py-3 border-b border-border">
        <h1 className="text-xl font-semibold">Server</h1>
        <ServerSelector
          configs={configs}
          activeConfig={activeConfigPath}
          onSelect={selectConfig}
        />
      </div>

      {/* Tabs */}
      <div className="flex gap-1 px-6 border-b border-border">
        <button
          data-testid="tab-config"
          onClick={() => setTab("config")}
          className={`px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors ${
            tab === "config"
              ? "border-primary text-foreground"
              : "border-transparent text-muted-foreground hover:text-foreground"
          }`}
        >
          Server Config
        </button>
        <button
          data-testid="tab-sandbox"
          onClick={() => setTab("sandbox")}
          className={`px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors ${
            tab === "sandbox"
              ? "border-primary text-foreground"
              : "border-transparent text-muted-foreground hover:text-foreground"
          }`}
        >
          Sandbox Variables
          {!sandboxVars && (
            <span className="ml-1 text-xs text-muted-foreground/60">(none)</span>
          )}
        </button>
        <button
          data-testid="tab-rcon"
          onClick={() => setTab("rcon")}
          className={`px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors ${
            tab === "rcon"
              ? "border-primary text-foreground"
              : "border-transparent text-muted-foreground hover:text-foreground"
          }`}
        >
          RCON Terminal
        </button>
      </div>

      {/* Content */}
      {isLoading ? (
        <div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
          Loading...
        </div>
      ) : (
        <div className="flex-1 min-h-0">
          {tab === "config" && <ServerConfigEditor />}
          {tab === "sandbox" && <SandboxEditor />}
          {tab === "rcon" && <RconTerminal />}
        </div>
      )}
    </div>
  );
}
