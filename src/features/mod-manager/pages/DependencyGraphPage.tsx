import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
  type NodeChange,
  MarkerType,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useModManagerStore } from "../store";
import { useActiveProfile } from "../../profiles/store";

const STORAGE_KEY = "modzboid-graph-positions";

const categoryColors: Record<string, string> = {
  framework: "#3b82f6",  // blue
  map: "#22c55e",         // green
  content: "#a855f7",     // purple
  overhaul: "#f59e0b",    // amber
};

/** Topological layer assignment — nodes with no deps go to layer 0, dependents go deeper */
function assignLayers(
  mods: { id: string; requires: string[] }[],
  enabledSet: Set<string>
): Map<string, number> {
  const layers = new Map<string, number>();
  const modMap = new Map(mods.map((m) => [m.id, m]));

  function getLayer(id: string, visited: Set<string>): number {
    if (layers.has(id)) return layers.get(id)!;
    if (visited.has(id)) return 0; // cycle
    visited.add(id);
    const mod = modMap.get(id);
    if (!mod || mod.requires.length === 0) {
      layers.set(id, 0);
      return 0;
    }
    const depLayers = mod.requires
      .filter((d) => enabledSet.has(d) && modMap.has(d))
      .map((d) => getLayer(d, visited));
    const layer = depLayers.length > 0 ? Math.max(...depLayers) + 1 : 0;
    layers.set(id, layer);
    return layer;
  }

  for (const mod of mods) {
    getLayer(mod.id, new Set());
  }
  return layers;
}

/** Load saved positions from localStorage */
function loadPositions(): Record<string, { x: number; y: number }> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

/** Save positions to localStorage */
function savePositions(nodes: Node[]) {
  const positions: Record<string, { x: number; y: number }> = {};
  for (const node of nodes) {
    positions[node.id] = { x: node.position.x, y: node.position.y };
  }
  localStorage.setItem(STORAGE_KEY, JSON.stringify(positions));
}

export default function DependencyGraphPage() {
  const allMods = useModManagerStore((s) => s.allMods);
  const selectMod = useModManagerStore((s) => s.selectMod);
  const activeProfile = useActiveProfile();
  const enabledSet = useMemo(
    () => new Set(activeProfile?.loadOrder ?? []),
    [activeProfile]
  );
  const [resetCounter, setResetCounter] = useState(0);
  const { initialNodes, initialEdges } = useMemo(() => {
    const enabledMods = allMods.filter((m) => enabledSet.has(m.id));
    const modMap = new Map(allMods.map((m) => [m.id, m]));
    const savedPositions = loadPositions();

    // Assign layers for hierarchical layout
    const layers = assignLayers(enabledMods, enabledSet);
    // Group mods by layer
    const layerBuckets: Map<number, typeof enabledMods> = new Map();
    for (const mod of enabledMods) {
      const layer = layers.get(mod.id) ?? 0;
      const bucket = layerBuckets.get(layer) || [];
      bucket.push(mod);
      layerBuckets.set(layer, bucket);
    }

    const spacingX = 240;
    const spacingY = 120;

    const nodes: Node[] = enabledMods.map((mod) => {
      const category = mod.detectedCategory ?? mod.category ?? "content";
      const color = categoryColors[category] ?? "#6b7280";
      const layer = layers.get(mod.id) ?? 0;
      const bucket = layerBuckets.get(layer) ?? [];
      const indexInLayer = bucket.indexOf(mod);
      const layerWidth = bucket.length * spacingX;
      const offsetX = -layerWidth / 2 + indexInLayer * spacingX;

      // Use saved position if available, otherwise use computed layout
      const savedPos = savedPositions[mod.id];
      const position = savedPos ?? {
        x: offsetX,
        y: layer * spacingY,
      };

      const hasDeps = mod.requires.some((d) => enabledSet.has(d) && modMap.has(d));
      const isDependedOn = enabledMods.some(
        (other) => other.requires.includes(mod.id)
      );

      return {
        id: mod.id,
        position,
        data: { label: mod.name },
        style: {
          background: color + "20",
          border: `2px solid ${color}`,
          borderRadius: 8,
          padding: "8px 12px",
          fontSize: 12,
          fontWeight: isDependedOn ? 700 : 500,
          minWidth: 140,
          textAlign: "center" as const,
          opacity: !hasDeps && !isDependedOn ? 0.5 : 1,
        },
      };
    });

    const edges: Edge[] = [];
    for (const mod of enabledMods) {
      for (const depId of mod.requires) {
        if (enabledSet.has(depId) && modMap.has(depId)) {
          edges.push({
            id: `${depId}->${mod.id}`,
            source: depId,
            target: mod.id,
            animated: false,
            style: { stroke: "#6b7280", strokeWidth: 1.5 },
            markerEnd: { type: MarkerType.ArrowClosed, width: 12, height: 12, color: "#6b7280" },
          });
        }
      }
    }

    return { initialNodes: nodes, initialEdges: edges };
  }, [allMods, enabledSet, resetCounter]);

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, , onEdgesChange] = useEdgesState(initialEdges);

  // Sync when initialNodes changes (profile/mods changed)
  useEffect(() => {
    setNodes(initialNodes);
  }, [initialNodes, setNodes]);

  // Save positions on node drag (debounced)
  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      onNodesChange(changes);
      // Position saving is handled by onNodeDragStop
    },
    [onNodesChange]
  );

  // Save positions whenever a drag ends
  const onNodeDragStop = useCallback(
    (_: React.MouseEvent, _node: Node, allNodes: Node[]) => {
      savePositions(allNodes);
    },
    []
  );

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      selectMod(node.id);
    },
    [selectMod]
  );

  const handleResetLayout = useCallback(() => {
    localStorage.removeItem(STORAGE_KEY);
    setResetCounter((c) => c + 1);
  }, []);

  if (initialNodes.length === 0) {
    return (
      <div data-testid="page-graph" className="flex items-center justify-center h-full text-sm text-muted-foreground">
        No enabled mods to visualize. Enable some mods first.
      </div>
    );
  }

  // Count mods with dependencies (connected to the graph)
  const connectedCount = initialNodes.filter((n) => n.style?.opacity !== 0.5).length;
  const isolatedCount = initialNodes.length - connectedCount;

  // Counteract CSS zoom on <html> to prevent ReactFlow coordinate mismatch.
  // ReactFlow measures coordinates via getBoundingClientRect which is affected by zoom,
  // but mouse events may not be, causing drag to move 2x as far.
  const htmlZoom = parseFloat(getComputedStyle(document.documentElement).zoom) || 1;

  return (
    <div
      data-testid="page-graph"
      className="h-full w-full relative"
      style={htmlZoom !== 1 ? { zoom: 1 / htmlZoom, width: `${htmlZoom * 100}%`, height: `${htmlZoom * 100}%` } : undefined}
    >
      {/* Stats bar */}
      <div className="absolute top-2 left-2 z-10 flex items-center gap-3 bg-card/90 border border-border rounded-md px-3 py-1.5 text-xs">
        <span>{initialNodes.length} mods</span>
        <span>{initialEdges.length} dependencies</span>
        {isolatedCount > 0 && (
          <span className="text-muted-foreground">({isolatedCount} standalone)</span>
        )}
        <button
          onClick={handleResetLayout}
          className="text-muted-foreground hover:text-foreground transition-colors"
          title="Reset layout to auto-arranged positions"
        >
          Reset Layout
        </button>
      </div>
      {/* Legend */}
      <div className="absolute top-2 right-2 z-10 flex items-center gap-2 bg-card/90 border border-border rounded-md px-3 py-1.5 text-xs">
        {Object.entries(categoryColors).map(([cat, color]) => (
          <span key={cat} className="flex items-center gap-1">
            <span className="w-2.5 h-2.5 rounded-sm" style={{ background: color }} />
            {cat}
          </span>
        ))}
      </div>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={handleNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={onNodeClick}
        onNodeDragStop={onNodeDragStop}
        fitView
        attributionPosition="bottom-left"
        style={{ width: "100%", height: "100%" }}
      >
        <Background />
        <Controls />
        <MiniMap
          nodeColor={(node) => {
            const style = node.style as Record<string, string> | undefined;
            return style?.border?.replace("2px solid ", "") ?? "#6b7280";
          }}
          style={{ background: "#1a1a2e" }}
        />
      </ReactFlow>
    </div>
  );
}
