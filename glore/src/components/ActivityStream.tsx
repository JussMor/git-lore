import { useMemo } from "react";

type LogEntry = {
  id: string;
  level: "info" | "success" | "error";
  message: string;
  createdAt: string;
};

type ActivityItem = {
  id: string;
  createdAt: string;
  message: string;
  tone: "info" | "success" | "error";
  badge: string;
};

type Props = {
  logs: LogEntry[];
};

const badgeFromLog = (entry: LogEntry) => {
  const message = entry.message.toLowerCase();

  if (message.startsWith("$ ")) {
    return "Command";
  }
  if (
    message.includes("created new") ||
    message.includes("created signal atom")
  ) {
    return "Created";
  }
  if (message.includes("transitioned atom")) {
    return "State";
  }
  if (message.includes("validation")) {
    return "Validation";
  }
  if (message.includes("workspace") || message.includes("project")) {
    return "Workspace";
  }
  if (entry.level === "success") {
    return "Success";
  }
  if (entry.level === "error") {
    return "Error";
  }
  return "Info";
};

const toneClass = (tone: ActivityItem["tone"]) => {
  switch (tone) {
    case "success":
      return "border-emerald-900 bg-emerald-950/30";
    case "error":
      return "border-red-900 bg-red-950/30";
    default:
      return "border-[#333] bg-[#171717]";
  }
};

const badgeClass = (tone: ActivityItem["tone"]) => {
  switch (tone) {
    case "success":
      return "border-emerald-800 bg-emerald-950/60 text-emerald-200";
    case "error":
      return "border-red-800 bg-red-950/60 text-red-200";
    default:
      return "border-sky-800 bg-sky-950/40 text-sky-200";
  }
};

export function ActivityStream({ logs }: Props) {
  const items = useMemo<ActivityItem[]>(
    () =>
      logs.slice(0, 14).map((entry) => ({
        id: entry.id,
        createdAt: entry.createdAt,
        message: entry.message,
        tone: entry.level,
        badge: badgeFromLog(entry),
      })),
    [logs],
  );

  return (
    <div className="rounded border border-[#3a3a3a] bg-[#1f1f1f] p-3">
      <div className="mb-2 flex items-center justify-between">
        <div className="text-xs font-semibold text-gray-200">
          Recent Activity
        </div>
        <span className="text-[10px] text-gray-500">Real events</span>
      </div>

      {items.length === 0 ? (
        <div className="rounded border border-[#333] bg-[#171717] px-2 py-3 text-xs text-gray-500">
          No operations yet.
        </div>
      ) : (
        <div className="max-h-64 space-y-2 overflow-y-auto pr-1">
          {items.map((item) => (
            <div
              key={item.id}
              className={`rounded border px-2 py-2 ${toneClass(item.tone)}`}
            >
              <div className="mb-1 flex items-center justify-between gap-2">
                <span className="text-[10px] text-gray-400">
                  {item.createdAt}
                </span>
                <span
                  className={`rounded border px-1.5 py-0.5 text-[10px] ${badgeClass(item.tone)}`}
                >
                  {item.badge}
                </span>
              </div>
              <div className="text-xs text-gray-200">{item.message}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
