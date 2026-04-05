import {
  ExternalLink,
  FileCode2,
  Hash,
  NotebookText,
  Radar,
} from "lucide-react";
import { useEffect, useState } from "react";
import { GitDiffDrawer } from "./GitDiffDrawer";

type LoreAtomLike = {
  id: string;
  kind: string;
  state: string;
  title: string;
  body?: string;
  scope?: string;
  path?: string;
};

type GitContextCommit = {
  commit_hash: string;
  subject: string;
  trailer_values: string[];
};

type CommitDiffReport = {
  commit_hash: string;
  subject: string;
  diff: string;
  truncated: boolean;
};

type AtomStateValue = "draft" | "proposed" | "accepted" | "deprecated";

type TransitionOption = {
  value: AtomStateValue;
  label: string;
};

type Props = {
  selectedAtom: LoreAtomLike | null;
  contextLoading: boolean;
  gitContextCommits: GitContextCommit[];
  selectedCommitHash: string | null;
  onSelectCommitHash: (value: string) => void;
  diffLoading: boolean;
  diffError: string;
  selectedCommitDiff: CommitDiffReport | null;
  targetState: AtomStateValue | "";
  onTargetStateChange: (value: AtomStateValue | "") => void;
  stateReason: string;
  onStateReasonChange: (value: string) => void;
  onApplyState: () => void;
  onOpenFile: (filePath: string) => Promise<void> | void;
  working: boolean;
  loading: boolean;
};

const shortHash = (value: string) => value.slice(0, 7);

const humanize = (value: string) =>
  value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");

const stateBadgeClass = (state: string) => {
  const normalized = state.trim().toLowerCase();
  switch (normalized) {
    case "accepted":
      return "border-emerald-700/50 bg-emerald-950/20 text-emerald-200";
    case "deprecated":
      return "border-amber-700/50 bg-amber-950/20 text-amber-200";
    case "draft":
      return "border-slate-600/70 bg-slate-800/60 text-slate-200";
    default:
      return "border-sky-700/50 bg-sky-950/20 text-sky-200";
  }
};

const kindBadgeClass = (kind: string) => {
  const normalized = kind.trim().toLowerCase();
  switch (normalized) {
    case "decision":
      return "border-indigo-700/50 bg-indigo-950/20 text-indigo-200";
    case "assumption":
      return "border-cyan-700/50 bg-cyan-950/20 text-cyan-200";
    case "open_question":
      return "border-amber-700/50 bg-amber-950/20 text-amber-200";
    case "signal":
      return "border-rose-700/50 bg-rose-950/20 text-rose-200";
    default:
      return "border-slate-700/70 bg-slate-900/60 text-slate-200";
  }
};

const commitTooltipText = (commit: GitContextCommit) =>
  [
    commit.subject,
    `Commit: ${commit.commit_hash}`,
    `Lore refs: ${commit.trailer_values.length}`,
    ...commit.trailer_values.map((value, index) => `${index + 1}. ${value}`),
  ].join("\n");

const stripTrailerPrefix = (value: string) =>
  value.replace(/^\[[^\]]+\]\s*/, "").trim();

const getAllowedTransitions = (currentState: string): TransitionOption[] => {
  const normalized = currentState.trim().toLowerCase();

  switch (normalized) {
    case "draft":
      return [
        { value: "proposed", label: "proposed" },
        { value: "deprecated", label: "deprecated" },
      ];
    case "proposed":
      return [
        { value: "accepted", label: "accepted" },
        { value: "deprecated", label: "deprecated" },
      ];
    case "accepted":
      return [{ value: "deprecated", label: "deprecated" }];
    default:
      return [];
  }
};

export function AtomDetailsPanel({
  selectedAtom,
  contextLoading,
  gitContextCommits,
  selectedCommitHash,
  onSelectCommitHash,
  diffLoading,
  diffError,
  selectedCommitDiff,
  targetState,
  onTargetStateChange,
  stateReason,
  onStateReasonChange,
  onApplyState,
  onOpenFile,
  working,
  loading,
}: Props) {
  const [diffDrawerOpen, setDiffDrawerOpen] = useState(false);
  const [openFileBusy, setOpenFileBusy] = useState(false);
  const [openFileError, setOpenFileError] = useState("");

  useEffect(() => {
    if (!selectedAtom) {
      setDiffDrawerOpen(false);
      setOpenFileError("");
      setOpenFileBusy(false);
    }
  }, [selectedAtom]);

  const selectedPath = selectedAtom?.path?.trim() ?? "";
  const transitionOptions = selectedAtom
    ? getAllowedTransitions(selectedAtom.state)
    : [];
  const selectedTransitionIsAllowed =
    targetState !== "" &&
    transitionOptions.some((option) => option.value === targetState);

  const handleOpenPath = async () => {
    if (!selectedPath || openFileBusy) {
      return;
    }

    setOpenFileBusy(true);
    setOpenFileError("");

    try {
      await Promise.resolve(onOpenFile(selectedPath));
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      setOpenFileError(message);
    } finally {
      setOpenFileBusy(false);
    }
  };

  useEffect(() => {
    if (!diffDrawerOpen) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setDiffDrawerOpen(false);
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [diffDrawerOpen]);

  return (
    <>
      <div className="w-80 flex flex-col bg-[#252526]">
        <div className="flex h-10 items-center border-b border-[#333333] bg-[#2d2d2d] px-4 text-xs font-semibold tracking-wide text-gray-100">
          Atom Details
        </div>

        <div className="flex-1 overflow-y-auto bg-[#252526] p-4 text-xs leading-5 text-gray-200">
          {selectedAtom ? (
            <>
              <section className="pb-4 border-b border-[#333333]">
                <h2 className="text-[1.35rem] font-semibold leading-tight text-white sm:text-[1.45rem]">
                  {selectedAtom.title}
                </h2>

                <div className="mt-3 flex flex-wrap items-center gap-2">
                  <span
                    className={`rounded-full border px-3 py-1 text-[0.68rem] ${kindBadgeClass(selectedAtom.kind)}`}
                  >
                    {humanize(selectedAtom.kind)}
                  </span>
                  <span
                    className={`rounded-full border px-3 py-1 text-[0.68rem] ${stateBadgeClass(selectedAtom.state)}`}
                  >
                    {humanize(selectedAtom.state)}
                  </span>
                </div>

                <button
                  type="button"
                  onClick={handleOpenPath}
                  disabled={!selectedPath || openFileBusy}
                  className="mt-3 flex w-full items-center gap-2 rounded border border-[#404040] bg-[#1f1f1f] px-3 py-1.5 text-left text-[0.72rem] text-gray-200 hover:bg-[#2a2a2a] disabled:cursor-not-allowed disabled:opacity-55"
                  title={selectedPath || "No path attached to this atom"}
                >
                  <FileCode2 size={16} className="text-gray-400" />
                  <span className="min-w-0 flex-1 truncate font-mono text-[0.72rem]">
                    {selectedPath || "path not set"}
                  </span>
                  <ExternalLink size={14} className="text-gray-400" />
                </button>

                {openFileError ? (
                  <div className="mt-2 text-[10px] text-red-300">
                    {openFileError}
                  </div>
                ) : null}
              </section>

              <section className="pt-4 pb-4 border-b border-[#333333]">
                <h3 className="mb-3 text-[1.05rem] font-semibold leading-none text-white sm:text-[1.1rem]">
                  Rationale
                </h3>

                <div className="space-y-0">
                  <div className="grid grid-cols-[24px_1fr] gap-3 border-b border-white/5 pb-3">
                    <Hash size={16} className="mt-1 text-[#4fc3f7]" />
                    <div>
                      <div className="text-[9px] uppercase tracking-[0.18em] text-gray-500">
                        ID
                      </div>
                      <div className="mt-1 break-all font-mono text-[0.8rem] text-gray-100">
                        {selectedAtom.id}
                      </div>
                    </div>
                  </div>

                  <div className="grid grid-cols-[24px_1fr] gap-3 border-b border-white/5 py-3">
                    <Radar size={16} className="mt-1 text-[#4fc3f7]" />
                    <div>
                      <div className="text-[9px] uppercase tracking-[0.18em] text-gray-500">
                        Scope
                      </div>
                      <div className="mt-1 font-mono text-[0.8rem] text-gray-100">
                        {selectedAtom.scope || "global"}
                      </div>
                    </div>
                  </div>

                  <div className="grid grid-cols-[24px_1fr] gap-3 pt-3">
                    <NotebookText size={16} className="mt-1 text-[#4fc3f7]" />
                    <div>
                      <div className="text-[9px] uppercase tracking-[0.18em] text-gray-500">
                        Narrative
                      </div>
                      <div className="mt-1 whitespace-pre-wrap text-[0.8rem] leading-6 text-gray-100">
                        {(selectedAtom.body || selectedAtom.title).trim()}
                      </div>
                    </div>
                  </div>
                </div>
              </section>

              <section className="pt-4 pb-4 border-b border-[#333333]">
                <div className="mb-2 flex items-center justify-between gap-2">
                  <h3 className="text-[0.95rem] font-semibold text-white">
                    Git Context
                  </h3>
                  <span className="rounded-full px-2 py-0.5 text-[10px] text-gray-300">
                    {gitContextCommits.length} commits
                  </span>
                </div>

                {contextLoading ? (
                  <div className="text-[10px] text-gray-500">
                    Loading git context...
                  </div>
                ) : gitContextCommits.length > 0 ? (
                  <div className="max-h-64 overflow-y-auto pr-1 text-[10px] leading-4">
                    <div className="space-y-0">
                      {gitContextCommits.map((commit) => {
                        const selected =
                          selectedCommitHash === commit.commit_hash;
                        const isFirst =
                          gitContextCommits[0]?.commit_hash ===
                          commit.commit_hash;
                        const isLast =
                          gitContextCommits[gitContextCommits.length - 1]
                            ?.commit_hash === commit.commit_hash;

                        return (
                          <div
                            key={commit.commit_hash}
                            className={`relative pl-6 pb-2 last:pb-0 before:content-[''] after:content-[''] before:absolute before:left-[11px] before:top-0 before:w-px before:bg-[#404040] after:absolute after:left-[11px] after:bottom-0 after:w-px after:bg-[#404040] ${
                              isFirst ? "before:top-2" : ""
                            } ${isLast ? "after:top-2" : "after:top-0"}`}
                          >
                            <span
                              className={`absolute left-[6px] top-2 z-10 h-2.5 w-2.5 rounded-full ${
                                selected ? "bg-[#4fc3f7]" : "bg-[#0e639c]"
                              }`}
                            />
                            <button
                              type="button"
                              onClick={() => {
                                onSelectCommitHash(commit.commit_hash);
                                setDiffDrawerOpen(true);
                              }}
                              title={commitTooltipText(commit)}
                              aria-label={commitTooltipText(commit)}
                              className={`w-full rounded px-2.5 py-1.5 text-left transition-colors ${
                                selected ? "bg-[#2b2b2b]" : "hover:bg-[#2a2a2a]"
                              }`}
                            >
                              <div
                                className="truncate font-medium text-[0.72rem] text-gray-100"
                                title={commit.subject}
                              >
                                {commit.subject}
                              </div>
                              <div className="mt-0.5 flex items-center gap-2 text-[9px] text-gray-500">
                                <span className="font-mono">
                                  {shortHash(commit.commit_hash)}
                                </span>
                                <span>·</span>
                                <span>
                                  {commit.trailer_values.length} lore refs
                                </span>
                              </div>
                              {commit.trailer_values.length > 0 ? (
                                <div className="mt-1 truncate text-[9px] leading-4 text-gray-400">
                                  {stripTrailerPrefix(commit.trailer_values[0])}
                                  {commit.trailer_values.length > 1
                                    ? ` +${commit.trailer_values.length - 1} more`
                                    : ""}
                                </div>
                              ) : null}
                            </button>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                ) : (
                  <div className="text-[10px] text-gray-500">
                    No git context found for this atom path yet.
                  </div>
                )}
              </section>

              <section className="pt-4">
                <div className="mb-2 text-[9px] font-semibold uppercase tracking-[0.18em] text-gray-300">
                  Lifecycle Transition
                </div>
                <select
                  className="mb-2 w-full rounded border border-[#404040] bg-[#1f1f1f] px-2 py-1.5 text-[10px] text-gray-200 outline-none focus:border-[#4fc3f7] disabled:cursor-not-allowed disabled:opacity-60"
                  value={targetState || ""}
                  disabled={transitionOptions.length === 0}
                  onChange={(event) =>
                    onTargetStateChange(
                      event.target.value as AtomStateValue | "",
                    )
                  }
                >
                  <option value="" disabled>
                    choose transition
                  </option>
                  {transitionOptions.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
                {selectedAtom ? (
                  <div className="mb-2 text-[10px] text-gray-500">
                    Current state: {selectedAtom.state.trim().toLowerCase()}.{" "}
                    {transitionOptions.length > 0
                      ? `Available transitions: ${transitionOptions.map((option) => option.label).join(", ")}.`
                      : "No lifecycle transitions are available from this state."}
                  </div>
                ) : null}
                <textarea
                  className="mb-2 w-full rounded border border-[#404040] bg-[#1f1f1f] px-2 py-1.5 text-[10px] text-gray-200 outline-none focus:border-[#4fc3f7]"
                  rows={3}
                  placeholder="Reason for transition"
                  value={stateReason}
                  onChange={(event) => onStateReasonChange(event.target.value)}
                />
                <button
                  className="w-full rounded-lg bg-[#0e639c] px-2 py-1.5 text-[10px] font-semibold text-white hover:bg-[#1177bb] disabled:opacity-60"
                  onClick={onApplyState}
                  disabled={working || loading || !selectedTransitionIsAllowed}
                >
                  Apply State
                </button>
              </section>
            </>
          ) : (
            <div className="mt-10 text-center italic text-[10px] text-slate-500">
              Pick a project folder, then select an atom from the graph
            </div>
          )}
        </div>
      </div>

      <GitDiffDrawer
        open={diffDrawerOpen}
        onClose={() => setDiffDrawerOpen(false)}
        diffLoading={diffLoading}
        diffError={diffError}
        selectedCommitHash={selectedCommitHash}
        selectedCommitDiff={selectedCommitDiff}
      />
    </>
  );
}
