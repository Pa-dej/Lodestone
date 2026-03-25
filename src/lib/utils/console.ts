import type { ConsoleEntry, ConsoleLine, LogLevel, ParsedConsoleLine } from "$lib/types";

const LINE_PATTERN = /^\[(\d{2}:\d{2}:\d{2})\]\s+\[([^/\]]+)\/([A-Za-z]+)\]\s*(.*)$/;

function inferLevel(raw: string): LogLevel {
  const upper = raw.toUpperCase();
  if (upper.includes("ERROR") || upper.includes("EXCEPTION") || upper.includes("FAILED")) {
    return "ERROR";
  }
  if (upper.includes("WARN")) {
    return "WARN";
  }
  if (upper.includes("DONE") || upper.includes("STARTED") || upper.includes("SUCCESS")) {
    return "SUCCESS";
  }
  if (upper.includes("DEBUG") || upper.includes("TRACE")) {
    return "DIM";
  }
  return "INFO";
}

export function formatTimestamp(epochSeconds: number): string {
  const date = new Date(epochSeconds * 1000);
  const h = date.getHours().toString().padStart(2, "0");
  const m = date.getMinutes().toString().padStart(2, "0");
  const s = date.getSeconds().toString().padStart(2, "0");
  return `${h}:${m}:${s}`;
}

export function parseConsoleLine(payload: ConsoleLine): ParsedConsoleLine {
  const line = payload.line.trimEnd();
  const match = line.match(LINE_PATTERN);
  if (match) {
    const levelToken = match[3].toUpperCase();
    const level: LogLevel =
      levelToken === "ERROR"
        ? "ERROR"
        : levelToken === "WARN" || levelToken === "WARNING"
          ? "WARN"
          : levelToken === "SUCCESS"
            ? "SUCCESS"
            : levelToken === "DEBUG" || levelToken === "TRACE"
              ? "DIM"
              : "INFO";
    return {
      timestamp: match[1],
      tag: match[2],
      level,
      message: match[4],
      raw: line,
    };
  }

  const inferredLevel = inferLevel(line);
  return {
    timestamp: formatTimestamp(payload.timestamp),
    tag: inferredLevel === "ERROR" ? "ERROR" : "Console",
    level: inferredLevel,
    message: line,
    raw: line,
  };
}

export function toConsoleEntry(payload: ConsoleLine): ConsoleEntry {
  const parsed = parseConsoleLine(payload);
  return {
    id: crypto.randomUUID(),
    timestamp: payload.timestamp,
    timestampLabel: parsed.timestamp,
    tag: parsed.tag,
    level: parsed.level,
    message: parsed.message,
    raw: parsed.raw,
    repeats: 1,
  };
}
