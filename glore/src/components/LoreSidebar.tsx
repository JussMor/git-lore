import { Activity, GitBranch, Plus } from "lucide-react";

type NewAtomKind = "decision" | "assumption" | "open_question" | "signal";

type SidebarAtom = {
  id: string;
  title: string;
  state: string;
};

type Props = {
  projectPath: string;
  loading: boolean;
  working: boolean;
  isMissingLoreError: boolean;
  root: string;
  error: string;
  atomsCount: number;
  filteredAtoms: SidebarAtom[];
  showCreateAtom: boolean;
  newAtomTitle: string;
  newAtomBody: string;
  newAtomScope: string;
  newAtomPath: string;
  newAtomKind: NewAtomKind;
  onChooseProject: () => void;
  onInitializeWorkspace: () => void;
  onToggleCreateAtom: () => void;
  onNewAtomTitleChange: (value: string) => void;
  onNewAtomBodyChange: (value: string) => void;
  onNewAtomScopeChange: (value: string) => void;
  onNewAtomPathChange: (value: string) => void;
  onNewAtomKindChange: (value: NewAtomKind) => void;
  onCreateAtom: () => void;
  onSelectAtom: (atomId: string) => void;
};

const stateTextClass = (state: string) => {
  switch (state) {
    case "accepted":
      return "text-emerald-400";
    case "deprecated":
      return "text-amber-400";
    case "draft":
      return "text-gray-400";
    default:
      return "text-blue-400";
  }
};

export function LoreSidebar({
  projectPath,
  loading,
  working,
  isMissingLoreError,
  root,
  error,
  atomsCount,
  filteredAtoms,
  showCreateAtom,
  newAtomTitle,
  newAtomBody,
  newAtomScope,
  newAtomPath,
  newAtomKind,
  onChooseProject,
  onInitializeWorkspace,
  onToggleCreateAtom,
  onNewAtomTitleChange,
  onNewAtomBodyChange,
  onNewAtomScopeChange,
  onNewAtomPathChange,
  onNewAtomKindChange,
  onCreateAtom,
  onSelectAtom,
}: Props) {
  return (
    <div className="w-64 bg-[#252526] flex flex-col border-r border-[#333333]">
      <div className="border-b border-[#333333] px-4 py-3 space-y-2">
        <div className="flex items-center justify-between gap-2">
          <span className="font-semibold text-sm">LOCAL REPO & LORE</span>
          <button
            className="rounded bg-[#0e639c] px-2 py-1 text-[11px] font-medium text-white hover:bg-[#1177bb] disabled:opacity-60"
            onClick={onChooseProject}
            disabled={loading}
          >
            Choose
          </button>
        </div>
        <button
          className="w-full rounded border border-[#3c3c3c] bg-[#1f1f1f] px-2 py-1 text-left text-[11px] text-gray-300 hover:border-[#555] disabled:opacity-60"
          onClick={onChooseProject}
          disabled={loading}
        >
          {projectPath || "Choose a project folder..."}
        </button>
        {isMissingLoreError ? (
          <button
            className="w-full rounded border border-[#2b5f2f] bg-[#1a2f1d] px-2 py-1 text-left text-[11px] text-emerald-200 hover:bg-[#204327] disabled:opacity-60"
            onClick={onInitializeWorkspace}
            disabled={working || loading}
          >
            Initialize .lore in this folder
          </button>
        ) : null}
        <span className="text-[10px] text-gray-500 truncate block" title={root}>
          {root || (loading ? "Loading workspace..." : "No project selected")}
        </span>
      </div>

      <div className="flex-1 overflow-y-auto pt-2 text-sm">
        <div className="px-3 py-1 flex justify-between items-center group cursor-pointer hover:bg-[#2a2d2e]">
          <div className="flex items-center space-x-2">
            <GitBranch size={16} className="text-[#007acc]" />
            <span>main</span>
          </div>
          <span className="text-gray-500 text-xs">HEAD</span>
        </div>

        <div className="mt-4 px-3 mb-1 text-xs font-semibold text-gray-400 uppercase tracking-widest flex justify-between">
          <span>
            .lore ATOMS ({filteredAtoms.length}/{atomsCount})
          </span>
          <button
            className="text-gray-300 hover:text-white"
            onClick={onToggleCreateAtom}
            title="Create atom"
          >
            <Plus size={14} className="cursor-pointer" />
          </button>
        </div>

        {showCreateAtom ? (
          <div className="mx-3 mb-3 rounded border border-[#3a3a3a] bg-[#1f1f1f] p-2 space-y-2">
            <input
              className="w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-xs text-gray-200"
              placeholder="Title"
              value={newAtomTitle}
              onChange={(event) => onNewAtomTitleChange(event.target.value)}
            />
            <textarea
              className="w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-xs text-gray-200"
              placeholder="Body (optional)"
              value={newAtomBody}
              onChange={(event) => onNewAtomBodyChange(event.target.value)}
              rows={2}
            />
            <input
              className="w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-xs text-gray-200"
              placeholder="Scope (optional)"
              value={newAtomScope}
              onChange={(event) => onNewAtomScopeChange(event.target.value)}
            />
            <input
              className="w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-xs text-gray-200"
              placeholder="File path (optional)"
              value={newAtomPath}
              onChange={(event) => onNewAtomPathChange(event.target.value)}
            />
            <select
              className="w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-xs text-gray-200"
              value={newAtomKind}
              onChange={(event) =>
                onNewAtomKindChange(event.target.value as NewAtomKind)
              }
            >
              <option value="decision">decision</option>
              <option value="assumption">assumption</option>
              <option value="open_question">open_question</option>
              <option value="signal">signal</option>
            </select>
            <button
              className="w-full rounded bg-[#0e639c] px-2 py-1 text-xs font-semibold text-white hover:bg-[#1177bb] disabled:opacity-60"
              onClick={onCreateAtom}
              disabled={working || loading}
            >
              Create Atom
            </button>
          </div>
        ) : null}

        {error ? (
          <div className="mx-3 mb-2 rounded border border-red-900 bg-red-950/60 px-3 py-2 text-xs text-red-200">
            {error}
          </div>
        ) : null}

        {filteredAtoms.length === 0 ? (
          <div className="px-5 py-2 text-xs text-gray-500 italic">
            {loading
              ? "Loading atoms..."
              : "No atoms found for current filters."}
          </div>
        ) : (
          filteredAtoms.map((atom) => (
            <button
              key={atom.id}
              className="w-full px-3 py-1 flex items-center space-x-2 text-left text-gray-300 cursor-pointer hover:bg-[#2a2d2e]"
              title={atom.id}
              onClick={() => onSelectAtom(atom.id)}
            >
              <Activity size={14} className={stateTextClass(atom.state)} />
              <span className="truncate">{atom.title}</span>
            </button>
          ))
        )}
      </div>
    </div>
  );
}
