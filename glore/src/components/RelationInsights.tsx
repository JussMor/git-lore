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

export function RelationInsights({ atoms, selectedAtomId }: Props) {
  const model = useMemo(() => buildGraphModel(atoms), [atoms]);

  const atomMap = useMemo(
    () => new Map(atoms.map((atom) => [atom.id, atom])),
    [atoms],
  );

  const selected = selectedAtomId
    ? (atomMap.get(selectedAtomId) ?? null)
    : null;

  const linked = useMemo(() => {
    if (!selectedAtomId) {
      return [];
    }

    return model.edges
      .filter(
        (edge) =>
          edge.source === selectedAtomId || edge.target === selectedAtomId,
      )
      .map((edge) => {
        const counterpartId =
          edge.source === selectedAtomId ? edge.target : edge.source;
        return {
          edge,
          atom: atomMap.get(counterpartId),
        };
      })
      .filter((value) => !!value.atom);
  }, [atomMap, model.edges, selectedAtomId]);

  const groupedLinks = useMemo(() => {
    const grouped = new Map<
      string,
      {
        atom: GraphAtom;
        reasons: Array<{ type: RelationType; explanation: string }>;
      }
    >();

    for (const item of linked) {
      if (!item.atom) {
        continue;
      }

      const existing = grouped.get(item.atom.id);
      const reason = {
        type: item.edge.type,
        explanation: item.edge.explanation,
      };

      if (!existing) {
        grouped.set(item.atom.id, {
          atom: item.atom,
          reasons: [reason],
        });
        continue;
      }

      if (
        !existing.reasons.some(
          (value) =>
            value.type === reason.type &&
            value.explanation === reason.explanation,
        )
      ) {
        existing.reasons.push(reason);
      }
    }

    return [...grouped.values()].sort(
      (left, right) => right.reasons.length - left.reasons.length,
    );
  }, [linked]);

  const stats = useMemo(() => {
    const counts: Record<RelationType, number> = {
      temporal: 0,
      same_scope: 0,
      same_path: 0,
      same_kind: 0,
    };

    for (const item of linked) {
      counts[item.edge.type] += 1;
    }

    return counts;
  }, [linked]);

  if (!selected) {
    return (
      <div className="mt-4 rounded border border-[#3a3a3a] bg-[#1f1f1f] p-3">
        <div className="mb-2 text-xs font-semibold text-gray-300">
          Relation Insights
        </div>
        <div className="text-xs text-gray-500">
          Select an atom to understand why it is connected to other atoms.
        </div>
      </div>
    );
  }

  return (
    <div className="mt-4 rounded border border-[#3a3a3a] bg-[#1f1f1f] p-3">
      <div className="mb-2 text-xs font-semibold text-gray-300">
        Why This Atom Fits
      </div>
      <div className="mb-3 rounded border border-[#3a3a3a] bg-[#161616] p-2 text-xs text-gray-300">
        <div>
          <span className="text-gray-500">Linked atoms:</span>{" "}
          {groupedLinks.length}
        </div>
        <div className="mt-1 flex flex-wrap gap-1">
          {relationOrder.map((type) => (
            <span
              key={type}
              className={`rounded border px-1.5 py-0.5 text-[10px] ${relationClass(type)}`}
            >
              {edgeLabel(type)}: {stats[type]}
            </span>
          ))}
        </div>
      </div>

      {groupedLinks.length === 0 ? (
        <div className="text-xs text-gray-500">
          This atom has no inferred relation yet. Add scope/path metadata to
          increase graph linkage.
        </div>
      ) : (
        <div className="max-h-44 space-y-2 overflow-y-auto pr-1 text-xs">
          {groupedLinks.slice(0, 10).map((item) => (
            <div
              key={item.atom.id}
              className="rounded border border-[#333] bg-[#161616] p-2"
            >
              <div className="font-medium text-gray-200">{item.atom.title}</div>
              <div className="mt-1 text-gray-400">
                {item.reasons.length > 1
                  ? `${item.reasons.length} relation signals`
                  : item.reasons[0].explanation}
              </div>
              <div className="mt-1 flex flex-wrap gap-1">
                {item.reasons.map((reason) => (
                  <span
                    key={`${item.atom.id}-${reason.type}-${reason.explanation}`}
                    className={`rounded border px-1.5 py-0.5 text-[10px] ${relationClass(reason.type)}`}
                    title={reason.explanation}
                  >
                    {edgeLabel(reason.type)}
                  </span>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
