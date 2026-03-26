import type { ConsoleEntry, ConsoleLine, LogLevel, ParsedConsoleLine } from "$lib/types";

// Паттерн для логов с нашим форматом: [HH:MM:SS] [Tag/Level] message
const LINE_PATTERN = /^\[(\d{2}:\d{2}:\d{2})\]\s+\[([^/\]]+)\/([A-Za-z]+)\]\s*(.*)$/;

// Паттерн для стандартных Minecraft логов: [HH:MM:SS INFO]: message
const MINECRAFT_PATTERN = /^\[(\d{2}:\d{2}:\d{2})\s+(INFO|WARN|ERROR|DEBUG|TRACE)\]:\s*(.*)$/;

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
  // Префиксы уже убраны на бэкенде, просто инферим уровень из сообщения
  const line = payload.line.trimEnd();
  const inferredLevel = inferLevel(line);
  const timestamp = formatTimestamp(payload.timestamp);
  
  return {
    timestamp,
    tag: inferredLevel === "ERROR" ? "ERROR" : "Server",
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
