import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Folder, Layers, RefreshCw, ShieldCheck, Terminal } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import "./App.css";
import { ActivityConsole } from "./components/ActivityConsole";
import { AtomDetailsPanel } from "./components/AtomDetailsPanel";
import { LoreBrainGraph } from "./components/LoreBrainGraph";
import { LoreSidebar } from "./components/LoreSidebar";

//This file should not be more than 1k lines
export interface LoreAtom {
  id: string;
  kind: string;
  state: string;
  title: string;
  body?: string;
  scope?: string;
  path?: string;
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

type GitDecisionSummary = {
  commit_hash: string;
  subject: string;
  trailer_value: string;
};

type AtomContextReport = {
  atom_id: string;
  file_path?: string;
  scope?: string;
  constraints: string[];
  historical_decisions: GitDecisionSummary[];
};

type CommitDiffReport = {
  commit_hash: string;
  subject: string;
  diff: string;
  truncated: boolean;
};

type GitContextCommit = {
  commit_hash: string;
  subject: string;
  trailer_values: string[];
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

type AtomStateKey = "accepted" | "proposed" | "draft" | "deprecated";

type StateFilterMap = Record<AtomStateKey, boolean>;

type LogEntry = {
  id: string;
  level: "info" | "success" | "error";
  message: string;
  createdAt: string;
};

type ConsoleTab = "activity" | "commands" | "tools";

const normalizeState = (state: string): AtomStateKey => {
  const value = state.trim().toLowerCase();
  if (value === "accepted") {
    return "accepted";
  }
  if (value === "draft") {
    return "draft";
  }
  if (value === "deprecated") {
    return "deprecated";
  }
  return "proposed";
};

const FILTER_LABELS: Array<{ key: AtomStateKey; label: string }> = [
  { key: "accepted", label: "Accepted" },
  { key: "proposed", label: "Proposed" },
  { key: "draft", label: "Draft" },
  { key: "deprecated", label: "Deprecated" },
];

const LAST_PROJECT_PATH_KEY = "glore.lastProjectPath";

const readPersistedProjectPath = () => {
  try {
    return localStorage.getItem(LAST_PROJECT_PATH_KEY)?.trim() ?? "";
  } catch {
    return "";
  }
};

const persistProjectPath = (path: string) => {
  try {
    const next = path.trim();
    if (!next) {
      localStorage.removeItem(LAST_PROJECT_PATH_KEY);
      return;
    }
    localStorage.setItem(LAST_PROJECT_PATH_KEY, next);
  } catch {
    // Ignore persistence errors in restricted environments.
  }
};

const tokenizeCommand = (input: string) =>
  (input.match(/"[^"]*"|'[^']*'|\S+/g) ?? []).map((token) =>
    token.replace(/^['"]|['"]$/g, ""),
  );

function App() {
  const [atoms, setAtoms] = useState<LoreAtom[]>([]);
  const [root, setRoot] = useState<string>("");
  const [projectPath, setProjectPath] = useState<string>(() =>
    readPersistedProjectPath(),
  );
  const [selectedAtomId, setSelectedAtomId] = useState<string | null>(null);
  const [status, setStatus] = useState<StatusReport | null>(null);
  const [validation, setValidation] = useState<ValidationReport | null>(null);
  const [showCreateAtom, setShowCreateAtom] = useState(false);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [activityDockOpen, setActivityDockOpen] = useState(false);
  const [activityDockTab, setActivityDockTab] =
    useState<ConsoleTab>("activity");
  const [commandText, setCommandText] = useState("");
  const [commandRunning, setCommandRunning] = useState(false);

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
  const [stateFilters, setStateFilters] = useState<StateFilterMap>({
    accepted: true,
    proposed: true,
    draft: true,
    deprecated: true,
  });

  const [loading, setLoading] = useState(false);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string>("");
  const [atomContext, setAtomContext] = useState<AtomContextReport | null>(
    null,
  );
  const [contextLoading, setContextLoading] = useState(false);
  const [selectedCommitHash, setSelectedCommitHash] = useState<string | null>(
    null,
  );
  const [selectedCommitDiff, setSelectedCommitDiff] =
    useState<CommitDiffReport | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);
  const [diffError, setDiffError] = useState<string>("");

  const filteredAtoms = useMemo(
    () =>
      atoms.filter((atom) => {
        const state = normalizeState(atom.state);
        return stateFilters[state];
      }),
    [atoms, stateFilters],
  );

  const selectedAtom = atoms.find((atom) => atom.id === selectedAtomId) ?? null;

  const gitContextCommits = useMemo<GitContextCommit[]>(() => {
    if (!atomContext) {
      return [];
    }

    const grouped = new Map<string, GitContextCommit>();

    for (const decision of atomContext.historical_decisions) {
      const existing = grouped.get(decision.commit_hash);

      if (!existing) {
        grouped.set(decision.commit_hash, {
          commit_hash: decision.commit_hash,
          subject: decision.subject,
          trailer_values: [decision.trailer_value],
        });
        continue;
      }

      if (!existing.trailer_values.includes(decision.trailer_value)) {
        existing.trailer_values.push(decision.trailer_value);
      }
    }

    return [...grouped.values()];
  }, [atomContext]);

  useEffect(() => {
    if (filteredAtoms.length === 0) {
      if (selectedAtomId !== null) {
        setSelectedAtomId(null);
      }
      return;
    }

    const exists = filteredAtoms.some((atom) => atom.id === selectedAtomId);
    if (!exists) {
      setSelectedAtomId(filteredAtoms[0].id);
    }
  }, [filteredAtoms, selectedAtomId]);

  const toggleFilter = (key: AtomStateKey) => {
    setStateFilters((previous) => ({ ...previous, [key]: !previous[key] }));
  };

  const enableAllFilters = () => {
    setStateFilters({
      accepted: true,
      proposed: true,
      draft: true,
      deprecated: true,
    });
  };

  const clearAllFilters = () => {
    setStateFilters({
      accepted: false,
      proposed: false,
      draft: false,
      deprecated: false,
    });
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setActivityDockOpen(true);
        setActivityDockTab("commands");
      }

      if (event.key === "Escape" && activityDockOpen) {
        setActivityDockOpen(false);
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [activityDockOpen]);

  useEffect(() => {
    if (!selectedAtom || !projectPath) {
      setAtomContext(null);
      setSelectedCommitHash(null);
      setSelectedCommitDiff(null);
      setDiffError("");
      return;
    }

    const fetchContext = async () => {
      setContextLoading(true);
      try {
        const context = await invoke<AtomContextReport>("atom_context", {
          path: projectPath,
          input: { atom_id: selectedAtom.id },
        });
        setAtomContext(context);
      } catch {
        setAtomContext(null);
      } finally {
        setContextLoading(false);
      }
    };

    fetchContext();
  }, [projectPath, selectedAtom]);

  useEffect(() => {
    if (gitContextCommits.length === 0) {
      setSelectedCommitHash(null);
      setSelectedCommitDiff(null);
      setDiffError("");
      return;
    }

    setSelectedCommitHash((previous) => {
      if (
        previous &&
        gitContextCommits.some((commit) => commit.commit_hash === previous)
      ) {
        return previous;
      }
      return gitContextCommits[0].commit_hash;
    });
  }, [gitContextCommits]);

  useEffect(() => {
    if (!projectPath || !selectedCommitHash) {
      setSelectedCommitDiff(null);
      setDiffError("");
      return;
    }

    const fetchDiff = async () => {
      setDiffLoading(true);
      setDiffError("");

      try {
        const report = await invoke<CommitDiffReport>("commit_diff", {
          path: projectPath,
          input: {
            commit_hash: selectedCommitHash,
            file_path:
              atomContext?.file_path ?? selectedAtom?.path ?? undefined,
          },
        });
        setSelectedCommitDiff(report);
      } catch (cause) {
        const message = cause instanceof Error ? cause.message : String(cause);
        setSelectedCommitDiff(null);
        setDiffError(message);
      } finally {
        setDiffLoading(false);
      }
    };

    fetchDiff();
  }, [
    projectPath,
    selectedCommitHash,
    atomContext?.file_path,
    selectedAtom?.path,
  ]);

  useEffect(() => {
    persistProjectPath(projectPath);
  }, [projectPath]);

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

  const openAtomFileInEditor = async (filePath: string) => {
    const nextPath = filePath.trim();
    if (!projectPath) {
      pushLog("error", "Choose a project path first.");
      return;
    }
    if (!nextPath) {
      pushLog("error", "This atom has no file path.");
      return;
    }

    try {
      await invoke<string>("open_file_in_editor", {
        path: projectPath,
        input: {
          file_path: nextPath,
          line: 1,
        },
      });
      pushLog("success", `Opened file in VS Code: ${nextPath}`);
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      pushLog("error", `Open file failed: ${message}`);
    }
  };

  useEffect(() => {
    if (!projectPath) {
      return;
    }

    setValidation(null);
    void loadWorkspace(projectPath);
  }, []);

  const createSignalFromCommand = async (title: string) => {
    if (!projectPath) {
      pushLog("error", "Choose a project path first.");
      return;
    }

    const payload: MarkAtomInput = {
      title: title.trim(),
      kind: "signal",
    };

    const snapshot = await invoke<WorkspaceSnapshot>("mark_atom", {
      path: projectPath,
      input: payload,
    });

    setRoot(snapshot.root);
    setAtoms(snapshot.atoms);
    setSelectedAtomId(snapshot.atoms[0]?.id ?? null);
    pushLog("success", `Created signal atom: ${title.trim()}`);
    await refreshStatus(projectPath, true);
  };

  const setStateFromCommand = async (
    nextState: SetStateInput["state"],
    reason: string,
  ) => {
    if (!projectPath || !selectedAtom) {
      pushLog("error", "Select a project and atom first.");
      return;
    }

    if (!reason.trim()) {
      pushLog("error", "Provide a reason after the target state.");
      return;
    }

    const snapshot = await invoke<WorkspaceSnapshot>("set_atom_state", {
      path: projectPath,
      input: {
        atom_id: selectedAtom.id,
        state: nextState,
        reason: reason.trim(),
      } satisfies SetStateInput,
    });

    setRoot(snapshot.root);
    setAtoms(snapshot.atoms);
    pushLog("success", `Transitioned atom to ${nextState}`);
    await refreshStatus(projectPath, true);
  };

  const executeConsoleCommand = async (raw: string) => {
    const input = raw.trim();
    if (!input || commandRunning) {
      return;
    }

    if (input.toLowerCase() !== "clear-log") {
      pushLog("info", `$ ${input}`);
    }

    setCommandText("");
    setCommandRunning(true);

    const tokens = tokenizeCommand(input);
    const command = tokens[0]?.toLowerCase() ?? "";
    const args = tokens.slice(1);

    try {
      switch (command) {
        case "help":
          pushLog(
            "info",
            "Commands: help | refresh | validate | choose | init | activity | commands | focus signal | filters all|none | signal <title> | set-state <state> <reason> | clear-log",
          );
          setActivityDockOpen(true);
          setActivityDockTab("activity");
          break;
        case "refresh":
          if (!projectPath) {
            pushLog("error", "Choose a project path first.");
            break;
          }
          await refreshStatus(projectPath);
          break;
        case "validate":
          await runValidate();
          break;
        case "choose":
        case "open":
          await chooseProject();
          break;
        case "init":
          await initializeWorkspace();
          break;
        case "activity":
          setActivityDockOpen(true);
          setActivityDockTab("activity");
          break;
        case "commands":
          setActivityDockOpen(true);
          setActivityDockTab("commands");
          break;
        case "filters": {
          const mode = args[0]?.toLowerCase();
          if (mode === "all") {
            enableAllFilters();
            pushLog("success", "Enabled all state filters.");
            break;
          }
          if (mode === "none") {
            clearAllFilters();
            pushLog("success", "Disabled all state filters.");
            break;
          }
          pushLog("error", "Usage: filters all | filters none");
          break;
        }
        case "focus": {
          if ((args[0] ?? "").toLowerCase() !== "signal") {
            pushLog("error", "Usage: focus signal");
            break;
          }

          const signal = atoms.find(
            (atom) => atom.kind.trim().toLowerCase() === "signal",
          );
          if (!signal) {
            pushLog("error", "No signal atom found in this workspace.");
            break;
          }

          setSelectedAtomId(signal.id);
          setActivityDockOpen(true);
          setActivityDockTab("activity");
          pushLog("success", `Focused signal: ${signal.title}`);
          break;
        }
        case "signal": {
          const title = args.join(" ").trim();
          if (!title) {
            pushLog("error", "Usage: signal <title>");
            break;
          }
          await createSignalFromCommand(title);
          break;
        }
        case "set-state": {
          const rawState = (args[0] ?? "").toLowerCase();
          const reason = args.slice(1).join(" ").trim();
          if (
            rawState !== "draft" &&
            rawState !== "proposed" &&
            rawState !== "accepted" &&
            rawState !== "deprecated"
          ) {
            pushLog(
              "error",
              "Usage: set-state <draft|proposed|accepted|deprecated> <reason>",
            );
            break;
          }
          await setStateFromCommand(rawState, reason);
          break;
        }
        case "clear-log":
          setLogs([]);
          break;
        default:
          pushLog(
            "error",
            `Unknown command: ${command || input}. Run \"help\" for options.`,
          );
      }
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      pushLog("error", `Command failed: ${message}`);
    } finally {
      setCommandRunning(false);
    }
  };

  return (
    <div className="relative flex h-screen w-screen overflow-hidden bg-[#1e1e1e] text-[#cccccc] font-sans selection:bg-[#264f78]">
      <div className="w-12 bg-[#252526] flex flex-col items-center py-4 space-y-4 border-r border-[#333333]">
        <button
          className="p-2 bg-[#37373d] text-white rounded cursor-pointer hover:bg-[#505050]"
          onClick={() => {
            setActivityDockOpen(true);
            setActivityDockTab("activity");
          }}
          title="Open activity"
          type="button"
        >
          <Layers size={20} />
        </button>
        <button
          className="p-2 text-gray-400 cursor-pointer hover:text-white"
          onClick={chooseProject}
          title="Choose project"
          type="button"
        >
          <Folder size={20} />
        </button>
        <button
          className={`p-2 cursor-pointer rounded ${
            activityDockOpen && activityDockTab === "commands"
              ? "bg-[#0e639c] text-white"
              : "text-gray-400 hover:text-white"
          }`}
          onClick={() => {
            setActivityDockOpen(true);
            setActivityDockTab("commands");
          }}
          title="Open command deck"
          type="button"
        >
          <Terminal size={20} />
        </button>
      </div>

      <LoreSidebar
        projectPath={projectPath}
        loading={loading}
        working={working}
        isMissingLoreError={isMissingLoreError}
        root={root}
        error={error}
        atomsCount={atoms.length}
        filteredAtoms={filteredAtoms}
        showCreateAtom={showCreateAtom}
        newAtomTitle={newAtomTitle}
        newAtomBody={newAtomBody}
        newAtomScope={newAtomScope}
        newAtomPath={newAtomPath}
        newAtomKind={newAtomKind}
        onChooseProject={chooseProject}
        onInitializeWorkspace={initializeWorkspace}
        onToggleCreateAtom={() => setShowCreateAtom((value) => !value)}
        onNewAtomTitleChange={setNewAtomTitle}
        onNewAtomBodyChange={setNewAtomBody}
        onNewAtomScopeChange={setNewAtomScope}
        onNewAtomPathChange={setNewAtomPath}
        onNewAtomKindChange={setNewAtomKind}
        onCreateAtom={createAtom}
        onSelectAtom={setSelectedAtomId}
      />

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

          <div className="flex items-center justify-between gap-2 border-b border-[#333333] bg-[#1d1f21] px-3 py-2 text-xs">
            <div className="flex flex-wrap items-center gap-2">
              {FILTER_LABELS.map((item) => {
                const active = stateFilters[item.key];
                return (
                  <button
                    key={item.key}
                    className={`rounded border px-2 py-1 ${
                      active
                        ? "border-[#0e639c] bg-[#0e639c]/25 text-blue-200"
                        : "border-[#3d3d3d] text-gray-300 hover:bg-[#2b2b2b]"
                    }`}
                    onClick={() => toggleFilter(item.key)}
                  >
                    {item.label}
                  </button>
                );
              })}
            </div>
            <div className="flex items-center gap-1">
              <button
                className="rounded border border-[#3d3d3d] px-2 py-1 text-gray-300 hover:bg-[#2b2b2b]"
                onClick={enableAllFilters}
              >
                All
              </button>
              <button
                className="rounded border border-[#3d3d3d] px-2 py-1 text-gray-300 hover:bg-[#2b2b2b]"
                onClick={clearAllFilters}
              >
                None
              </button>
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

          <div className="flex-1 p-3 min-h-0">
            <LoreBrainGraph
              atoms={filteredAtoms}
              selectedAtomId={selectedAtomId}
              onSelectAtom={setSelectedAtomId}
            />
          </div>

          <ActivityConsole
            open={activityDockOpen}
            activeTab={activityDockTab}
            logs={logs}
            commandText={commandText}
            commandBusy={commandRunning || loading || working}
            defaultFilePath={selectedAtom?.path ?? ""}
            onTabChange={setActivityDockTab}
            onToggleOpen={() => setActivityDockOpen((value) => !value)}
            onCommandTextChange={setCommandText}
            onRunCommand={executeConsoleCommand}
            onToolContext={async () =>
              "Tool context is not wired in this UI mode."
            }
            onToolMemorySearch={async () =>
              "Tool memory search is not wired in this UI mode."
            }
            onToolPropose={async () =>
              "Tool propose is not wired in this UI mode."
            }
            onToolStateSnapshot={async () =>
              "Tool state snapshot is not wired in this UI mode."
            }
            onToolMemoryPreflight={async () =>
              "Tool memory preflight is not wired in this UI mode."
            }
            onToolStateTransitionPreview={async () =>
              "Tool state preview is not wired in this UI mode."
            }
          />
        </div>

        <AtomDetailsPanel
          selectedAtom={selectedAtom}
          contextLoading={contextLoading}
          gitContextCommits={gitContextCommits}
          selectedCommitHash={selectedCommitHash}
          onSelectCommitHash={setSelectedCommitHash}
          diffLoading={diffLoading}
          diffError={diffError}
          selectedCommitDiff={selectedCommitDiff}
          targetState={targetState}
          onTargetStateChange={setTargetState}
          stateReason={stateReason}
          onStateReasonChange={setStateReason}
          onApplyState={updateAtomState}
          onOpenFile={openAtomFileInEditor}
          working={working}
          loading={loading}
        />
      </div>
    </div>
  );
}

export default App;
