import { Link, useRoute, useLocation } from "wouter";
import { cn } from "../shared/lib/utils";
import { ProfileSwitcher } from "../features/profiles/components/ProfileSwitcher";
import { useModManagerStore } from "../features/mod-manager/store";
import { useActiveProfile } from "../features/profiles/store";
import { useAppStore } from "../shared/stores/appStore";

const PAGE_TITLES: Record<string, string> = {
  "/mods": "Mod Manager",
  "/profiles": "Profiles",
  "/settings": "Settings",
  "/server": "Server",
  "/backups": "Backups",
  "/graph": "Dependency Graph",
  "/compatibility": "Mod Compatibility",
  "/diagnostics": "Diagnostics",
  "/extensions": "Extensions",
  "/api-docs": "API Docs",
};

function PageTitle() {
  const [location] = useLocation();
  const title = PAGE_TITLES[location] ?? "Project Modzboid";
  return <h1 className="text-sm font-medium">{title}</h1>;
}

export function Layout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex h-full bg-background text-foreground">
      {/* Sidebar */}
      <aside data-testid="sidebar" className="w-60 bg-card border-r border-border flex flex-col">
        <div className="p-4 font-bold text-lg">Project Modzboid</div>
        <nav className="flex-1 px-2 space-y-1">
          <NavLink href="/mods" label="Mods" />
          <NavLink href="/profiles" label="Profiles" />
          <NavLink href="/settings" label="Settings" />
          <NavLink href="/server" label="Server" />
          <NavLink href="/backups" label="Backups" />
          <NavLink href="/graph" label="Graph" />
          <NavLink href="/compatibility" label="Compatibility" />
          <NavLink href="/diagnostics" label="Diagnostics" />
          <NavLink href="/extensions" label="Extensions" />
          <NavLink href="/api-docs" label="API Docs" />
        </nav>
        <div className="p-3 border-t border-border" data-testid="profile-switcher">
          <ProfileSwitcher />
        </div>
      </aside>

      {/* Main area */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Top bar */}
        <header data-testid="header" className="h-12 border-b border-border flex items-center px-4 justify-between">
          <PageTitle />
          <button
            data-testid="cmd-palette-btn"
            className="text-xs text-muted-foreground px-2 py-1 border border-border rounded hover:bg-muted"
            title="Command Palette (Ctrl+K)"
            onClick={() => {
              window.dispatchEvent(
                new KeyboardEvent("keydown", {
                  key: "k",
                  ctrlKey: true,
                  bubbles: true,
                })
              );
            }}
          >
            Ctrl+K
          </button>
        </header>

        {/* Content */}
        <main className="flex-1 overflow-auto">{children}</main>

        {/* Status bar */}
        <StatusBar />
      </div>
    </div>
  );
}

function StatusBar() {
  const { allMods, issues } = useModManagerStore();
  const activeProfile = useActiveProfile();
  const { gameRunning } = useAppStore();

  const enabledCount = activeProfile?.loadOrder.length ?? 0;
  const totalCount = allMods.length;
  const errorCount = issues.filter((i) => i.severity === "error").length;
  const warningCount = issues.filter((i) => i.severity === "warning").length;

  let healthIcon = "✅";
  let healthText = "No issues";
  if (errorCount > 0) {
    healthIcon = "❌";
    healthText = `${errorCount} error${errorCount > 1 ? "s" : ""}`;
  } else if (warningCount > 0) {
    healthIcon = "⚠️";
    healthText = `${warningCount} warning${warningCount > 1 ? "s" : ""}`;
  }

  return (
    <footer data-testid="status-bar" className="h-7 border-t border-border flex items-center px-4 text-xs text-muted-foreground gap-4">
      <span data-testid="status-health">
        {healthIcon} {healthText}
      </span>
      <span data-testid="status-mods">
        {enabledCount}/{totalCount} mods
      </span>
      <span data-testid="status-profile">{activeProfile?.name ?? "No profile"}</span>
      {gameRunning && <span className="text-green-500">Game running</span>}
      <span className="ml-auto">v0.1.0</span>
    </footer>
  );
}

function NavLink({ href, label }: { href: string; label: string }) {
  const [isActive] = useRoute(href);
  return (
    <Link
      href={href}
      data-testid={`nav-${href.slice(1)}`}
      className={cn(
        "block px-3 py-2 rounded text-sm",
        isActive
          ? "bg-primary text-primary-foreground"
          : "text-muted-foreground hover:bg-muted"
      )}
    >
      {label}
    </Link>
  );
}
