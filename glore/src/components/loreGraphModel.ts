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
  | "scope_membership"
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

export type GraphNodeType = "atom" | "scope";

export interface GraphNode {
  id: string;
  title: string;
  nodeType: GraphNodeType;
  kind: string;
  state: string;
  scope?: string;
  path?: string;
  created_unix_seconds: number;
  x: number;
  y: number;
  radius: number;
}

export interface GraphModel {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

const GROUP_ORDER = ["accepted", "proposed", "draft", "deprecated"];

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
    case "scope_membership":
      return "scope member";
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

const createAtomRelationEdges = (atoms: GraphAtom[]) => {
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

const scopeInfoFromAtom = (atom: GraphAtom) => {
  const label = atom.scope?.trim() || "Global";
  return {
    key: normalize(atom.scope) || "global",
    label,
  };
};

const sortedScopeEntries = (atoms: GraphAtom[]) => {
  const grouped = new Map<
    string,
    {
      key: string;
      label: string;
      atoms: GraphAtom[];
    }
  >();

  for (const atom of atoms) {
    const info = scopeInfoFromAtom(atom);
    const existing = grouped.get(info.key);
    if (!existing) {
      grouped.set(info.key, {
        key: info.key,
        label: info.label,
        atoms: [atom],
      });
      continue;
    }

    existing.atoms.push(atom);
  }

  return [...grouped.values()].sort((left, right) => {
    if (right.atoms.length !== left.atoms.length) {
      return right.atoms.length - left.atoms.length;
    }

    return left.label.localeCompare(right.label);
  });
};

const scopeNodeId = (scopeKey: string) => `scope::${scopeKey}`;

type BuildNodesResult = {
  nodes: GraphNode[];
  atomScopeNodeIds: Map<string, string>;
};

const buildNodes = (
  atoms: GraphAtom[],
  width: number,
  height: number,
): BuildNodesResult => {
  const scopeEntries = sortedScopeEntries(atoms);
  const nodes: GraphNode[] = [];
  const atomScopeNodeIds = new Map<string, string>();

  if (scopeEntries.length === 0) {
    return {
      nodes,
      atomScopeNodeIds,
    };
  }

  const centerX = width / 2;
  const centerY = height / 2;
  const ringRadius = Math.max(120, Math.min(width, height) * 0.28);

  scopeEntries.forEach((entry, scopeIndex) => {
    const angleOffset = -Math.PI / 2;
    const angle =
      scopeEntries.length === 1
        ? angleOffset
        : (scopeIndex / scopeEntries.length) * Math.PI * 2 + angleOffset;

    const scopeX =
      scopeEntries.length === 1
        ? centerX
        : centerX + Math.cos(angle) * ringRadius;
    const scopeY =
      scopeEntries.length === 1
        ? centerY
        : centerY + Math.sin(angle) * ringRadius;

    const scopeNode: GraphNode = {
      id: scopeNodeId(entry.key),
      title: entry.label,
      nodeType: "scope",
      kind: "scope",
      state: "scope",
      scope: entry.label,
      created_unix_seconds: 0,
      x: scopeX,
      y: scopeY,
      radius: Math.min(28, 18 + Math.ceil(Math.sqrt(entry.atoms.length) * 2)),
    };

    nodes.push(scopeNode);

    const byState = new Map<string, GraphAtom[]>();
    for (const state of GROUP_ORDER) {
      byState.set(state, []);
    }

    const sortedAtoms = [...entry.atoms].sort(
      (left, right) => left.created_unix_seconds - right.created_unix_seconds,
    );

    for (const atom of sortedAtoms) {
      const key = stateKey(atom.state);
      const bucket = byState.get(key) ?? byState.get("proposed");
      bucket?.push(atom);
      byState.set(key, bucket ?? [atom]);
    }

    GROUP_ORDER.forEach((state, stateIndex) => {
      const atomsInState = byState.get(state) ?? [];
      if (atomsInState.length === 0) {
        return;
      }

      const stateAngle =
        (stateIndex / GROUP_ORDER.length) * Math.PI * 2 - Math.PI / 2;
      const stateCenterX = scopeX + Math.cos(stateAngle) * 60;
      const stateCenterY = scopeY + Math.sin(stateAngle) * 60;

      atomsInState.forEach((atom, atomIndex) => {
        const ring = Math.floor(atomIndex / 6);
        const slot = atomIndex % 6;
        const spread = 16 + ring * 20;
        const wobble = (hash(atom.id) % 1000) / 1000;
        const angle = (slot / 6) * Math.PI * 2 + wobble * 0.9;

        atomScopeNodeIds.set(atom.id, scopeNode.id);
        nodes.push({
          ...atom,
          nodeType: "atom",
          scope: atom.scope?.trim() || "Global",
          x: stateCenterX + Math.cos(angle) * spread,
          y: stateCenterY + Math.sin(angle) * spread,
          radius: 9,
        });
      });
    });
  });

  return {
    nodes,
    atomScopeNodeIds,
  };
};

const createScopeMembershipEdges = (
  atoms: GraphAtom[],
  atomScopeNodeIds: Map<string, string>,
): GraphEdge[] => {
  const edges: GraphEdge[] = [];

  for (const atom of atoms) {
    const scopeNode = atomScopeNodeIds.get(atom.id);
    if (!scopeNode) {
      continue;
    }

    edges.push({
      id: relationKey(scopeNode, atom.id, "scope_membership"),
      source: scopeNode,
      target: atom.id,
      type: "scope_membership",
      explanation: `belongs to scope: ${atom.scope?.trim() || "Global"}`,
    });
  }

  return edges;
};

export const buildGraphModel = (
  atoms: GraphAtom[],
  width = 1060,
  height = 620,
): GraphModel => {
  const { nodes, atomScopeNodeIds } = buildNodes(atoms, width, height);
  const edges = [
    ...createAtomRelationEdges(atoms),
    ...createScopeMembershipEdges(atoms, atomScopeNodeIds),
  ];
  return { nodes, edges };
};

export const edgeColor = (type: RelationType) => {
  switch (type) {
    case "scope_membership":
      return "#fb7185";
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
