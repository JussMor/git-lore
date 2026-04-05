import { useEffect, useMemo, useRef, useState } from "react";

type ConsoleTab = "activity" | "commands" | "tools";

type ProposeKind = "decision" | "assumption" | "open_question" | "signal";

type PreviewState = "draft" | "proposed" | "accepted" | "deprecated";

type LogEntry = {
  id: string;
  level: "info" | "success" | "error";
  message: string;
  createdAt: string;
};

type Props = {
  open: boolean;
  activeTab: ConsoleTab;
  logs: LogEntry[];
  commandText: string;
  commandBusy: boolean;
  defaultFilePath: string;
  onTabChange: (tab: ConsoleTab) => void;
  onToggleOpen: () => void;
  onCommandTextChange: (value: string) => void;
  onRunCommand: (command: string) => Promise<void> | void;
  onToolContext: (input: {
    filePath: string;
    cursorLine?: number;
  }) => Promise<unknown>;
  onToolMemorySearch: (input: {
    query: string;
    filePath?: string;
    cursorLine?: number;
    limit?: number;
  }) => Promise<unknown>;
  onToolPropose: (input: {
    filePath: string;
    cursorLine?: number;
    kind: ProposeKind;
    title?: string;
    body?: string;
    scope?: string;
    validationScript?: string;
    autofill: boolean;
  }) => Promise<unknown>;
  onToolStateSnapshot: () => Promise<unknown>;
  onToolMemoryPreflight: (input: { operation: string }) => Promise<unknown>;
  onToolStateTransitionPreview: (input: {
    atomId: string;
    targetState: PreviewState;
  }) => Promise<unknown>;
};

type ToolAction =
  | "context"
  | "memory_search"
  | "propose"
  | "state_snapshot"
  | "memory_preflight"
  | "state_preview";

const QUICK_COMMANDS = [
  {
    label: "Help",
    command: "help",
    className: "border-sky-800 bg-sky-950/40 text-sky-200",
  },
  {
    label: "Refresh",
    command: "refresh",
    className: "border-cyan-800 bg-cyan-950/40 text-cyan-200",
  },
  {
    label: "Validate",
    command: "validate",
    className: "border-emerald-800 bg-emerald-950/40 text-emerald-200",
  },
  {
    label: "Focus Signal",
    command: "focus signal",
    className: "border-rose-800 bg-rose-950/40 text-rose-200",
  },
  {
    label: "Filters All",
    command: "filters all",
    className: "border-amber-800 bg-amber-950/40 text-amber-200",
  },
  {
    label: "Filters None",
    command: "filters none",
    className: "border-gray-700 bg-gray-900/50 text-gray-200",
  },
];

const FIELD_CLASS =
  "w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-sm text-gray-200";

const SELECT_CLASS = `${FIELD_CLASS} appearance-none pr-8`;

const COLLAPSED_HEIGHT = 48;
const MIN_OPEN_HEIGHT = 220;

const maxOpenHeight = () => {
  if (typeof window === "undefined") {
    return 640;
  }
  return Math.max(320, Math.floor(window.innerHeight * 0.82));
};

export function ActivityConsole({
  open,
  activeTab,
  logs,
  commandText,
  commandBusy,
  defaultFilePath,
  onTabChange,
  onToggleOpen,
  onCommandTextChange,
  onRunCommand,
  onToolContext,
  onToolMemorySearch,
  onToolPropose,
  onToolStateSnapshot,
  onToolMemoryPreflight,
  onToolStateTransitionPreview,
}: Props) {
  const latestLog = logs[0];
  const [toolAction, setToolAction] = useState<ToolAction>("context");
  const [toolOutput, setToolOutput] = useState("Run a tool to see output.");
  const [toolRunning, setToolRunning] = useState(false);

  const [filePath, setFilePath] = useState("");
  const [cursorLineText, setCursorLineText] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [searchLimitText, setSearchLimitText] = useState("10");
  const [proposeKind, setProposeKind] = useState<ProposeKind>("decision");
  const [proposeTitle, setProposeTitle] = useState("");
  const [proposeBody, setProposeBody] = useState("");
  const [proposeScope, setProposeScope] = useState("");
  const [proposeValidation, setProposeValidation] = useState("");
  const [proposeAutofill, setProposeAutofill] = useState(true);
  const [preflightOperation, setPreflightOperation] = useState("edit");
  const [previewAtomId, setPreviewAtomId] = useState("");
  const [previewState, setPreviewState] = useState<PreviewState>("accepted");
  const [openHeight, setOpenHeight] = useState(420);
  const [resizing, setResizing] = useState(false);
  const dragStartYRef = useRef(0);
  const dragStartHeightRef = useRef(0);

  const busy = commandBusy || toolRunning;

  useEffect(() => {
    if (!resizing) {
      return;
    }

    const previousCursor = document.body.style.cursor;
    const previousUserSelect = document.body.style.userSelect;
    document.body.style.cursor = "ns-resize";
    document.body.style.userSelect = "none";

    const onPointerMove = (event: PointerEvent) => {
      const delta = dragStartYRef.current - event.clientY;
      const next = dragStartHeightRef.current + delta;
      const clamped = Math.max(
        MIN_OPEN_HEIGHT,
        Math.min(maxOpenHeight(), next),
      );
      setOpenHeight(clamped);
    };

    const stopResize = () => {
      setResizing(false);
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", stopResize);
    window.addEventListener("pointercancel", stopResize);

    return () => {
      document.body.style.cursor = previousCursor;
      document.body.style.userSelect = previousUserSelect;
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", stopResize);
      window.removeEventListener("pointercancel", stopResize);
    };
  }, [resizing]);

  const startResize = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!open) {
      return;
    }

    dragStartYRef.current = event.clientY;
    dragStartHeightRef.current = openHeight;
    setResizing(true);
  };

  const panelHeight = open
    ? Math.max(MIN_OPEN_HEIGHT, Math.min(maxOpenHeight(), openHeight))
    : COLLAPSED_HEIGHT;

  const effectiveFilePath = useMemo(
    () => (filePath.trim() ? filePath.trim() : defaultFilePath.trim()),
    [defaultFilePath, filePath],
  );

  const parsePositiveInt = (value: string): number | undefined => {
    const raw = value.trim();
    if (!raw) {
      return undefined;
    }
    const numeric = Number(raw);
    if (!Number.isFinite(numeric)) {
      return undefined;
    }
    return Math.max(0, Math.floor(numeric));
  };

  const formatOutput = (value: unknown) => {
    if (typeof value === "string") {
      return value;
    }
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return String(value);
    }
  };

  const runSelectedTool = async () => {
    if (busy) {
      return;
    }

    setToolRunning(true);
    try {
      let result: unknown;

      switch (toolAction) {
        case "context": {
          if (!effectiveFilePath) {
            setToolOutput("Provide file path for context tool.");
            return;
          }
          result = await onToolContext({
            filePath: effectiveFilePath,
            cursorLine: parsePositiveInt(cursorLineText),
          });
          break;
        }
        case "memory_search": {
          if (!searchQuery.trim()) {
            setToolOutput("Provide query for memory search.");
            return;
          }
          result = await onToolMemorySearch({
            query: searchQuery.trim(),
            filePath: effectiveFilePath || undefined,
            cursorLine: parsePositiveInt(cursorLineText),
            limit: parsePositiveInt(searchLimitText),
          });
          break;
        }
        case "propose": {
          if (!effectiveFilePath) {
            setToolOutput("Provide file path for propose.");
            return;
          }
          result = await onToolPropose({
            filePath: effectiveFilePath,
            cursorLine: parsePositiveInt(cursorLineText),
            kind: proposeKind,
            title: proposeTitle.trim() || undefined,
            body: proposeBody.trim() || undefined,
            scope: proposeScope.trim() || undefined,
            validationScript: proposeValidation.trim() || undefined,
            autofill: proposeAutofill,
          });
          break;
        }
        case "state_snapshot":
          result = await onToolStateSnapshot();
          break;
        case "memory_preflight":
          result = await onToolMemoryPreflight({
            operation: preflightOperation,
          });
          break;
        case "state_preview": {
          if (!previewAtomId.trim()) {
            setToolOutput("Provide atom id for transition preview.");
            return;
          }
          result = await onToolStateTransitionPreview({
            atomId: previewAtomId.trim(),
            targetState: previewState,
          });
          break;
        }
      }

      setToolOutput(formatOutput(result));
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      setToolOutput(`Tool failed: ${message}`);
    } finally {
      setToolRunning(false);
    }
  };

  return (
    <div
      className={`border-t border-[#333333] bg-[#141414] ${
        resizing ? "" : "transition-[height] duration-200"
      }`}
      style={{ height: `${panelHeight}px` }}
    >
      {open ? (
        <div
          className="h-2 cursor-row-resize border-b border-[#2a2a2a] bg-[#181818] hover:bg-[#1d2732]"
          onPointerDown={startResize}
          title="Drag to resize"
        />
      ) : null}
      <div className="flex h-12 items-center justify-between border-b border-[#2d2d2d] px-3">
        <div>
          <div className="text-xs font-semibold text-gray-200">
            Activity Console
          </div>
          <div className="text-[10px] text-gray-500">
            Cmd/Ctrl+K opens command deck
          </div>
        </div>
        <button
          className="rounded border border-[#3a3a3a] px-2 py-1 text-[10px] text-gray-300 hover:bg-[#1f1f1f]"
          onClick={onToggleOpen}
          type="button"
        >
          {open ? "Collapse" : "Expand"}
        </button>
      </div>

      {!open ? (
        <div className="px-3 py-2 text-xs text-gray-400">
          {latestLog
            ? `[${latestLog.createdAt}] ${latestLog.message}`
            : "No operations yet. Open the console to run commands."}
        </div>
      ) : (
        <div className="flex h-[calc(100%-48px)] flex-col">
          <div className="flex items-center gap-1 border-b border-[#2d2d2d] px-2 py-1 text-xs">
            <button
              className={`rounded px-2 py-1 ${
                activeTab === "activity"
                  ? "bg-[#0e639c]/30 text-blue-200"
                  : "text-gray-300 hover:bg-[#1f1f1f]"
              }`}
              onClick={() => onTabChange("activity")}
              type="button"
            >
              Activity
            </button>
            <button
              className={`rounded px-2 py-1 ${
                activeTab === "commands"
                  ? "bg-[#0e639c]/30 text-blue-200"
                  : "text-gray-300 hover:bg-[#1f1f1f]"
              }`}
              onClick={() => onTabChange("commands")}
              type="button"
            >
              Command Deck
            </button>
            <button
              className={`rounded px-2 py-1 ${
                activeTab === "tools"
                  ? "bg-[#0e639c]/30 text-blue-200"
                  : "text-gray-300 hover:bg-[#1f1f1f]"
              }`}
              onClick={() => onTabChange("tools")}
              type="button"
            >
              Tools
            </button>
          </div>

          {activeTab === "activity" ? (
            <div className="h-full overflow-y-auto px-3 py-2 text-xs">
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
          ) : activeTab === "commands" ? (
            <div className="grid h-full min-h-0 grid-cols-1 gap-2 p-2 md:grid-cols-[1fr_280px]">
              <div className="rounded border border-[#333] bg-[#181818] p-2 text-xs">
                <div className="mb-2 text-gray-400">Run command</div>
                <div className="flex items-center gap-2">
                  <input
                    className={FIELD_CLASS}
                    placeholder="Try: help, refresh, validate, focus signal"
                    value={commandText}
                    onChange={(event) =>
                      onCommandTextChange(event.target.value)
                    }
                    onKeyDown={(event) => {
                      if (event.key === "Enter") {
                        event.preventDefault();
                        onRunCommand(commandText);
                      }
                    }}
                    disabled={commandBusy}
                  />
                  <button
                    className="rounded bg-[#0e639c] px-2 py-1 font-semibold text-white hover:bg-[#1177bb] disabled:opacity-60"
                    onClick={() => onRunCommand(commandText)}
                    disabled={commandBusy || !commandText.trim()}
                    type="button"
                  >
                    Run
                  </button>
                </div>
                <div className="mt-2 text-[10px] text-gray-500">
                  You can run internal commands like "signal &lt;title&gt;" and
                  "set-state accepted &lt;reason&gt;".
                </div>
              </div>

              <div className="rounded border border-[#333] bg-[#181818] p-2 text-xs">
                <div className="mb-2 text-gray-400">Quick actions</div>
                <div className="grid grid-cols-2 gap-1">
                  {QUICK_COMMANDS.map((item) => (
                    <button
                      key={item.command}
                      className={`rounded border px-2 py-1 text-left ${item.className}`}
                      onClick={() => onRunCommand(item.command)}
                      disabled={commandBusy}
                      type="button"
                    >
                      {item.label}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          ) : (
            <div className="grid h-full min-h-0 grid-cols-1 gap-2 p-2 text-xs xl:grid-cols-[1.3fr_1fr]">
              <div className="min-h-0 overflow-y-auto rounded border border-[#333] bg-[#181818] p-2">
                <div className="mb-2 flex flex-wrap items-center gap-2">
                  <select
                    className={`${SELECT_CLASS} min-w-[240px] flex-1`}
                    value={toolAction}
                    onChange={(event) =>
                      setToolAction(event.target.value as ToolAction)
                    }
                    disabled={busy}
                  >
                    <option value="context">git_lore_context</option>
                    <option value="memory_search">
                      git_lore_memory_search
                    </option>
                    <option value="propose">git_lore_propose (guarded)</option>
                    <option value="state_snapshot">
                      git_lore_state_snapshot
                    </option>
                    <option value="memory_preflight">
                      git_lore_memory_preflight
                    </option>
                    <option value="state_preview">
                      git_lore_state_transition_preview
                    </option>
                  </select>
                  <button
                    className="rounded bg-[#0e639c] px-2 py-1 font-semibold text-white hover:bg-[#1177bb] disabled:opacity-60"
                    onClick={runSelectedTool}
                    disabled={busy}
                    type="button"
                  >
                    {busy ? "Running..." : "Run Tool"}
                  </button>
                </div>

                {(toolAction === "context" ||
                  toolAction === "memory_search" ||
                  toolAction === "propose") && (
                  <div className="mb-2">
                    <div className="mb-1 text-gray-400">File path</div>
                    <input
                      className={FIELD_CLASS}
                      placeholder={defaultFilePath || "src/main.rs"}
                      value={filePath}
                      onChange={(event) => setFilePath(event.target.value)}
                      disabled={busy}
                    />
                  </div>
                )}

                {(toolAction === "context" ||
                  toolAction === "memory_search" ||
                  toolAction === "propose") && (
                  <div className="mb-2">
                    <div className="mb-1 text-gray-400">
                      Cursor line (optional)
                    </div>
                    <input
                      className={FIELD_CLASS}
                      placeholder="120"
                      value={cursorLineText}
                      onChange={(event) =>
                        setCursorLineText(event.target.value)
                      }
                      disabled={busy}
                    />
                  </div>
                )}

                {toolAction === "memory_search" && (
                  <>
                    <div className="mb-2">
                      <div className="mb-1 text-gray-400">Query</div>
                      <input
                        className={FIELD_CLASS}
                        placeholder="auth architecture"
                        value={searchQuery}
                        onChange={(event) => setSearchQuery(event.target.value)}
                        disabled={busy}
                      />
                    </div>
                    <div className="mb-2">
                      <div className="mb-1 text-gray-400">Limit</div>
                      <input
                        className={FIELD_CLASS}
                        placeholder="10"
                        value={searchLimitText}
                        onChange={(event) =>
                          setSearchLimitText(event.target.value)
                        }
                        disabled={busy}
                      />
                    </div>
                  </>
                )}

                {toolAction === "propose" && (
                  <>
                    <div className="mb-2">
                      <div className="mb-1 text-gray-400">Kind</div>
                      <select
                        className={SELECT_CLASS}
                        value={proposeKind}
                        onChange={(event) =>
                          setProposeKind(event.target.value as ProposeKind)
                        }
                        disabled={busy}
                      >
                        <option value="decision">decision</option>
                        <option value="assumption">assumption</option>
                        <option value="open_question">open_question</option>
                        <option value="signal">signal</option>
                      </select>
                    </div>
                    <div className="mb-2">
                      <div className="mb-1 text-gray-400">Title</div>
                      <input
                        className={FIELD_CLASS}
                        placeholder="Rule title"
                        value={proposeTitle}
                        onChange={(event) =>
                          setProposeTitle(event.target.value)
                        }
                        disabled={busy}
                      />
                    </div>
                    <div className="mb-2">
                      <div className="mb-1 text-gray-400">Body</div>
                      <textarea
                        className={FIELD_CLASS}
                        rows={3}
                        placeholder="Rationale"
                        value={proposeBody}
                        onChange={(event) => setProposeBody(event.target.value)}
                        disabled={busy}
                      />
                    </div>
                    <div className="mb-2">
                      <div className="mb-1 text-gray-400">Scope (optional)</div>
                      <input
                        className={FIELD_CLASS}
                        placeholder="src/auth"
                        value={proposeScope}
                        onChange={(event) =>
                          setProposeScope(event.target.value)
                        }
                        disabled={busy}
                      />
                    </div>
                    <div className="mb-2">
                      <div className="mb-1 text-gray-400">
                        Validation command (optional)
                      </div>
                      <input
                        className={FIELD_CLASS}
                        placeholder="cargo test -p auth"
                        value={proposeValidation}
                        onChange={(event) =>
                          setProposeValidation(event.target.value)
                        }
                        disabled={busy}
                      />
                    </div>
                    <label className="inline-flex items-center gap-2 text-gray-300">
                      <input
                        type="checkbox"
                        checked={proposeAutofill}
                        onChange={(event) =>
                          setProposeAutofill(event.target.checked)
                        }
                        disabled={busy}
                      />
                      Autofill title/body/scope when missing
                    </label>
                  </>
                )}

                {toolAction === "memory_preflight" && (
                  <div className="mb-2">
                    <div className="mb-1 text-gray-400">Operation</div>
                    <select
                      className={SELECT_CLASS}
                      value={preflightOperation}
                      onChange={(event) =>
                        setPreflightOperation(event.target.value)
                      }
                      disabled={busy}
                    >
                      <option value="edit">edit</option>
                      <option value="commit">commit</option>
                      <option value="sync">sync</option>
                    </select>
                  </div>
                )}

                {toolAction === "state_preview" && (
                  <>
                    <div className="mb-2">
                      <div className="mb-1 text-gray-400">Atom ID</div>
                      <input
                        className={FIELD_CLASS}
                        placeholder="atom-123..."
                        value={previewAtomId}
                        onChange={(event) =>
                          setPreviewAtomId(event.target.value)
                        }
                        disabled={busy}
                      />
                    </div>
                    <div className="mb-2">
                      <div className="mb-1 text-gray-400">Target state</div>
                      <select
                        className={SELECT_CLASS}
                        value={previewState}
                        onChange={(event) =>
                          setPreviewState(event.target.value as PreviewState)
                        }
                        disabled={busy}
                      >
                        <option value="draft">draft</option>
                        <option value="proposed">proposed</option>
                        <option value="accepted">accepted</option>
                        <option value="deprecated">deprecated</option>
                      </select>
                    </div>
                  </>
                )}
              </div>

              <div className="min-h-0 overflow-hidden rounded border border-[#333] bg-[#101214] p-2">
                <div className="mb-2 text-gray-400">Tool output</div>
                <pre className="h-full max-h-[260px] overflow-auto whitespace-pre-wrap text-[11px] text-gray-200 xl:max-h-full">
                  {toolOutput}
                </pre>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
