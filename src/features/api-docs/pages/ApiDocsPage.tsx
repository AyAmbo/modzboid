import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as shellOpen } from "@tauri-apps/plugin-shell";
import { Button } from "../../../shared/components/ui/button";

/* ------------------------------------------------------------------ */
/* Types                                                               */
/* ------------------------------------------------------------------ */

interface ExtensionInfo {
  id: string;
  name: string;
  version: string;
  enabled: boolean;
  path: string;
}

/* ------------------------------------------------------------------ */
/* Main Page                                                           */
/* ------------------------------------------------------------------ */

export default function ApiDocsPage() {
  const [docsExtension, setDocsExtension] = useState<ExtensionInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [iframeError, setIframeError] = useState(false);

  useEffect(() => {
    invoke<ExtensionInfo[]>("list_extensions_cmd")
      .then((list) => {
        const ext = list.find((e) => e.id === "pz-api-docs" && e.enabled);
        setDocsExtension(ext ?? null);
      })
      .catch((err) => console.error("Failed to check extensions:", err))
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div data-testid="page-api-docs" className="p-6 flex items-center justify-center h-full">
        <p className="text-sm text-muted-foreground">Loading...</p>
      </div>
    );
  }

  // Extension installed — show docs in iframe via asset protocol
  if (docsExtension) {
    const docsFilePath = `${docsExtension.path}/docs/index.html`;
    // We can NOT use convertFileSrc here — it URL-encodes the path, turning
    // / into %2F and \ into %5C. The browser doesn't treat encoded chars as
    // path separators, so relative CSS/JS references in the iframe resolve
    // to http://asset.localhost/_static/... (root) instead of the docs subdir.
    // Using raw forward slashes lets relative URLs resolve correctly.
    const docsUrl = "http://asset.localhost/" + docsFilePath.replace(/\\/g, "/");

    const handleOpenInBrowser = async () => {
      try {
        const normalized = docsFilePath.replace(/\\/g, "/");
        const fileUrl = normalized.startsWith("/")
          ? "file://" + normalized
          : "file:///" + normalized;
        await shellOpen(fileUrl);
      } catch {
        await shellOpen(docsFilePath).catch(() => {});
      }
    };

    if (iframeError) {
      return (
        <div data-testid="page-api-docs" className="p-6 flex items-center justify-center h-full">
          <div className="max-w-md text-center space-y-4">
            <h1 className="text-xl font-semibold">API Documentation</h1>
            <p className="text-sm text-muted-foreground">
              Could not load docs inside the app.
            </p>
            <Button onClick={handleOpenInBrowser}>Open in Browser</Button>
          </div>
        </div>
      );
    }

    return (
      <div data-testid="page-api-docs" className="flex flex-col h-full">
        {/* Minimal toolbar */}
        <div className="px-4 py-1.5 border-b border-border flex items-center gap-3 bg-muted/30 shrink-0">
          <span className="text-xs text-muted-foreground truncate flex-1">
            {docsExtension.name}
          </span>
          <Button variant="ghost" size="sm" className="text-xs h-6" onClick={handleOpenInBrowser}>
            Open in Browser
          </Button>
        </div>

        {/* Docs iframe */}
        <iframe
          src={docsUrl}
          title="PZ API Documentation"
          className="flex-1 w-full border-0"
          onError={() => setIframeError(true)}
        />
      </div>
    );
  }

  // No extension — show install instructions
  return (
    <div data-testid="page-api-docs" className="p-6 flex items-center justify-center h-full">
      <div className="max-w-md text-center space-y-4">
        <h1 className="text-xl font-semibold">API Documentation</h1>
        <p className="text-sm text-muted-foreground">
          No documentation extension installed.
        </p>
        <div className="border border-border rounded-lg bg-card p-4 text-left space-y-2">
          <h3 className="text-sm font-medium">How to install</h3>
          <ol className="text-xs text-muted-foreground space-y-1 list-decimal list-inside">
            <li>Build the docs with <span className="font-mono bg-muted px-1 rounded">pz-api-extractor</span></li>
            <li>Go to the <span className="font-medium text-foreground">Extensions</span> tab</li>
            <li>Click <span className="font-medium text-foreground">Install Extension</span></li>
            <li>Select the <span className="font-mono bg-muted px-1 rounded">docs-extension</span> folder</li>
          </ol>
        </div>
      </div>
    </div>
  );
}
