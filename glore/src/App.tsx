import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Activity,
  AlertTriangle,
  Folder,
  GitBranch,
  Layers,
  Plus,
  RefreshCw,
  ShieldCheck,
  Terminal,
} from "lucide-react";
import { useState } from "react";
import "./App.css";

export interface LoreAtom {
  id: string;
  kind: string;
  state: string;
  title: string;
  body?: string;
  scope?: string;
  validation_script?: string;
  created_unix_seconds: number;
}

type WorkspaceSnapshot = {
  root: string;
  atoms: LoreAtom[];
};

type ContradictionSummary = {
  key: string;
  kind: string;
  message: string;
  atom_ids: string[];
};

type StatusReport = {
  root: string;
  total_atoms: number;
  entropy_score: number;
  draft_atoms: number;
  proposed_atoms: number;
  accepted_atoms: number;
  deprecated_atoms: number;
  contradictions: ContradictionSummary[];
  notes: string[];
};

type ValidationReport = {
  root: string;
  ok: boolean;
  issues: string[];
};

type MarkAtomInput = {
  title: string;
  body?: string;
  scope?: string;
  file_path?: string;
  validation_script?: string;
  kind: "decision" | "assumption" | "open_question" | "signal";
};

type SetStateInput = {
  atom_id: string;
  state: "draft" | "proposed" | "accepted" | "deprecated";
  reason: string;
  actor?: string;
};

type LogEntry = {
  id: string;
  level: "info" | "success" | "error";
  message: string;
  createdAt: string;
};

const stateDotClass = (state: string) => {
  switch (state) {
    case "accepted":
      return "bg-emerald-500 shadow-emerald-500";
    case "deprecated":
      return "bg-amber-500 shadow-amber-500";
    case "draft":
      return "bg-gray-500 shadow-gray-500";
    default:
      return "bg-[#007acc] shadow-[#007acc]";
  }
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

const humanize = (value: string) =>
  value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");

function App() {
  const [atoms, setAtoms] = useState<LoreAtom[]>([]);
  const [root, setRoot] = useState<string>("");
  const [projectPath, setProjectPath] = useState<string>("");
  const [selectedAtomId, setSelectedAtomId] = useState<string | null>(null);
  const [status, setStatus] = useState<StatusReport | null>(null);
  const [validation, setValidation] = useState<ValidationReport | null>(null);
  const [showCreateAtom, setShowCreateAtom] = useState(false);
  const [logs, setLogs] = useState<LogEntry[]>([]);

  const [newAtomTitle, setNewAtomTitle] = useState("");
  const [newAtomBody, setNewAtomBody] = useState("");
  const [newAtomScope, setNewAtomScope] = useState("");
  const [newAtomPath, setNewAtomPath] = useState("");
  const [newAtomKind, setNewAtomKind] = useState<
    "decision" | "assumption" | "open_question" | "signal"
  >("decision");

  const [targetState, setTargetState] = useState<
    "draft" | "proposed" | "accepted" | "deprecated"
  >("accepted");
  const [stateReason, setStateReason] = useState("");

  const [loading, setLoading] = useState(false);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string>("");

  const selectedAtom = atoms.find((atom) => atom.id === selectedAtomId) ?? null;

  const pushLog = (level: LogEntry["level"], message: string) => {
    setLogs((previous) =>
      [
        {
          id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
          level,
          message,
          createdAt: new Date().toLocaleTimeString(),
        },
        ...previous,
      ].slice(0, 60),
    );
  };

  const isMissingLoreError = error
    .toLowerCase()
    .includes("could not find a .lore workspace");

  const loadWorkspace = async (path: string) => {
    setLoading(true);
    setError("");

    try {
      const snapshot = await invoke<WorkspaceSnapshot>("load_workspace", {
        path,
      });

      setRoot(snapshot.root);
      setAtoms(snapshot.atoms);
      setSelectedAtomId(snapshot.atoms[0]?.id ?? null);
      pushLog("success", `Loaded workspace at ${snapshot.root}`);
      await refreshStatus(path, true);
    } catch (cause) {
      setRoot("");
      setAtoms([]);
      setStatus(null);
      setValidation(null);
      setSelectedAtomId(null);
      const message = cause instanceof Error ? cause.message : String(cause);
      setError(message);
      pushLog("error", `Workspace load failed: ${message}`);
    } finally {
      setLoading(false);
    }
  };

  const refreshStatus = async (path: string, silent = false) => {
    try {
      const report = await invoke<StatusReport>("workspace_status", { path });
      setStatus(report);
      if (!silent) {
        pushLog("info", `Status refreshed for ${report.root}`);
      }
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      if (!silent) {
        pushLog("error", `Status refresh failed: ${message}`);
      }
    }
  };

  const runValidate = async () => {
    if (!projectPath) {
      pushLog("error", "Choose a project path first.");
      return;
    }

    setWorking(true);
    try {
      const report = await invoke<ValidationReport>("validate_workspace", {
        path: projectPath,
      });
      setValidation(report);
      pushLog(
        report.ok ? "success" : "error",
        report.ok ? "Validation passed" : "Validation found issues",
      );
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      pushLog("error", `Validation failed: ${message}`);
    } finally {
      setWorking(false);
    }
  };

  const initializeWorkspace = async () => {
    if (!projectPath) {
      pushLog("error", "Choose a project path first.");
      return;
    }

    setWorking(true);
    try {
      const snapshot = await invoke<WorkspaceSnapshot>("init_workspace", {
        path: projectPath,
      });
      setRoot(snapshot.root);
      setAtoms(snapshot.atoms);
      setSelectedAtomId(snapshot.atoms[0]?.id ?? null);
      setError("");
      pushLog("success", `Initialized .lore workspace at ${snapshot.root}`);
      await refreshStatus(projectPath, true);
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      pushLog("error", `Init failed: ${message}`);
    } finally {
      setWorking(false);
    }
  };

  const createAtom = async () => {
    if (!projectPath) {
      pushLog("error", "Choose a project path first.");
      return;
    }

    if (!newAtomTitle.trim()) {
      pushLog("error", "Atom title is required.");
      return;
    }

    const payload: MarkAtomInput = {
      title: newAtomTitle.trim(),
      kind: newAtomKind,
      body: newAtomBody.trim() || undefined,
      scope: newAtomScope.trim() || undefined,
      file_path: newAtomPath.trim() || undefined,
    };

    setWorking(true);
    try {
      const snapshot = await invoke<WorkspaceSnapshot>("mark_atom", {
        path: projectPath,
        input: payload,
      });
      setRoot(snapshot.root);
      setAtoms(snapshot.atoms);
      setSelectedAtomId(snapshot.atoms[0]?.id ?? null);
      setNewAtomTitle("");
      setNewAtomBody("");
      setNewAtomScope("");
      setNewAtomPath("");
      setShowCreateAtom(false);
      pushLog("success", `Created new ${newAtomKind} atom`);
      await refreshStatus(projectPath, true);
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      pushLog("error", `Create atom failed: ${message}`);
    } finally {
      setWorking(false);
    }
  };

  const updateAtomState = async () => {
    if (!projectPath || !selectedAtom) {
      pushLog("error", "Select a project and atom first.");
      return;
    }

    if (!stateReason.trim()) {
      pushLog("error", "State transition reason is required.");
      return;
    }

    const payload: SetStateInput = {
      atom_id: selectedAtom.id,
      state: targetState,
      reason: stateReason.trim(),
    };

    setWorking(true);
    try {
      const snapshot = await invoke<WorkspaceSnapshot>("set_atom_state", {
        path: projectPath,
        input: payload,
      });
      setRoot(snapshot.root);
      setAtoms(snapshot.atoms);
      setStateReason("");
      pushLog(
        "success",
        `Transitioned atom ${selectedAtom.id} to ${targetState}`,
      );
      await refreshStatus(projectPath, true);
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      pushLog("error", `State transition failed: ${message}`);
    } finally {
      setWorking(false);
    }
  };

  const chooseProject = async () => {
    const picked = await open({
      directory: true,
      multiple: false,
      title: "Select a project folder",
    });

    if (!picked) {
      return;
    }

    const path = Array.isArray(picked) ? picked[0] : picked;

    if (!path) {
      return;
    }

    setProjectPath(path);
    setValidation(null);
    await loadWorkspace(path);
  };

  return (
    <div className="flex h-screen w-screen bg-[#1e1e1e] text-[#cccccc] font-sans selection:bg-[#264f78]">
      <div className="w-12 bg-[#252526] flex flex-col items-center py-4 space-y-4 border-r border-[#333333]">
        <div className="p-2 bg-[#37373d] text-white rounded cursor-pointer hover:bg-[#505050]">
          <Layers size={20} />
        </div>
        <div className="p-2 text-gray-400 cursor-pointer hover:text-white">
          <Folder size={20} />
        </div>
        <div className="p-2 text-gray-400 cursor-pointer hover:text-white">
          <Terminal size={20} />
        </div>
      </div>

      <div className="w-64 bg-[#252526] flex flex-col border-r border-[#333333]">
        <div className="border-b border-[#333333] px-4 py-3 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <span className="font-semibold text-sm">LOCAL REPO & LORE</span>
            <button
              className="rounded bg-[#0e639c] px-2 py-1 text-[11px] font-medium text-white hover:bg-[#1177bb] disabled:opacity-60"
              onClick={chooseProject}
              disabled={loading}
            >
              Choose
            </button>
          </div>
          <button
            className="w-full rounded border border-[#3c3c3c] bg-[#1f1f1f] px-2 py-1 text-left text-[11px] text-gray-300 hover:border-[#555] disabled:opacity-60"
            onClick={chooseProject}
            disabled={loading}
          >
            {projectPath || "Choose a project folder..."}
          </button>
          {isMissingLoreError ? (
            <button
              className="w-full rounded border border-[#2b5f2f] bg-[#1a2f1d] px-2 py-1 text-left text-[11px] text-emerald-200 hover:bg-[#204327] disabled:opacity-60"
              onClick={initializeWorkspace}
              disabled={working || loading}
            >
              Initialize .lore in this folder
            </button>
          ) : null}
          <span
            className="text-[10px] text-gray-500 truncate block"
            title={root}
          >
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
            <span>.lore ATOMS ({atoms.length})</span>
            <button
              className="text-gray-300 hover:text-white"
              onClick={() => setShowCreateAtom((value) => !value)}
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
                onChange={(event) => setNewAtomTitle(event.target.value)}
              />
              <textarea
                className="w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-xs text-gray-200"
                placeholder="Body (optional)"
                value={newAtomBody}
                onChange={(event) => setNewAtomBody(event.target.value)}
                rows={2}
              />
              <input
                className="w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-xs text-gray-200"
                placeholder="Scope (optional)"
                value={newAtomScope}
                onChange={(event) => setNewAtomScope(event.target.value)}
              />
              <input
                className="w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-xs text-gray-200"
                placeholder="File path (optional)"
                value={newAtomPath}
                onChange={(event) => setNewAtomPath(event.target.value)}
              />
              <select
                className="w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-xs text-gray-200"
                value={newAtomKind}
                onChange={(event) =>
                  setNewAtomKind(event.target.value as MarkAtomInput["kind"])
                }
              >
                <option value="decision">decision</option>
                <option value="assumption">assumption</option>
                <option value="open_question">open_question</option>
                <option value="signal">signal</option>
              </select>
              <button
                className="w-full rounded bg-[#0e639c] px-2 py-1 text-xs font-semibold text-white hover:bg-[#1177bb] disabled:opacity-60"
                onClick={createAtom}
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

          {atoms.length === 0 ? (
            <div className="px-5 py-2 text-xs text-gray-500 italic">
              {loading ? "Loading atoms..." : "No atoms found."}
            </div>
          ) : (
            atoms.map((atom) => (
              <button
                key={atom.id}
                className="w-full px-3 py-1 flex items-center space-x-2 text-left text-gray-300 cursor-pointer hover:bg-[#2a2d2e]"
                title={atom.id}
                onClick={() => setSelectedAtomId(atom.id)}
              >
                <Activity size={14} className={stateTextClass(atom.state)} />
                <span className="truncate">{atom.title}</span>
              </button>
            ))
          )}
        </div>
      </div>

      <div className="flex-1 flex bg-[#1e1e1e]">
        <div className="flex-1 flex flex-col border-r border-[#333333]">
          <div className="h-10 bg-[#252526] border-b border-[#333333] flex items-center px-4 font-semibold text-sm">
            <div className="flex w-full items-center justify-between gap-2">
              <span>Lore Graph Visualization</span>
              <div className="flex items-center gap-2">
                <button
                  className="rounded border border-[#3d3d3d] px-2 py-1 text-xs text-gray-200 hover:bg-[#2b2b2b] disabled:opacity-60"
                  onClick={() => projectPath && refreshStatus(projectPath)}
                  disabled={!projectPath || working || loading}
                >
                  <span className="inline-flex items-center gap-1">
                    <RefreshCw size={12} /> Refresh
                  </span>
                </button>
                <button
                  className="rounded border border-[#3d3d3d] px-2 py-1 text-xs text-gray-200 hover:bg-[#2b2b2b] disabled:opacity-60"
                  onClick={runValidate}
                  disabled={!projectPath || working || loading}
                >
                  <span className="inline-flex items-center gap-1">
                    <ShieldCheck size={12} /> Validate
                  </span>
                </button>
              </div>
            </div>
          </div>

          {status ? (
            <div className="grid grid-cols-4 gap-2 border-b border-[#333333] bg-[#202020] p-2 text-xs">
              <div className="rounded border border-[#383838] bg-[#181818] p-2">
                <div className="text-gray-400">Total Atoms</div>
                <div className="text-white text-sm font-semibold">
                  {status.total_atoms}
                </div>
              </div>
              <div className="rounded border border-[#383838] bg-[#181818] p-2">
                <div className="text-gray-400">Entropy</div>
                <div className="text-white text-sm font-semibold">
                  {status.entropy_score}/100
                </div>
              </div>
              <div className="rounded border border-[#383838] bg-[#181818] p-2">
                <div className="text-gray-400">Accepted</div>
                <div className="text-emerald-300 text-sm font-semibold">
                  {status.accepted_atoms}
                </div>
              </div>
              <div className="rounded border border-[#383838] bg-[#181818] p-2">
                <div className="text-gray-400">Contradictions</div>
                <div className="text-amber-300 text-sm font-semibold">
                  {status.contradictions.length}
                </div>
              </div>
            </div>
          ) : null}

          {validation ? (
            <div
              className={`mx-2 mt-2 rounded border px-3 py-2 text-xs ${validation.ok ? "border-emerald-900 bg-emerald-950/40 text-emerald-200" : "border-amber-900 bg-amber-950/40 text-amber-200"}`}
            >
              {validation.ok
                ? "Validation passed"
                : `Validation issues (${validation.issues.length})`}
            </div>
          ) : null}

          <div className="flex-1 p-4 overflow-y-auto min-h-0">
            {atoms.map((atom) => (
              <button
                key={atom.id}
                className="w-full flex items-center pl-4 py-2 hover:bg-[#2a2d2e] cursor-pointer mb-1 rounded text-left"
                onClick={() => setSelectedAtomId(atom.id)}
              >
                <div
                  className={`w-4 h-4 rounded-full mr-4 border-2 border-[#1e1e1e] shadow-[0_0_0_2px] ${stateDotClass(
                    atom.state,
                  )}`}
                ></div>
                <div className="flex-1">
                  <div className="font-medium text-[#e5e5e5]">{atom.title}</div>
                  <div className="text-xs text-gray-500">
                    {humanize(atom.kind)} • {humanize(atom.state)} •{" "}
                    {new Date(
                      atom.created_unix_seconds * 1000,
                    ).toLocaleString()}
                  </div>
                </div>
              </button>
            ))}

            {atoms.length === 0 && (
              <div className="text-gray-500 italic mt-10 text-center">
                Choose a project folder to inspect its .lore workspace
              </div>
            )}
          </div>

          <div className="h-36 border-t border-[#333333] bg-[#161616] p-2">
            <div className="mb-1 text-xs font-semibold text-gray-300">
              Activity Log
            </div>
            <div className="h-26 overflow-y-auto text-xs">
              {logs.length === 0 ? (
                <div className="text-gray-500">No operations yet.</div>
              ) : (
                logs.map((entry) => (
                  <div key={entry.id} className="mb-1">
                    <span className="text-gray-500">[{entry.createdAt}]</span>{" "}
                    <span
                      className={
                        entry.level === "error"
                          ? "text-red-300"
                          : entry.level === "success"
                            ? "text-emerald-300"
                            : "text-blue-300"
                      }
                    >
                      {entry.message}
                    </span>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>

        <div className="w-80 flex flex-col bg-[#1e1e1e]">
          <div className="h-10 bg-[#252526] border-b border-[#333333] flex items-center px-4 font-semibold text-sm">
            Atom Details
          </div>
          <div className="flex-1 p-4 overflow-y-auto text-sm">
            {selectedAtom ? (
              <>
                <div className="text-lg font-bold mb-2 text-white">
                  {selectedAtom.title}
                </div>
                <div className="text-gray-400 mb-4 border-b border-[#444] pb-2">
                  {humanize(selectedAtom.kind)} · {humanize(selectedAtom.state)}
                </div>
                <div className="bg-[#2d2d2d] p-3 rounded font-mono text-xs text-gray-300">
                  <div className="text-green-400 mb-2">
                    id: {selectedAtom.id}
                  </div>
                  <div className="mb-2">
                    <strong>Scope:</strong> {selectedAtom.scope || "Global"}
                  </div>
                  {selectedAtom.body && (
                    <div className="mt-2 text-gray-400 whitespace-pre-wrap">
                      {selectedAtom.body}
                    </div>
                  )}
                </div>

                <div className="mt-4 rounded border border-[#3a3a3a] bg-[#1f1f1f] p-3">
                  <div className="mb-2 text-xs font-semibold text-gray-300">
                    Lifecycle Transition
                  </div>
                  <select
                    className="mb-2 w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-xs text-gray-200"
                    value={targetState}
                    onChange={(event) =>
                      setTargetState(
                        event.target.value as SetStateInput["state"],
                      )
                    }
                  >
                    <option value="draft">draft</option>
                    <option value="proposed">proposed</option>
                    <option value="accepted">accepted</option>
                    <option value="deprecated">deprecated</option>
                  </select>
                  <textarea
                    className="mb-2 w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-xs text-gray-200"
                    rows={3}
                    placeholder="Reason for transition"
                    value={stateReason}
                    onChange={(event) => setStateReason(event.target.value)}
                  />
                  <button
                    className="w-full rounded bg-[#0e639c] px-2 py-1 text-xs font-semibold text-white hover:bg-[#1177bb] disabled:opacity-60"
                    onClick={updateAtomState}
                    disabled={working || loading}
                  >
                    Apply State
                  </button>
                </div>

                {status && status.contradictions.length > 0 ? (
                  <div className="mt-4 rounded border border-amber-900 bg-amber-950/50 p-3 text-xs text-amber-200">
                    <div className="mb-2 inline-flex items-center gap-1 font-semibold">
                      <AlertTriangle size={12} />
                      Contradictions
                    </div>
                    {status.contradictions.slice(0, 3).map((item) => (
                      <div key={`${item.key}-${item.kind}`} className="mb-2">
                        <div className="font-medium">{item.key}</div>
                        <div>{item.message}</div>
                      </div>
                    ))}
                  </div>
                ) : null}
              </>
            ) : (
              <div className="text-gray-500 italic mt-10 text-center">
                Pick a project folder, then select an atom from the graph
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
