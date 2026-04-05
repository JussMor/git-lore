import { useMemo } from "react";
import {
  buildGraphModel,
  edgeLabel,
  type GraphAtom,
  type RelationType,
} from "./loreGraphModel";

type Props = {
  atoms: GraphAtom[];
  selectedAtomId: string | null;
};

const stateOrder = ["accepted", "proposed", "draft", "deprecated"];

const normalize = (value?: string) => value?.trim().toLowerCase() ?? "";

const relationOrder: RelationType[] = [
  "same_scope",
  "same_path",
  "same_kind",
  "temporal",
];

const relationClass = (type: RelationType) => {
  switch (type) {
    case "same_scope":
      return "border-amber-800 text-amber-200 bg-amber-950/40";
    case "same_path":
      return "border-emerald-800 text-emerald-200 bg-emerald-950/40";
    case "same_kind":
      return "border-fuchsia-800 text-fuchsia-200 bg-fuchsia-950/40";
    default:
      return "border-sky-800 text-sky-200 bg-sky-950/40";
  }
};

const stateClass = (state: string) => {
  switch (normalize(state)) {
    case "accepted":
      return "border-emerald-800 text-emerald-200 bg-emerald-950/40";
    case "draft":
      return "border-gray-700 text-gray-200 bg-gray-900/60";
    case "deprecated":
      return "border-amber-800 text-amber-200 bg-amber-950/40";
    default:
      return "border-sky-800 text-sky-200 bg-sky-950/40";
  }
};

export function SignalInsights({ atoms, selectedAtomId }: Props) {
  const atomMap = useMemo(
    () => new Map(atoms.map((atom) => [atom.id, atom])),
    [atoms],
  );

  const model = useMemo(() => buildGraphModel(atoms), [atoms]);

  const signals = useMemo(
    () =>
      atoms
        .filter((atom) => normalize(atom.kind) === "signal")
        .sort(
          (left, right) =>
            right.created_unix_seconds - left.created_unix_seconds,
        ),
    [atoms],
  );

  const signalIds = useMemo(
    () => new Set(signals.map((atom) => atom.id)),
    [signals],
  );

  const stateCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const atom of signals) {
      const key = normalize(atom.state);
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
    return counts;
  }, [signals]);

  const scopeSummary = useMemo(() => {
    const counts = new Map<string, number>();
    for (const atom of signals) {
      const key = atom.scope?.trim() || "global";
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }

    return [...counts.entries()]
      .sort((left, right) => right[1] - left[1])
      .slice(0, 4);
  }, [signals]);

  const relationCounts = useMemo(() => {
    const counts: Record<RelationType, number> = {
      temporal: 0,
      same_scope: 0,
      same_path: 0,
      same_kind: 0,
    };

    for (const edge of model.edges) {
      if (signalIds.has(edge.source) || signalIds.has(edge.target)) {
        counts[edge.type] += 1;
      }
    }

    return counts;
  }, [model.edges, signalIds]);

  const selectedSignal = useMemo(() => {
    if (!selectedAtomId) {
      return null;
    }

    return signals.find((atom) => atom.id === selectedAtomId) ?? null;
  }, [selectedAtomId, signals]);

  const selectedSignalLinks = useMemo(() => {
    if (!selectedSignal) {
      return [] as Array<{ id: string; title: string; relation: RelationType }>;
    }

    return model.edges
      .filter(
        (edge) =>
          edge.source === selectedSignal.id ||
          edge.target === selectedSignal.id,
      )
      .map((edge) => {
        const counterpartId =
          edge.source === selectedSignal.id ? edge.target : edge.source;

        return {
          id: counterpartId,
          title: atomMap.get(counterpartId)?.title ?? counterpartId,
          relation: edge.type,
        };
      })
      .slice(0, 6);
  }, [atomMap, model.edges, selectedSignal]);

  if (signals.length === 0) {
    return (
      <div className="mt-4 rounded border border-[#3a3a3a] bg-[#1f1f1f] p-3">
        <div className="mb-2 text-xs font-semibold text-gray-300">
          Signal Lens
        </div>
        <div className="text-xs text-gray-500">
          No signal atoms yet. Create atoms with kind "signal" to visualize
          risk, telemetry, or anomalies.
        </div>
      </div>
    );
  }

  return (
    <div className="mt-4 rounded border border-[#3a3a3a] bg-[#1f1f1f] p-3">
      <div className="mb-2 flex items-center justify-between text-xs">
        <span className="font-semibold text-gray-300">Signal Lens</span>
        <span className="rounded border border-rose-900 bg-rose-950/50 px-2 py-0.5 text-[10px] text-rose-200">
          {signals.length} total
        </span>
      </div>

      <div className="mb-3 rounded border border-[#3a3a3a] bg-[#161616] p-2 text-xs text-gray-300">
        <div className="mb-1 text-gray-400">Signal states</div>
        <div className="flex flex-wrap gap-1">
          {stateOrder.map((state) => (
            <span
              key={state}
              className={`rounded border px-1.5 py-0.5 text-[10px] ${stateClass(state)}`}
            >
              {state}: {stateCounts.get(state) ?? 0}
            </span>
          ))}
        </div>
      </div>

      <div className="mb-3 rounded border border-[#3a3a3a] bg-[#161616] p-2 text-xs text-gray-300">
        <div className="mb-1 text-gray-400">Signal relation pressure</div>
        <div className="flex flex-wrap gap-1">
          {relationOrder.map((type) => (
            <span
              key={type}
              className={`rounded border px-1.5 py-0.5 text-[10px] ${relationClass(type)}`}
            >
              {edgeLabel(type)}: {relationCounts[type]}
            </span>
          ))}
        </div>
      </div>

      <div className="mb-3 rounded border border-[#3a3a3a] bg-[#161616] p-2 text-xs text-gray-300">
        <div className="mb-1 text-gray-400">Top signal scopes</div>
        {scopeSummary.length === 0 ? (
          <div className="text-gray-500">No scoped signals yet.</div>
        ) : (
          <div className="space-y-1">
            {scopeSummary.map(([scope, count]) => (
              <div key={scope} className="flex items-center justify-between">
                <span className="truncate text-gray-300">{scope}</span>
                <span className="text-gray-400">{count}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {selectedSignal ? (
        <div className="rounded border border-rose-900 bg-rose-950/25 p-2 text-xs">
          <div className="mb-1 font-semibold text-rose-200">
            Selected signal focus
          </div>
          <div className="mb-2 text-gray-300">{selectedSignal.title}</div>
          {selectedSignalLinks.length === 0 ? (
            <div className="text-gray-500">
              No graph links for this signal yet.
            </div>
          ) : (
            <div className="space-y-1">
              {selectedSignalLinks.map((item) => (
                <div key={`${selectedSignal.id}-${item.id}-${item.relation}`}>
                  <span className="text-gray-300">{item.title}</span>{" "}
                  <span className="text-gray-500">
                    ({edgeLabel(item.relation)})
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      ) : (
        <div className="text-xs text-gray-500">
          Select a signal atom to inspect its immediate graph links.
        </div>
      )}
    </div>
  );
}
