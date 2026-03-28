/** @deprecated This component is no longer used by ApiDocsPage. Docs are now rendered in-browser or via iframe. */
import { useState, useMemo } from "react";
import { cn } from "../../../shared/lib/utils";

export interface ApiEventParam {
  name: string;
  paramType: string;
  description: string;
}

export interface ApiEventInfo {
  name: string;
  description: string;
  params: ApiEventParam[];
  contexts: string[]; // e.g. ["Client", "Server", "Multiplayer"]
  usageExample: string | null;
}

interface Props {
  events: ApiEventInfo[];
}

const CONTEXT_COLORS: Record<string, string> = {
  Client: "bg-green-500/20 text-green-400 border-green-500/30",
  Server: "bg-orange-500/20 text-orange-400 border-orange-500/30",
  Multiplayer: "bg-blue-500/20 text-blue-400 border-blue-500/30",
};

export function EventList({ events }: Props) {
  const [search, setSearch] = useState("");
  const [contextFilter, setContextFilter] = useState<string>("all");
  const [expandedEvents, setExpandedEvents] = useState<Set<string>>(new Set());

  const allContexts = useMemo(() => {
    const set = new Set<string>();
    events.forEach((e) => e.contexts.forEach((c) => set.add(c)));
    return Array.from(set).sort();
  }, [events]);

  const filtered = useMemo(() => {
    return events.filter((e) => {
      const matchSearch =
        !search ||
        e.name.toLowerCase().includes(search.toLowerCase()) ||
        e.description.toLowerCase().includes(search.toLowerCase());
      const matchContext =
        contextFilter === "all" || e.contexts.includes(contextFilter);
      return matchSearch && matchContext;
    });
  }, [events, search, contextFilter]);

  const toggleEvent = (name: string) => {
    setExpandedEvents((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  };

  return (
    <div className="flex flex-col gap-3">
      {/* Filters */}
      <div className="flex gap-2 flex-wrap">
        <input
          type="text"
          placeholder="Filter events..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="flex-1 min-w-40 px-3 py-1.5 text-sm bg-background border border-border rounded focus:outline-none focus:ring-1 focus:ring-primary"
        />
        <div className="flex gap-1">
          <button
            onClick={() => setContextFilter("all")}
            className={cn(
              "px-2.5 py-1.5 text-xs rounded border",
              contextFilter === "all"
                ? "bg-primary text-primary-foreground border-primary"
                : "border-border text-muted-foreground hover:bg-muted"
            )}
          >
            All
          </button>
          {allContexts.map((ctx) => (
            <button
              key={ctx}
              onClick={() => setContextFilter(ctx)}
              className={cn(
                "px-2.5 py-1.5 text-xs rounded border",
                contextFilter === ctx
                  ? "bg-primary text-primary-foreground border-primary"
                  : "border-border text-muted-foreground hover:bg-muted"
              )}
            >
              {ctx}
            </button>
          ))}
        </div>
      </div>

      {/* Count */}
      <p className="text-xs text-muted-foreground">
        {filtered.length} event{filtered.length !== 1 ? "s" : ""}
        {search || contextFilter !== "all" ? " (filtered)" : ""}
      </p>

      {/* Event list */}
      <div className="space-y-1">
        {filtered.map((event) => {
          const isExpanded = expandedEvents.has(event.name);
          return (
            <div
              key={event.name}
              className="border border-border rounded-lg bg-card"
            >
              <button
                onClick={() => toggleEvent(event.name)}
                className="w-full text-left px-3 py-2.5 flex items-start gap-3"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="font-mono text-sm font-medium">{event.name}</span>
                    {event.contexts.map((ctx) => (
                      <span
                        key={ctx}
                        className={cn(
                          "px-1.5 py-0.5 rounded text-xs border",
                          CONTEXT_COLORS[ctx] ??
                            "bg-gray-500/20 text-gray-400 border-gray-500/30"
                        )}
                      >
                        {ctx}
                      </span>
                    ))}
                  </div>
                  {event.description && !isExpanded && (
                    <p className="text-xs text-muted-foreground mt-0.5 truncate">
                      {event.description}
                    </p>
                  )}
                </div>
                <span className="text-muted-foreground text-xs shrink-0 mt-0.5">
                  {isExpanded ? "▲" : "▼"}
                </span>
              </button>

              {isExpanded && (
                <div className="px-3 pb-3 space-y-3 border-t border-border">
                  {event.description && (
                    <p className="text-sm text-muted-foreground pt-2">
                      {event.description}
                    </p>
                  )}

                  {event.params.length > 0 && (
                    <div>
                      <h4 className="text-xs font-medium mb-1.5">Parameters</h4>
                      <div className="space-y-1">
                        {event.params.map((param) => (
                          <div
                            key={param.name}
                            className="flex gap-3 text-xs items-start"
                          >
                            <span className="font-mono text-blue-400 shrink-0">
                              {param.name}
                            </span>
                            <span className="text-muted-foreground shrink-0">
                              {param.paramType}
                            </span>
                            {param.description && (
                              <span className="text-muted-foreground">
                                {param.description}
                              </span>
                            )}
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {event.usageExample && (
                    <div>
                      <h4 className="text-xs font-medium mb-1.5">Usage</h4>
                      <pre className="text-xs bg-muted p-2.5 rounded overflow-x-auto font-mono whitespace-pre-wrap">
                        {event.usageExample}
                      </pre>
                    </div>
                  )}

                  {event.params.length === 0 && !event.usageExample && (
                    <p className="text-xs text-muted-foreground pt-2">
                      No additional details.
                    </p>
                  )}
                </div>
              )}
            </div>
          );
        })}

        {filtered.length === 0 && (
          <p className="text-sm text-muted-foreground text-center py-8">
            No events match your filter.
          </p>
        )}
      </div>
    </div>
  );
}
