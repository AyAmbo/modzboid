import { useState, useMemo, useCallback, useEffect } from "react";
import { useKeyboard } from "../hooks/useKeyboard";
import { useLocation } from "wouter";
import Fuse from "fuse.js";
import { cn } from "../lib/utils";

interface Command {
  id: string;
  label: string;
  action: () => void;
}

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [, setLocation] = useLocation();

  const toggleOpen = useCallback(() => {
    setOpen((o) => !o);
    setQuery("");
    setSelectedIndex(0);
  }, []);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  useKeyboard("k", toggleOpen, { ctrl: true });

  const commands: Command[] = useMemo(
    () => [
      {
        id: "mods",
        label: "Go to Mods",
        action: () => {
          setLocation("/mods");
          setOpen(false);
          setQuery("");
        },
      },
      {
        id: "profiles",
        label: "Go to Profiles",
        action: () => {
          setLocation("/profiles");
          setOpen(false);
          setQuery("");
        },
      },
      {
        id: "settings",
        label: "Go to Settings",
        action: () => {
          setLocation("/settings");
          setOpen(false);
          setQuery("");
        },
      },
      {
        id: "server",
        label: "Go to Server",
        action: () => {
          setLocation("/server");
          setOpen(false);
          setQuery("");
        },
      },
      {
        id: "backups",
        label: "Go to Backups",
        action: () => {
          setLocation("/backups");
          setOpen(false);
          setQuery("");
        },
      },
      {
        id: "graph",
        label: "Go to Dependency Graph",
        action: () => {
          setLocation("/graph");
          setOpen(false);
          setQuery("");
        },
      },
      {
        id: "extensions",
        label: "Go to Extensions",
        action: () => {
          setLocation("/extensions");
          setOpen(false);
          setQuery("");
        },
      },
      {
        id: "diagnostics",
        label: "Go to Diagnostics",
        action: () => {
          setLocation("/diagnostics");
          setOpen(false);
          setQuery("");
        },
      },
    ],
    [setLocation]
  );

  const fuse = useMemo(
    () => new Fuse(commands, { keys: ["label"], threshold: 0.4 }),
    [commands]
  );

  const results = query ? fuse.search(query).map((r) => r.item) : commands;

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]"
      onClick={() => {
        setOpen(false);
        setQuery("");
      }}
    >
      <div
        className="bg-card text-card-foreground border border-border rounded-lg shadow-xl w-96"
        onClick={(e) => e.stopPropagation()}
      >
        <input
          autoFocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setSelectedIndex((i) => Math.max(i - 1, 0));
            } else if (e.key === "Enter" && results[selectedIndex]) {
              e.preventDefault();
              results[selectedIndex].action();
            } else if (e.key === "Escape") {
              setOpen(false);
              setQuery("");
            }
          }}
          placeholder="Type a command..."
          className="w-full px-4 py-3 bg-transparent border-b border-border text-sm outline-none text-foreground placeholder:text-muted-foreground"
        />
        <div className="max-h-64 overflow-y-auto">
          {results.length === 0 ? (
            <div className="px-4 py-3 text-sm text-muted-foreground">
              No commands found
            </div>
          ) : (
            results.map((cmd, idx) => (
              <button
                key={cmd.id}
                onClick={cmd.action}
                className={cn(
                  "w-full text-left px-4 py-2 text-sm text-card-foreground",
                  idx === selectedIndex ? "bg-muted" : "hover:bg-muted/50"
                )}
              >
                {cmd.label}
              </button>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
