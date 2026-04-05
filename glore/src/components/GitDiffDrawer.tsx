import { useMemo } from "react";

type DiffLineKind = "meta" | "hunk" | "addition" | "deletion" | "context";

type ParsedDiffLine = {
  id: string;
  kind: DiffLineKind;
  leftLine: number | null;
  rightLine: number | null;
  content: string;
};

type CommitDiffReport = {
  commit_hash: string;
  subject: string;
  diff: string;
  truncated: boolean;
};

type Props = {
  open: boolean;
  onClose: () => void;
  diffLoading: boolean;
  diffError: string;
  selectedCommitHash: string | null;
  selectedCommitDiff: CommitDiffReport | null;
};

const shortHash = (value: string) => value.slice(0, 7);

const parseHunkHeader = (line: string) => {
  const match = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
  if (!match) {
    return null;
  }

  return {
    leftStart: Number(match[1]),
    rightStart: Number(match[2]),
  };
};

const isDiffMetaLine = (line: string) =>
  line.startsWith("diff --git") ||
  line.startsWith("index ") ||
  line.startsWith("--- ") ||
  line.startsWith("+++ ") ||
  line.startsWith("new file mode") ||
  line.startsWith("deleted file mode") ||
  line.startsWith("similarity index") ||
  line.startsWith("rename from ") ||
  line.startsWith("rename to ") ||
  line.startsWith("old mode ") ||
  line.startsWith("new mode ");

const parseUnifiedDiff = (diff: string): ParsedDiffLine[] => {
  if (!diff.trim()) {
    return [];
  }

  const lines = diff.replace(/\r\n/g, "\n").split("\n");
  const parsed: ParsedDiffLine[] = [];

  let leftLine = 0;
  let rightLine = 0;
  let hasHunk = false;

  lines.forEach((line, index) => {
    if (line.startsWith("@@")) {
      const header = parseHunkHeader(line);
      if (header) {
        leftLine = header.leftStart;
        rightLine = header.rightStart;
        hasHunk = true;
      }

      parsed.push({
        id: `hunk-${index}`,
        kind: "hunk",
        leftLine: null,
        rightLine: null,
        content: line,
      });
      return;
    }

    if (
      isDiffMetaLine(line) ||
      line.startsWith("\\ No newline at end of file")
    ) {
      parsed.push({
        id: `meta-${index}`,
        kind: "meta",
        leftLine: null,
        rightLine: null,
        content: line,
      });
      return;
    }

    if (!hasHunk) {
      parsed.push({
        id: `meta-pre-${index}`,
        kind: "meta",
        leftLine: null,
        rightLine: null,
        content: line,
      });
      return;
    }

    if (line.startsWith("+") && !line.startsWith("+++")) {
      parsed.push({
        id: `add-${index}`,
        kind: "addition",
        leftLine: null,
        rightLine,
        content: line,
      });
      rightLine += 1;
      return;
    }

    if (line.startsWith("-") && !line.startsWith("---")) {
      parsed.push({
        id: `del-${index}`,
        kind: "deletion",
        leftLine,
        rightLine: null,
        content: line,
      });
      leftLine += 1;
      return;
    }

    parsed.push({
      id: `ctx-${index}`,
      kind: "context",
      leftLine,
      rightLine,
      content: line,
    });
    leftLine += 1;
    rightLine += 1;
  });

  return parsed;
};

const diffRowClass = (kind: DiffLineKind) => {
  switch (kind) {
    case "addition":
      return "bg-emerald-950/30";
    case "deletion":
      return "bg-rose-950/30";
    case "hunk":
      return "bg-sky-950/25";
    case "meta":
      return "bg-[#151719]";
    default:
      return "bg-[#111315]";
  }
};

const diffCodeClass = (kind: DiffLineKind) => {
  switch (kind) {
    case "addition":
      return "text-emerald-200";
    case "deletion":
      return "text-rose-200";
    case "hunk":
      return "text-sky-200";
    case "meta":
      return "text-gray-400";
    default:
      return "text-gray-200";
  }
};

const lineSymbol = (kind: DiffLineKind) => {
  switch (kind) {
    case "addition":
      return "+";
    case "deletion":
      return "-";
    case "context":
      return " ";
    case "hunk":
      return "@@";
    default:
      return "";
  }
};

const lineContent = (line: ParsedDiffLine) => {
  if (line.kind === "addition" || line.kind === "deletion" || line.kind === "context") {
    return line.content.slice(1);
  }
  return line.content;
};

export function GitDiffDrawer({
  open,
  onClose,
  diffLoading,
  diffError,
  selectedCommitHash,
  selectedCommitDiff,
}: Props) {
  const parsedCommitDiff = useMemo(
    () => parseUnifiedDiff(selectedCommitDiff?.diff ?? ""),
    [selectedCommitDiff?.diff],
  );

  const stats = useMemo(() => {
    let additions = 0;
    let deletions = 0;
    let hunks = 0;
    let files = 0;

    for (const line of parsedCommitDiff) {
      if (line.kind === "addition") additions += 1;
      if (line.kind === "deletion") deletions += 1;
      if (line.kind === "hunk") hunks += 1;
      if (line.kind === "meta" && line.content.startsWith("diff --git")) {
        files += 1;
      }
    }

    return { additions, deletions, hunks, files };
  }, [parsedCommitDiff]);

  return (
    <div className={`absolute inset-0 z-40 ${open ? "" : "pointer-events-none"}`}>
      <button
        type="button"
        aria-label="Close diff drawer backdrop"
        onClick={onClose}
        className={`absolute inset-0 bg-black/60 transition-opacity duration-300 ${
          open ? "opacity-100" : "opacity-0"
        }`}
      />

      <aside
        className={`absolute right-0 top-0 h-full w-[min(84rem,96vw)] border-l border-[#2b3138] bg-[#0f1318] shadow-2xl transition-transform duration-300 ${
          open ? "translate-x-0" : "translate-x-full"
        }`}
        aria-label="Commit diff drawer"
      >
        <div className="flex h-full flex-col">
          <div className="border-b border-[#2b3138] bg-[#121820] px-4 py-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <div className="text-sm font-semibold text-gray-100">Diff Drawer</div>
                <div className="text-xs text-gray-400">
                  {selectedCommitDiff
                    ? `${selectedCommitDiff.subject || shortHash(selectedCommitDiff.commit_hash)} · ${shortHash(selectedCommitDiff.commit_hash)}`
                    : selectedCommitHash
                      ? `Commit ${shortHash(selectedCommitHash)}`
                      : "No commit selected"}
                </div>
              </div>
              <button
                type="button"
                onClick={onClose}
                className="rounded border border-[#3a3f46] px-2 py-1 text-xs text-gray-200 hover:bg-[#1b2530]"
              >
                Close
              </button>
            </div>

            {selectedCommitDiff ? (
              <div className="mt-2 flex flex-wrap gap-1.5 text-[10px]">
                <span className="rounded-full border border-emerald-700/70 bg-emerald-950/35 px-2 py-0.5 text-emerald-200">
                  +{stats.additions}
                </span>
                <span className="rounded-full border border-rose-700/70 bg-rose-950/35 px-2 py-0.5 text-rose-200">
                  -{stats.deletions}
                </span>
                <span className="rounded-full border border-sky-700/70 bg-sky-950/35 px-2 py-0.5 text-sky-200">
                  {stats.hunks} hunks
                </span>
                <span className="rounded-full border border-gray-700/70 bg-gray-900/60 px-2 py-0.5 text-gray-300">
                  {stats.files} files
                </span>
              </div>
            ) : null}
          </div>

          {diffLoading ? (
            <div className="p-4 text-sm text-gray-400">Loading diff...</div>
          ) : diffError ? (
            <div className="p-4 text-sm text-rose-300">{diffError}</div>
          ) : !selectedCommitDiff ? (
            <div className="p-4 text-sm text-gray-500">
              Select a commit in Git Context to render the patch.
            </div>
          ) : parsedCommitDiff.length === 0 ? (
            <div className="p-4 text-sm text-gray-500">
              No diff lines available for this commit.
            </div>
          ) : (
            <div className="flex-1 overflow-auto">
              <div className="min-w-195">
                <div className="sticky top-0 z-10 grid grid-cols-[64px_64px_36px_1fr] border-b border-[#2b3138] bg-[#161c24] text-[11px] text-gray-400">
                  <div className="border-r border-[#2b3138] px-2 py-1.5 text-right">Old</div>
                  <div className="border-r border-[#2b3138] px-2 py-1.5 text-right">New</div>
                  <div className="border-r border-[#2b3138] px-2 py-1.5 text-center">Δ</div>
                  <div className="px-3 py-1.5">Code</div>
                </div>

                {parsedCommitDiff.map((line) => (
                  <div
                    key={line.id}
                    className={`grid grid-cols-[64px_64px_36px_1fr] border-b border-[#1f242c] ${diffRowClass(line.kind)}`}
                  >
                    <div className="select-none border-r border-[#2b3138] px-2 py-1 text-right font-mono text-[11px] text-gray-500 tabular-nums">
                      {line.leftLine ?? ""}
                    </div>
                    <div className="select-none border-r border-[#2b3138] px-2 py-1 text-right font-mono text-[11px] text-gray-500 tabular-nums">
                      {line.rightLine ?? ""}
                    </div>
                    <div className={`select-none border-r border-[#2b3138] px-2 py-1 text-center font-mono text-[11px] ${diffCodeClass(line.kind)}`}>
                      {lineSymbol(line.kind)}
                    </div>
                    <pre
                      className={`overflow-x-auto px-3 py-1.5 font-mono text-[12px] leading-5 whitespace-pre ${diffCodeClass(line.kind)}`}
                    >
                      {lineContent(line) || " "}
                    </pre>
                  </div>
                ))}

                {selectedCommitDiff.truncated ? (
                  <div className="border-t border-[#2b3138] bg-[#1c1a12] px-3 py-2 text-xs text-amber-300">
                    Diff truncated for preview.
                  </div>
                ) : null}
              </div>
            </div>
          )}
        </div>
      </aside>
    </div>
  );
}
