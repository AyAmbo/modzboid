import type { ServerConfigInfo } from "../types";

interface ServerSelectorProps {
  configs: ServerConfigInfo[];
  activeConfig: string | null;
  onSelect: (path: string) => void;
}

export function ServerSelector({ configs, activeConfig, onSelect }: ServerSelectorProps) {
  if (configs.length === 0) {
    return (
      <div className="text-sm text-muted-foreground">
        No server configs found. Configure your Zomboid user directory in Settings.
      </div>
    );
  }

  return (
    <select
      data-testid="server-selector"
      value={activeConfig ?? ""}
      onChange={(e) => onSelect(e.target.value)}
      className="px-3 py-1.5 text-sm bg-muted border border-border rounded"
    >
      <option value="" disabled>Select a server...</option>
      {configs.map((c) => (
        <option key={c.path} value={c.path}>{c.name}</option>
      ))}
    </select>
  );
}
