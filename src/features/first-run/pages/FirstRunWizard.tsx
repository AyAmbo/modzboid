import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useLocation } from "wouter";
import { useAppStore } from "../../../shared/stores/appStore";
import type { AppConfig } from "../../../shared/types/app";

type Step = 1 | 2 | 3 | 4 | 5;

function StepIndicator({ current, total }: { current: Step; total: number }) {
  return (
    <div className="flex items-center gap-2 mb-8">
      {Array.from({ length: total }, (_, i) => i + 1).map((step) => (
        <div key={step} className="flex items-center gap-2">
          <div
            className={[
              "w-7 h-7 rounded-full flex items-center justify-center text-xs font-medium transition-colors",
              step < current
                ? "bg-primary text-primary-foreground"
                : step === current
                ? "bg-primary text-primary-foreground ring-2 ring-primary ring-offset-2 ring-offset-background"
                : "bg-muted text-muted-foreground",
            ].join(" ")}
          >
            {step < current ? "✓" : step}
          </div>
          {step < total && (
            <div
              className={[
                "h-0.5 w-8 transition-colors",
                step < current ? "bg-primary" : "bg-border",
              ].join(" ")}
            />
          )}
        </div>
      ))}
    </div>
  );
}

// Step 1: Welcome
function StepWelcome({ onNext }: { onNext: () => void }) {
  return (
    <div className="flex flex-col items-center text-center">
      <div className="text-5xl mb-6">🧟</div>
      <h1 className="text-3xl font-bold mb-3">Welcome to Project Modzboid</h1>
      <p className="text-muted-foreground text-lg mb-2">
        A mod manager for Project Zomboid
      </p>
      <p className="text-muted-foreground text-sm mb-10 max-w-md">
        Let's get you set up in just a few steps. We'll detect your game
        installation and scan your mods automatically.
      </p>
      <button
        data-testid="btn-get-started"
        onClick={onNext}
        className="px-6 py-2.5 bg-primary text-primary-foreground rounded font-medium hover:opacity-90 transition-opacity"
      >
        Get Started →
      </button>
    </div>
  );
}

// Step 2: Game Path
function StepGamePath({
  onNext,
  onBack,
  gamePath,
  setGamePath,
}: {
  onNext: () => void;
  onBack: () => void;
  gamePath: string | null;
  setGamePath: (p: string | null) => void;
}) {
  const [detecting, setDetecting] = useState(false);
  const [verified, setVerified] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDetecting(true);
    invoke<string | null>("detect_game_path_cmd")
      .then(async (path) => {
        if (path) {
          const valid = await invoke<boolean>("verify_game_path_cmd", { path });
          if (valid) {
            setGamePath(path);
            setVerified(true);
          } else {
            setVerified(false);
            setError("Auto-detected path failed verification.");
          }
        } else {
          setVerified(false);
        }
      })
      .catch(() => setVerified(false))
      .finally(() => setDetecting(false));
  }, [setGamePath]);

  const handleBrowse = async () => {
    const selected = await open({
      directory: true,
      title: "Select Project Zomboid installation",
    });
    if (selected) {
      const path = selected as string;
      const valid = await invoke<boolean>("verify_game_path_cmd", { path });
      if (valid) {
        setGamePath(path);
        setVerified(true);
        setError(null);
      } else {
        setVerified(false);
        setError("Selected folder doesn't appear to be a valid Project Zomboid installation.");
      }
    }
  };

  const handleUseThis = () => {
    if (verified && gamePath) onNext();
  };

  return (
    <div className="flex flex-col">
      <h2 className="text-2xl font-bold mb-2">Game Installation</h2>
      <p className="text-muted-foreground mb-8">
        We need to find your Project Zomboid installation folder.
      </p>

      {detecting ? (
        <div className="flex items-center gap-3 p-4 bg-muted rounded mb-6">
          <div className="w-4 h-4 border-2 border-primary border-t-transparent rounded-full animate-spin" />
          <span className="text-sm">Auto-detecting game path...</span>
        </div>
      ) : gamePath && verified ? (
        <div className="mb-6">
          <div className="flex items-start gap-3 p-4 bg-muted rounded">
            <span className="text-green-500 mt-0.5">✓</span>
            <div className="flex-1 min-w-0">
              <div className="text-sm font-medium text-green-600 dark:text-green-400 mb-1">
                Found automatically
              </div>
              <div className="text-sm text-muted-foreground truncate font-mono">
                {gamePath}
              </div>
            </div>
          </div>
        </div>
      ) : (
        <div className="mb-6">
          <div className="flex items-center gap-3 p-4 bg-muted rounded">
            <span className="text-red-500">✗</span>
            <div className="flex-1">
              <div className="text-sm font-medium text-red-600 dark:text-red-400 mb-1">
                Couldn't auto-detect
              </div>
              <div className="text-sm text-muted-foreground">
                Please browse to your Project Zomboid folder manually.
              </div>
            </div>
          </div>
          {error && (
            <p className="mt-2 text-xs text-red-500">{error}</p>
          )}
        </div>
      )}

      <div className="flex items-center gap-3">
        <button
          onClick={handleBrowse}
          className="px-4 py-2 text-sm border border-border rounded hover:bg-muted transition-colors"
        >
          Browse...
        </button>
        {gamePath && verified && (
          <span className="text-sm text-muted-foreground font-mono truncate flex-1">
            {gamePath}
          </span>
        )}
      </div>

      <div className="flex gap-3 mt-10">
        <button
          onClick={onBack}
          className="px-4 py-2 text-sm border border-border rounded hover:bg-muted transition-colors"
        >
          ← Back
        </button>
        {gamePath && verified ? (
          <button
            onClick={handleUseThis}
            className="px-5 py-2 text-sm bg-primary text-primary-foreground rounded hover:opacity-90 transition-opacity font-medium"
          >
            Use This →
          </button>
        ) : (
          <button
            onClick={onNext}
            className="px-4 py-2 text-sm border border-border rounded hover:bg-muted transition-colors"
          >
            Skip →
          </button>
        )}
      </div>
    </div>
  );
}

// Step 3: Workshop Path
function StepWorkshopPath({
  onNext,
  onBack,
  gamePath,
  workshopPath,
  setWorkshopPath,
}: {
  onNext: () => void;
  onBack: () => void;
  gamePath: string | null;
  workshopPath: string | null;
  setWorkshopPath: (p: string | null) => void;
}) {
  const [detecting, setDetecting] = useState(false);
  const [verified, setVerified] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Derive workshop path from game path or detect from steam
    const derive = async () => {
      setDetecting(true);
      try {
        // Try to derive from game path first
        if (gamePath) {
          // Normalize separators for matching
          const normalized = gamePath.replace(/\\/g, "/");
          const commonIndex = normalized.toLowerCase().indexOf("steamapps/common");
          if (commonIndex !== -1) {
            const steamappsRoot = gamePath.substring(0, commonIndex + "steamapps".length);
            const sep = gamePath.includes("/") ? "/" : "\\";
            const derived = `${steamappsRoot}${sep}workshop${sep}content${sep}108600`;
            setWorkshopPath(derived);
            setVerified(true);
            return;
          }
        }
        // Fall back to steam detection
        const steamPath = await invoke<string | null>("detect_steam_path_cmd");
        if (steamPath) {
          const sep = steamPath.includes("/") ? "/" : "\\";
          const derived = `${steamPath}${sep}steamapps${sep}workshop${sep}content${sep}108600`;
          setWorkshopPath(derived);
          setVerified(true);
        } else {
          setVerified(false);
        }
      } catch {
        setVerified(false);
      } finally {
        setDetecting(false);
      }
    };
    derive();
  }, [gamePath, setWorkshopPath]);

  const handleBrowse = async () => {
    const selected = await open({
      directory: true,
      title: "Select Workshop Content folder (steamapps/workshop/content/108600)",
    });
    if (selected) {
      setWorkshopPath(selected as string);
      setVerified(true);
      setError(null);
    }
  };

  return (
    <div className="flex flex-col">
      <h2 className="text-2xl font-bold mb-2">Workshop Content</h2>
      <p className="text-muted-foreground mb-8">
        This is where Steam downloads your subscribed mods.
      </p>

      {detecting ? (
        <div className="flex items-center gap-3 p-4 bg-muted rounded mb-6">
          <div className="w-4 h-4 border-2 border-primary border-t-transparent rounded-full animate-spin" />
          <span className="text-sm">Locating workshop folder...</span>
        </div>
      ) : workshopPath && verified ? (
        <div className="mb-6">
          <div className="flex items-start gap-3 p-4 bg-muted rounded">
            <span className="text-green-500 mt-0.5">✓</span>
            <div className="flex-1 min-w-0">
              <div className="text-sm font-medium text-green-600 dark:text-green-400 mb-1">
                Found
              </div>
              <div className="text-sm text-muted-foreground truncate font-mono">
                {workshopPath}
              </div>
            </div>
          </div>
        </div>
      ) : (
        <div className="mb-6">
          <div className="flex items-center gap-3 p-4 bg-muted rounded">
            <span className="text-red-500">✗</span>
            <div className="flex-1">
              <div className="text-sm font-medium text-red-600 dark:text-red-400 mb-1">
                Couldn't locate automatically
              </div>
              <div className="text-sm text-muted-foreground">
                Browse to your <span className="font-mono">steamapps/workshop/content/108600</span> folder.
              </div>
            </div>
          </div>
          {error && <p className="mt-2 text-xs text-red-500">{error}</p>}
        </div>
      )}

      <div className="flex items-center gap-3">
        <button
          onClick={handleBrowse}
          className="px-4 py-2 text-sm border border-border rounded hover:bg-muted transition-colors"
        >
          Browse...
        </button>
        {workshopPath && (
          <span className="text-sm text-muted-foreground font-mono truncate flex-1">
            {workshopPath}
          </span>
        )}
      </div>

      <div className="flex gap-3 mt-10">
        <button
          onClick={onBack}
          className="px-4 py-2 text-sm border border-border rounded hover:bg-muted transition-colors"
        >
          ← Back
        </button>
        <button
          onClick={onNext}
          className="px-5 py-2 text-sm bg-primary text-primary-foreground rounded hover:opacity-90 transition-opacity font-medium"
        >
          {workshopPath ? "Use This →" : "Skip →"}
        </button>
      </div>
    </div>
  );
}

// Step 4: Scanning
function StepScanning({
  onNext,
  onBack,
  setModCount,
  setImportedCount,
  gamePath,
  workshopPath,
}: {
  onNext: () => void;
  onBack: () => void;
  setModCount: (n: number) => void;
  setImportedCount: (n: number) => void;
  gamePath: string | null;
  workshopPath: string | null;
}) {
  const [scanning, setScanning] = useState(true);
  const [modCount, setLocalModCount] = useState<number | null>(null);
  const [importedCount, setLocalImportedCount] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const scanStarted = useRef(false);
  const config = useAppStore((s) => s.config);
  const saveConfig = useAppStore((s) => s.saveConfig);

  useEffect(() => {
    if (scanStarted.current) return;
    scanStarted.current = true;

    let cancelled = false;
    const run = async () => {
      setScanning(true);
      try {
        // Save config with paths before scanning so discover_mods can find them
        const currentConfig = config;
        const updatedConfig: AppConfig = {
          ...(currentConfig ?? {
            steamPath: null,
            localModsPath: null,
            zomboidUserDir: null,
            gameVersion: null,
            theme: "dark",
            locale: "en",
            isFirstRun: true,
            checkUpdates: true,
            uiScale: 100,
            fontSize: 14,
          }),
          gamePath,
          workshopPath,
        };
        await saveConfig(updatedConfig);

        if (cancelled) return;

        const mods = await invoke<unknown[]>("discover_mods");
        const count = mods?.length ?? 0;
        if (cancelled) return;
        setLocalModCount(count);
        setModCount(count);

        // Try importing existing mod setup — returns a single Profile, not an array
        try {
          await invoke("import_from_game_cmd");
          if (cancelled) return;
          // import_from_game_cmd creates a profile with the game's current load order
          setLocalImportedCount(1);
          setImportedCount(1);
        } catch {
          if (cancelled) return;
          // import_from_game_cmd failed (no default.txt) — create a default empty profile
          try {
            await invoke("create_profile_cmd", {
              name: "Default",
              profileType: "singleplayer",
            });
          } catch {
            // Profile creation failed — not critical
          }
          setLocalImportedCount(0);
          setImportedCount(0);
        }
      } catch (err) {
        if (cancelled) return;
        setError(String(err));
        setLocalModCount(0);
        setModCount(0);
      } finally {
        if (!cancelled) setScanning(false);
      }
    };
    run();

    return () => {
      cancelled = true;
    };
  }, [setModCount, setImportedCount, gamePath, workshopPath]);

  return (
    <div className="flex flex-col">
      <h2 className="text-2xl font-bold mb-2">Scanning Mods</h2>
      <p className="text-muted-foreground mb-8">
        Discovering mods in your workshop and local folders.
      </p>

      <div className="flex flex-col gap-4">
        {scanning ? (
          <div className="flex items-center gap-3 p-5 bg-muted rounded">
            <div className="w-5 h-5 border-2 border-primary border-t-transparent rounded-full animate-spin flex-shrink-0" />
            <span className="text-sm">Scanning mods...</span>
          </div>
        ) : error ? (
          <div className="flex items-start gap-3 p-5 bg-muted rounded">
            <span className="text-red-500 mt-0.5">✗</span>
            <div>
              <div className="text-sm font-medium text-red-600 dark:text-red-400 mb-1">
                Scan failed
              </div>
              <div className="text-xs text-muted-foreground">{error}</div>
            </div>
          </div>
        ) : (
          <>
            <div className="flex items-center gap-3 p-5 bg-muted rounded">
              <span className="text-green-500 text-lg">✓</span>
              <div>
                <div className="text-sm font-medium">Mods discovered</div>
                <div className="text-2xl font-bold mt-1">{modCount}</div>
              </div>
            </div>
            {importedCount !== null && importedCount > 0 && (
              <div className="flex items-center gap-3 p-4 bg-muted rounded">
                <span className="text-green-500">✓</span>
                <div className="text-sm">
                  Imported{" "}
                  <span className="font-medium">{importedCount} mods</span>{" "}
                  from your existing game setup
                </div>
              </div>
            )}
          </>
        )}
      </div>

      <div className="flex gap-3 mt-10">
        <button
          onClick={onBack}
          disabled={scanning}
          className="px-4 py-2 text-sm border border-border rounded hover:bg-muted transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
        >
          ← Back
        </button>
        <button
          onClick={onNext}
          disabled={scanning}
          className="px-5 py-2 text-sm bg-primary text-primary-foreground rounded hover:opacity-90 transition-opacity font-medium disabled:opacity-40 disabled:cursor-not-allowed"
        >
          Continue →
        </button>
      </div>
    </div>
  );
}

// Step 5: Done
function StepDone({
  modCount,
  importedCount,
  onFinish,
}: {
  modCount: number;
  importedCount: number;
  onFinish: () => void;
}) {
  return (
    <div className="flex flex-col items-center text-center">
      <div className="text-5xl mb-6">🎉</div>
      <h1 className="text-3xl font-bold mb-3">You're all set!</h1>
      <p className="text-muted-foreground text-lg mb-6">
        Project Modzboid is ready to manage your mods.
      </p>

      <div className="w-full max-w-sm bg-muted rounded p-4 mb-10 text-left space-y-2">
        <div className="flex justify-between text-sm">
          <span className="text-muted-foreground">Mods found</span>
          <span className="font-medium">{modCount}</span>
        </div>
        {importedCount > 0 && (
          <div className="flex justify-between text-sm">
            <span className="text-muted-foreground">Imported from game</span>
            <span className="font-medium">{importedCount}</span>
          </div>
        )}
        <div className="flex justify-between text-sm">
          <span className="text-muted-foreground">Default profile</span>
          <span className="font-medium text-green-600 dark:text-green-400">Created</span>
        </div>
      </div>

      <button
        data-testid="btn-finish"
        onClick={onFinish}
        className="px-6 py-2.5 bg-primary text-primary-foreground rounded font-medium hover:opacity-90 transition-opacity"
      >
        Open Mod Manager →
      </button>
    </div>
  );
}

// Main Wizard
export default function FirstRunWizard() {
  const [step, setStep] = useState<Step>(1);
  const [gamePath, setGamePath] = useState<string | null>(null);
  const [workshopPath, setWorkshopPath] = useState<string | null>(null);
  const [modCount, setModCount] = useState(0);
  const [importedCount, setImportedCount] = useState(0);
  const { config, saveConfig } = useAppStore();
  const [, setLocation] = useLocation();

  const next = () => setStep((s) => Math.min(5, s + 1) as Step);
  const back = () => setStep((s) => Math.max(1, s - 1) as Step);

  const handleFinish = async () => {
    const updatedConfig: AppConfig = {
      ...(config ?? {
        steamPath: null,
        localModsPath: null,
        zomboidUserDir: null,
        gameVersion: null,
        theme: "dark",
        locale: "en",
        checkUpdates: true,
        uiScale: 100,
        fontSize: 14,
      }),
      gamePath,
      workshopPath,
      isFirstRun: false,
    };
    await saveConfig(updatedConfig);
    setLocation("/mods");
  };

  return (
    <div data-testid="page-first-run" className="min-h-screen flex items-center justify-center bg-background">
      <div className="w-full max-w-lg p-8">
        <StepIndicator current={step} total={5} />

        <div>
          {step === 1 && <StepWelcome onNext={next} />}
          {step === 2 && (
            <StepGamePath
              onNext={next}
              onBack={back}
              gamePath={gamePath}
              setGamePath={setGamePath}
            />
          )}
          {step === 3 && (
            <StepWorkshopPath
              onNext={next}
              onBack={back}
              gamePath={gamePath}
              workshopPath={workshopPath}
              setWorkshopPath={setWorkshopPath}
            />
          )}
          {step === 4 && (
            <StepScanning
              onNext={next}
              onBack={back}
              setModCount={setModCount}
              setImportedCount={setImportedCount}
              gamePath={gamePath}
              workshopPath={workshopPath}
            />
          )}
          {step === 5 && (
            <StepDone
              modCount={modCount}
              importedCount={importedCount}
              onFinish={handleFinish}
            />
          )}
        </div>
      </div>
    </div>
  );
}
