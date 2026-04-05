import { useEffect, useMemo, useRef, useState } from "react";
import {
  buildGraphModel,
  edgeColor,
  edgeLabel,
  nodeFill,
  type GraphAtom,
  type RelationType,
} from "./loreGraphModel";

type Props = {
  atoms: GraphAtom[];
  selectedAtomId: string | null;
  onSelectAtom: (atomId: string) => void;
};

type ViewState = {
  scale: number;
  tx: number;
  ty: number;
};

const short = (value: string, max = 22) =>
  value.length > max ? `${value.slice(0, max - 1)}...` : value;

const scopeNodeFill = "#162235";

const relationPath = (
  sourceX: number,
  sourceY: number,
  targetX: number,
  targetY: number,
) => {
  const midX = (sourceX + targetX) / 2;
  const midY = (sourceY + targetY) / 2;

  const dx = targetX - sourceX;
  const dy = targetY - sourceY;
  const distance = Math.sqrt(dx * dx + dy * dy) || 1;

  const normalX = -dy / distance;
  const normalY = dx / distance;
  const bend = Math.min(84, distance * 0.2);

  const controlX = midX + normalX * bend;
  const controlY = midY + normalY * bend;

  return `M ${sourceX} ${sourceY} Q ${controlX} ${controlY} ${targetX} ${targetY}`;
};

const groupTitle = (state: string) =>
  state.charAt(0).toUpperCase() + state.slice(1).toLowerCase();

const clamp = (value: number, min: number, max: number) =>
  Math.max(min, Math.min(max, value));

const computeFitView = (
  nodes: Array<{ x: number; y: number; radius: number }>,
  width: number,
  height: number,
): ViewState => {
  if (nodes.length === 0) {
    return { scale: 1, tx: 0, ty: 0 };
  }

  let minX = Number.POSITIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;

  for (const node of nodes) {
    const labelAllowance = 140;
    minX = Math.min(minX, node.x - node.radius - 14);
    maxX = Math.max(maxX, node.x + node.radius + labelAllowance);
    minY = Math.min(minY, node.y - node.radius - 16);
    maxY = Math.max(maxY, node.y + node.radius + 16);
  }

  const graphWidth = Math.max(1, maxX - minX);
  const graphHeight = Math.max(1, maxY - minY);
  const padding = 38;

  const fitScale = clamp(
    Math.min(
      (width - padding * 2) / graphWidth,
      (height - padding * 2) / graphHeight,
    ),
    0.35,
    2.8,
  );

  const graphCenterX = (minX + maxX) / 2;
  const graphCenterY = (minY + maxY) / 2;

  return {
    scale: fitScale,
    tx: width / 2 - graphCenterX * fitScale,
    ty: height / 2 - graphCenterY * fitScale,
  };
};

export function LoreBrainGraph({ atoms, selectedAtomId, onSelectAtom }: Props) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [viewport, setViewport] = useState({ width: 1060, height: 620 });
  const [view, setView] = useState<ViewState>({ scale: 1, tx: 0, ty: 0 });

  useEffect(() => {
    const element = containerRef.current;
    if (!element) {
      return;
    }

    const updateViewport = () => {
      const rect = element.getBoundingClientRect();
      setViewport({
        width: Math.max(320, Math.round(rect.width)),
        height: Math.max(260, Math.round(rect.height)),
      });
    };

    updateViewport();

    const observer = new ResizeObserver(updateViewport);
    observer.observe(element);

    return () => observer.disconnect();
  }, []);

  const model = useMemo(
    () => buildGraphModel(atoms, viewport.width, viewport.height),
    [atoms, viewport.height, viewport.width],
  );

  const fitView = useMemo(
    () => computeFitView(model.nodes, viewport.width, viewport.height),
    [model.nodes, viewport.height, viewport.width],
  );

  useEffect(() => {
    setView(fitView);
  }, [fitView]);

  const nodeMap = useMemo(
    () => new Map(model.nodes.map((node) => [node.id, node])),
    [model.nodes],
  );

  const connectedWithSelection = useMemo(() => {
    const connected = new Set<string>();
    if (!selectedAtomId) {
      return connected;
    }

    for (const edge of model.edges) {
      if (edge.source === selectedAtomId || edge.target === selectedAtomId) {
        connected.add(edge.source);
        connected.add(edge.target);
      }
    }

    return connected;
  }, [model.edges, selectedAtomId]);

  const minScale = fitView.scale * 0.45;
  const maxScale = fitView.scale * 6.2;

  const zoomAt = (anchorX: number, anchorY: number, factor: number) => {
    setView((previous) => {
      const nextScale = clamp(previous.scale * factor, minScale, maxScale);
      const worldX = (anchorX - previous.tx) / previous.scale;
      const worldY = (anchorY - previous.ty) / previous.scale;

      return {
        scale: nextScale,
        tx: anchorX - worldX * nextScale,
        ty: anchorY - worldY * nextScale,
      };
    });
  };

  const zoomIn = () => zoomAt(viewport.width / 2, viewport.height / 2, 1.16);
  const zoomOut = () => zoomAt(viewport.width / 2, viewport.height / 2, 0.86);

  const handleWheel = (event: React.WheelEvent<HTMLDivElement>) => {
    event.preventDefault();
    const rect = event.currentTarget.getBoundingClientRect();
    const anchorX = event.clientX - rect.left;
    const anchorY = event.clientY - rect.top;
    const factor = event.deltaY < 0 ? 1.1 : 0.9;
    zoomAt(anchorX, anchorY, factor);
  };

  if (atoms.length === 0) {
    return (
      <div className="flex h-full w-full items-center justify-center rounded-xl border border-[#333] bg-[#141414] text-sm text-gray-500">
        Load a workspace to generate the lore relation brain graph.
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      className="relative h-full w-full overflow-hidden rounded-xl border border-[#333] bg-[#111315]"
      onWheel={handleWheel}
    >
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_20%_20%,rgba(14,165,233,0.14),transparent_35%),radial-gradient(circle_at_80%_70%,rgba(16,185,129,0.12),transparent_35%),radial-gradient(circle_at_50%_55%,rgba(245,158,11,0.08),transparent_40%)]" />

      <svg
        className="relative z-10 h-full w-full"
        viewBox={`0 0 ${viewport.width} ${viewport.height}`}
        role="img"
        aria-label="Lore relationship graph"
      >
        <g transform={`translate(${view.tx} ${view.ty}) scale(${view.scale})`}>
          {model.edges.map((edge) => {
            const source = nodeMap.get(edge.source);
            const target = nodeMap.get(edge.target);
            if (!source || !target) {
              return null;
            }

            const selected =
              selectedAtomId &&
              (edge.source === selectedAtomId ||
                edge.target === selectedAtomId);
            const faded = selectedAtomId && !selected;
            const isScopeMembership = edge.type === "scope_membership";

            return (
              <path
                key={edge.id}
                d={relationPath(source.x, source.y, target.x, target.y)}
                fill="none"
                stroke={edgeColor(edge.type)}
                strokeOpacity={
                  faded ? 0.08 : selected ? 0.9 : isScopeMembership ? 0.22 : 0.3
                }
                strokeWidth={selected ? 2.6 : isScopeMembership ? 1.1 : 1.4}
                strokeLinecap="round"
                strokeDasharray={isScopeMembership ? "3 3" : undefined}
              >
                <title>{`${edgeLabel(edge.type)} | ${edge.explanation}`}</title>
              </path>
            );
          })}

          {model.nodes.map((node) => {
            const isScope = node.nodeType === "scope";
            const isSelected = !isScope && selectedAtomId === node.id;
            const isSignal =
              !isScope && node.kind.trim().toLowerCase() === "signal";
            const dimmed =
              !!selectedAtomId &&
              !isSelected &&
              !connectedWithSelection.has(node.id);

            if (isScope) {
              return (
                <g key={node.id} opacity={dimmed ? 0.42 : 1}>
                  <circle
                    cx={node.x}
                    cy={node.y}
                    r={node.radius + 3}
                    fill={scopeNodeFill}
                    stroke={
                      connectedWithSelection.has(node.id)
                        ? "#93c5fd"
                        : "#475569"
                    }
                    strokeWidth={
                      connectedWithSelection.has(node.id) ? 2.2 : 1.4
                    }
                  />
                  <circle
                    cx={node.x}
                    cy={node.y}
                    r={node.radius - 4}
                    fill="none"
                    stroke="#334155"
                    strokeWidth={1}
                    strokeDasharray="2 3"
                  />
                  <text
                    x={node.x}
                    y={node.y + 4}
                    fill="#dbeafe"
                    textAnchor="middle"
                    fontSize={11}
                    fontWeight={600}
                  >
                    {short(node.title, 18)}
                  </text>
                  <title>{`Scope cluster: ${node.title}`}</title>
                </g>
              );
            }

            return (
              <g
                key={node.id}
                onClick={() => onSelectAtom(node.id)}
                className="cursor-pointer"
                opacity={dimmed ? 0.34 : 1}
              >
                {isSignal ? (
                  <circle
                    cx={node.x}
                    cy={node.y}
                    r={isSelected ? node.radius + 7 : node.radius + 5}
                    fill="none"
                    stroke="#fb7185"
                    strokeOpacity={dimmed ? 0.22 : isSelected ? 0.95 : 0.7}
                    strokeWidth={isSelected ? 2.1 : 1.4}
                    strokeDasharray="4 3"
                  />
                ) : null}
                <circle
                  cx={node.x}
                  cy={node.y}
                  r={isSelected ? node.radius + 4 : node.radius}
                  fill={nodeFill(node.state)}
                  stroke={isSelected ? "#e5e7eb" : "#0b0f12"}
                  strokeWidth={isSelected ? 2.5 : 1.6}
                />
                {isSignal ? (
                  <circle
                    cx={node.x}
                    cy={node.y}
                    r={2.2}
                    fill={isSelected ? "#ffe4e6" : "#fda4af"}
                  />
                ) : null}
                <text
                  x={node.x + node.radius + 6}
                  y={node.y + 4}
                  fill={isSelected ? "#f8fafc" : "#cbd5e1"}
                  fontSize={12}
                  fontWeight={isSelected ? 600 : 400}
                >
                  {short(node.title)}
                </text>
                <title>{`${node.title} (${node.kind} / ${node.state})`}</title>
              </g>
            );
          })}
        </g>
      </svg>

      <div className="absolute left-3 top-3 z-20 rounded-md border border-[#333] bg-[#0f1011]/95 px-3 py-2 text-[11px] text-gray-300">
        <div className="mb-1 font-semibold text-gray-200">
          Clusters And Levels
        </div>
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <span className="inline-block h-2.5 w-2.5 rounded-full border border-slate-400 bg-slate-800" />
            <span>Scope cluster</span>
          </div>
          {["accepted", "proposed", "draft", "deprecated"].map((state) => (
            <div key={state} className="flex items-center gap-2">
              <span
                className="inline-block h-2.5 w-2.5 rounded-full"
                style={{ backgroundColor: nodeFill(state) }}
              />
              <span>{groupTitle(state)}</span>
            </div>
          ))}
          <div className="mt-2 border-t border-[#2e2e2e] pt-2">
            <span className="inline-flex items-center gap-2 text-[10px] text-rose-200">
              <span className="inline-block h-2.5 w-2.5 rounded-full border border-dashed border-rose-300" />
              Signal atom ring
            </span>
          </div>
        </div>
      </div>

      <div className="absolute right-3 top-3 z-20 rounded-md border border-[#333] bg-[#0f1011]/95 px-3 py-2 text-[11px] text-gray-300">
        <div className="mb-1 font-semibold text-gray-200">Relation Types</div>
        <div className="space-y-1">
          {[
            ["scope_membership", "scope membership"],
            ["temporal", "time neighbor"],
            ["same_scope", "same scope"],
            ["same_path", "same path"],
            ["same_kind", "same kind"],
          ].map(([type, label]) => (
            <div key={type} className="flex items-center gap-2">
              <span
                className="inline-block h-0.5 w-4"
                style={{ backgroundColor: edgeColor(type as RelationType) }}
              />
              <span>{label}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="absolute bottom-3 right-3 z-20 flex items-center gap-1 rounded-md border border-[#333] bg-[#0f1011]/95 p-1 text-[11px] text-gray-300">
        <button
          className="rounded border border-[#3b3b3b] px-2 py-1 hover:bg-[#1b1f24]"
          onClick={zoomOut}
          type="button"
        >
          -
        </button>
        <button
          className="rounded border border-[#3b3b3b] px-2 py-1 hover:bg-[#1b1f24]"
          onClick={() => setView(fitView)}
          type="button"
        >
          Fit
        </button>
        <button
          className="rounded border border-[#3b3b3b] px-2 py-1 hover:bg-[#1b1f24]"
          onClick={zoomIn}
          type="button"
        >
          +
        </button>
        <div className="ml-1 w-14 text-right text-gray-400">
          {Math.round((view.scale / fitView.scale) * 100)}%
        </div>
      </div>
    </div>
  );
}
