type ConsoleTab = "activity" | "commands";

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
  onTabChange: (tab: ConsoleTab) => void;
  onToggleOpen: () => void;
  onCommandTextChange: (value: string) => void;
  onRunCommand: (command: string) => Promise<void> | void;
};

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

export function ActivityConsole({
  open,
  activeTab,
  logs,
  commandText,
  commandBusy,
  onTabChange,
  onToggleOpen,
  onCommandTextChange,
  onRunCommand,
}: Props) {
  const latestLog = logs[0];

  return (
    <div
      className={`border-t border-[#333333] bg-[#141414] transition-[height] duration-200 ${
        open ? "h-56" : "h-12"
      }`}
    >
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
          ) : (
            <div className="grid h-full grid-cols-[1fr_280px] gap-2 p-2">
              <div className="rounded border border-[#333] bg-[#181818] p-2 text-xs">
                <div className="mb-2 text-gray-400">Run command</div>
                <div className="flex items-center gap-2">
                  <input
                    className="w-full rounded border border-[#404040] bg-[#111111] px-2 py-1 text-gray-200"
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
          )}
        </div>
      )}
    </div>
  );
}
