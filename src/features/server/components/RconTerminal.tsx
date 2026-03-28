import { useState, useRef, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../../../shared/components/ui/button";
import { Input } from "../../../shared/components/ui/input";
import { useServerStore } from "../store";

interface TerminalLine {
  type: "input" | "output" | "error" | "system";
  text: string;
  timestamp: string;
}

type PromptTarget = "servermsg" | "kick" | null;

export function RconTerminal() {
  const serverConfig = useServerStore((s) => s.serverConfig);
  const [host, setHost] = useState("127.0.0.1");
  const [port, setPort] = useState("27015");
  const [password, setPassword] = useState("");
  const [connected, setConnected] = useState(false);
  const [command, setCommand] = useState("");
  const [lines, setLines] = useState<TerminalLine[]>([]);
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [promptTarget, setPromptTarget] = useState<PromptTarget>(null);
  const [promptValue, setPromptValue] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const promptRef = useRef<HTMLInputElement>(null);

  // Auto-fill port/password from server config
  useEffect(() => {
    if (!serverConfig) return;
    const rconPort = serverConfig.settings.find((s) => s.key === "RCONPort");
    const rconPass = serverConfig.settings.find((s) => s.key === "RCONPassword");
    if (rconPort) setPort(rconPort.value);
    if (rconPass) setPassword(rconPass.value);
  }, [serverConfig]);

  // Auto-scroll to bottom
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [lines]);

  // Focus prompt input when it opens
  useEffect(() => {
    if (promptTarget && promptRef.current) {
      promptRef.current.focus();
    }
  }, [promptTarget]);

  const addLine = useCallback((type: TerminalLine["type"], text: string) => {
    setLines((prev) => [
      ...prev,
      { type, text, timestamp: new Date().toLocaleTimeString() },
    ]);
  }, []);

  const handleConnect = useCallback(async () => {
    addLine("system", `Connecting to ${host}:${port}...`);
    try {
      await invoke("rcon_test_cmd", {
        host,
        port: parseInt(port),
        password,
      });
      setConnected(true);
      addLine("system", "Connected successfully. Type commands below.");
    } catch (err) {
      addLine("error", `Connection failed: ${err}`);
    }
  }, [host, port, password, addLine]);

  const handleDisconnect = useCallback(() => {
    setConnected(false);
    addLine("system", "Disconnected.");
  }, [addLine]);

  const sendCommand = useCallback(async (cmd: string) => {
    if (!cmd.trim()) return;

    addLine("input", `> ${cmd}`);
    setHistory((prev) => [...prev, cmd]);
    setHistoryIndex(-1);

    try {
      const result = await invoke<{ success: boolean; response: string }>(
        "rcon_command_cmd",
        { host, port: parseInt(port), password, command: cmd }
      );
      if (result.response.trim()) {
        addLine("output", result.response.trim());
      } else {
        addLine("output", "(no response)");
      }
    } catch (err) {
      addLine("error", `Error: ${err}`);
    }
  }, [host, port, password, addLine]);

  const handleSend = useCallback(async () => {
    if (!command.trim()) return;
    const cmd = command.trim();
    setCommand("");
    await sendCommand(cmd);
  }, [command, sendCommand]);

  const handleQuickCommand = useCallback((cmd: string) => {
    sendCommand(cmd);
  }, [sendCommand]);

  const handlePromptSubmit = useCallback(() => {
    if (!promptValue.trim()) return;
    const value = promptValue.trim();
    setPromptValue("");
    if (promptTarget === "servermsg") {
      sendCommand(`servermsg "${value}"`);
    } else if (promptTarget === "kick") {
      sendCommand(`kick "${value}"`);
    }
    setPromptTarget(null);
  }, [promptTarget, promptValue, sendCommand]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSend();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (history.length > 0) {
        const newIndex = historyIndex === -1 ? history.length - 1 : Math.max(0, historyIndex - 1);
        setHistoryIndex(newIndex);
        setCommand(history[newIndex]);
      }
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      if (historyIndex >= 0) {
        const newIndex = historyIndex + 1;
        if (newIndex >= history.length) {
          setHistoryIndex(-1);
          setCommand("");
        } else {
          setHistoryIndex(newIndex);
          setCommand(history[newIndex]);
        }
      }
    }
  };

  const lineColors: Record<string, string> = {
    input: "text-blue-400",
    output: "text-foreground",
    error: "text-red-400",
    system: "text-yellow-400",
  };

  return (
    <div className="flex flex-col h-full">
      {/* Connection bar */}
      <div className="flex items-center gap-2 px-3 py-2 border-b border-border bg-card">
        <Input
          placeholder="Host"
          value={host}
          onChange={(e) => setHost(e.target.value)}
          className="w-32 h-7 text-xs font-mono"
          disabled={connected}
        />
        <span className="text-muted-foreground text-xs">:</span>
        <Input
          placeholder="Port"
          value={port}
          onChange={(e) => setPort(e.target.value)}
          className="w-20 h-7 text-xs font-mono"
          disabled={connected}
        />
        <Input
          type="password"
          placeholder="Password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          className="w-36 h-7 text-xs"
          disabled={connected}
        />
        {connected ? (
          <Button variant="outline" size="sm" onClick={handleDisconnect}>
            Disconnect
          </Button>
        ) : (
          <Button size="sm" onClick={handleConnect}>
            Connect
          </Button>
        )}
        <div className={`w-2 h-2 rounded-full ${connected ? "bg-green-500" : "bg-red-500"}`} />
      </div>

      {/* Quick command buttons — only shown when connected */}
      {connected && (
        <div className="flex items-center gap-1.5 px-3 py-1.5 border-b border-border bg-card/50">
          <span className="text-xs text-muted-foreground mr-1">Quick:</span>
          <Button variant="outline" size="sm" className="h-6 text-xs" onClick={() => handleQuickCommand("players")}>
            Players
          </Button>
          <Button variant="outline" size="sm" className="h-6 text-xs" onClick={() => { setPromptTarget("servermsg"); setPromptValue(""); }}>
            Server Msg
          </Button>
          <Button variant="outline" size="sm" className="h-6 text-xs" onClick={() => handleQuickCommand("save")}>
            Save
          </Button>
          <Button variant="outline" size="sm" className="h-6 text-xs" onClick={() => handleQuickCommand("chopper")}>
            Chopper
          </Button>
          <Button variant="outline" size="sm" className="h-6 text-xs" onClick={() => { setPromptTarget("kick"); setPromptValue(""); }}>
            Kick
          </Button>
          <Button variant="destructive" size="sm" className="h-6 text-xs" onClick={() => handleQuickCommand("quit")}>
            Quit
          </Button>
        </div>
      )}

      {/* Prompt input for servermsg/kick */}
      {promptTarget && (
        <div className="flex items-center gap-2 px-3 py-1.5 border-b border-border bg-muted/50">
          <span className="text-xs text-muted-foreground whitespace-nowrap">
            {promptTarget === "servermsg" ? "Message:" : "Player name:"}
          </span>
          <input
            ref={promptRef}
            value={promptValue}
            onChange={(e) => setPromptValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handlePromptSubmit();
              if (e.key === "Escape") setPromptTarget(null);
            }}
            className="flex-1 bg-transparent text-sm font-mono outline-none placeholder:text-muted-foreground/50"
            placeholder={promptTarget === "servermsg" ? "Type message to broadcast..." : "Type player name..."}
          />
          <Button size="sm" className="h-6 text-xs" onClick={handlePromptSubmit} disabled={!promptValue.trim()}>
            Send
          </Button>
          <Button variant="outline" size="sm" className="h-6 text-xs" onClick={() => setPromptTarget(null)}>
            Cancel
          </Button>
        </div>
      )}

      {/* Terminal output */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-auto bg-[#0d1117] p-3 font-mono text-xs leading-5"
      >
        {lines.length === 0 ? (
          <div className="text-muted-foreground">
            Connect to your PZ server's RCON to send commands.
            <br />
            Common commands: players, servermsg, kick, banuser, save, quit
          </div>
        ) : (
          lines.map((line, i) => (
            <div key={i} className={lineColors[line.type] ?? "text-foreground"}>
              <span className="text-muted-foreground/40 mr-2">[{line.timestamp}]</span>
              {line.text}
            </div>
          ))
        )}
      </div>

      {/* Command input */}
      <div className="flex items-center gap-2 px-3 py-2 border-t border-border bg-card">
        <span className="text-xs text-muted-foreground font-mono">&gt;</span>
        <input
          ref={inputRef}
          value={command}
          onChange={(e) => setCommand(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={connected ? "Type a command..." : "Connect first..."}
          disabled={!connected}
          className="flex-1 bg-transparent text-sm font-mono outline-none placeholder:text-muted-foreground/50"
        />
        <Button size="sm" onClick={handleSend} disabled={!connected || !command.trim()}>
          Send
        </Button>
      </div>
    </div>
  );
}
