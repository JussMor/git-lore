export interface GraphAtom {
  id: string;
  title: string;
  kind: string;
  state: string;
  scope?: string;
  path?: string;
  created_unix_seconds: number;
}

export type RelationType =
  | "temporal"
  | "same_scope"
  | "same_path"
  | "same_kind";

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  type: RelationType;
  explanation: string;
}

export interface GraphNode extends GraphAtom {
  x: number;
  y: number;
  radius: number;
}

export interface GraphModel {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

const GROUP_ORDER = ["accepted", "proposed", "draft", "deprecated"];

const CENTER_MAP: Record<string, { x: number; y: number }> = {
  accepted: { x: 0.72, y: 0.36 },
  proposed: { x: 0.3, y: 0.32 },
  draft: { x: 0.28, y: 0.74 },
  deprecated: { x: 0.7, y: 0.74 },
};

const stateKey = (value: string) => value.trim().toLowerCase();

const normalize = (value?: string) => value?.trim().toLowerCase() ?? "";

const hash = (input: string) => {
  let code = 2166136261;
  for (let index = 0; index < input.length; index += 1) {
    code ^= input.charCodeAt(index);
    code = Math.imul(code, 16777619);
  }
  return Math.abs(code >>> 0);
};

const sortedPair = (left: string, right: string) =>
  left < right ? `${left}::${right}` : `${right}::${left}`;

const relationKey = (left: string, right: string, type: RelationType) =>
  `${sortedPair(left, right)}::${type}`;

const relationLabel = (type: RelationType) => {
  switch (type) {
    case "same_scope":
      return "same scope";
    case "same_path":
      return "same path";
    case "same_kind":
      return "same kind";
    default:
      return "time neighbor";
  }
};

const createEdges = (atoms: GraphAtom[]) => {
  const edges: GraphEdge[] = [];
  const known = new Set<string>();

  const addEdge = (
    source: GraphAtom,
    target: GraphAtom,
    type: RelationType,
    explanation: string,
  ) => {
    if (source.id === target.id) {
      return;
    }

    const key = relationKey(source.id, target.id, type);
    if (known.has(key)) {
      return;
    }

    known.add(key);
    edges.push({
      id: key,
      source: source.id,
      target: target.id,
      type,
      explanation,
    });
  };

  const byTime = [...atoms].sort(
    (left, right) => left.created_unix_seconds - right.created_unix_seconds,
  );

  for (let index = 1; index < byTime.length; index += 1) {
    const current = byTime[index];
    const previous = byTime[index - 1];
    addEdge(previous, current, "temporal", "created in adjacent timestamps");
  }

  const groupBy = (selector: (atom: GraphAtom) => string) => {
    const groups = new Map<string, GraphAtom[]>();
    for (const atom of atoms) {
      const key = selector(atom);
      if (!key) {
        continue;
      }
      const existing = groups.get(key) ?? [];
      existing.push(atom);
      groups.set(key, existing);
    }
    return groups;
  };

  const scopeGroups = groupBy((atom) => normalize(atom.scope));
  for (const [scope, group] of scopeGroups.entries()) {
    if (group.length < 2) {
      continue;
    }

    const sorted = [...group].sort(
      (left, right) => left.created_unix_seconds - right.created_unix_seconds,
    );

    for (let index = 1; index < sorted.length; index += 1) {
      addEdge(
        sorted[index - 1],
        sorted[index],
        "same_scope",
        `share scope: ${scope}`,
      );
    }
  }

  const pathGroups = groupBy((atom) => normalize(atom.path));
  for (const [path, group] of pathGroups.entries()) {
    if (group.length < 2) {
      continue;
    }

    const sorted = [...group].sort(
      (left, right) => left.created_unix_seconds - right.created_unix_seconds,
    );

    for (let index = 1; index < sorted.length; index += 1) {
      addEdge(
        sorted[index - 1],
        sorted[index],
        "same_path",
        `touch path: ${path}`,
      );
    }
  }

  const kindGroups = groupBy((atom) => normalize(atom.kind));
  for (const [kind, group] of kindGroups.entries()) {
    if (group.length < 2) {
      continue;
    }

    const anchor = group[0];
    for (let index = 1; index < group.length && index < 7; index += 1) {
      addEdge(anchor, group[index], "same_kind", `same lore kind: ${kind}`);
    }
  }

  return edges;
};

const buildNodes = (
  atoms: GraphAtom[],
  width: number,
  height: number,
): GraphNode[] => {
  const grouped = new Map<string, GraphAtom[]>();

  for (const groupName of GROUP_ORDER) {
    grouped.set(groupName, []);
  }

  for (const atom of atoms) {
    const key = stateKey(atom.state);
    const bucket = grouped.get(key) ?? grouped.get("proposed");
    bucket?.push(atom);
    grouped.set(key, bucket ?? [atom]);
  }

  const nodes: GraphNode[] = [];

  for (const [groupName, groupAtoms] of grouped.entries()) {
    const center = CENTER_MAP[groupName] ?? CENTER_MAP.proposed;
    const cx = width * center.x;
    const cy = height * center.y;

    groupAtoms.forEach((atom, index) => {
      const ring = Math.floor(index / 7);
      const slot = index % 7;
      const spread = 36 + ring * 44;
      const wobble = (hash(atom.id) % 1000) / 1000;
      const angle = (slot / 7) * Math.PI * 2 + wobble * 0.8;

      nodes.push({
        ...atom,
        x: cx + Math.cos(angle) * spread,
        y: cy + Math.sin(angle) * spread,
        radius: 9,
      });
    });
  }

  return nodes;
};

export const buildGraphModel = (
  atoms: GraphAtom[],
  width = 1060,
  height = 620,
): GraphModel => {
  const nodes = buildNodes(atoms, width, height);
  const edges = createEdges(atoms);
  return { nodes, edges };
};

export const edgeColor = (type: RelationType) => {
  switch (type) {
    case "same_scope":
      return "#f59e0b";
    case "same_path":
      return "#22c55e";
    case "same_kind":
      return "#a855f7";
    default:
      return "#38bdf8";
  }
};

export const edgeLabel = (type: RelationType) => relationLabel(type);

export const nodeFill = (state: string) => {
  switch (stateKey(state)) {
    case "accepted":
      return "#10b981";
    case "deprecated":
      return "#f59e0b";
    case "draft":
      return "#6b7280";
    default:
      return "#0ea5e9";
  }
};
