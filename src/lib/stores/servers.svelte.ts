import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ConsoleEntry,
  ConsoleLine,
  CoreType,
  DownloadProgress,
  NewServerConfig,
  ServerPropertyEntry,
  ServerConfig,
} from "$lib/types";
import { toConsoleEntry } from "$lib/utils/console";

interface ServerStoreState {
  servers: ServerConfig[];
  loading: boolean;
  error: string | null;
  activeConsoleServer: string | null;
  consoleTabs: string[];
  consoleLines: Record<string, ConsoleEntry[]>;
  commandHistoryByServer: Record<string, string[]>;
  autoScrollByServer: Record<string, boolean>;
  creating: boolean;
  createError: string | null;
  download: DownloadProgress | null;
  restartingServers: string[];
  serverCommands: Record<string, string[]>;
}

const MAX_CONSOLE_LINES = 2500;

export const serverState = $state<ServerStoreState>({
  servers: [],
  loading: false,
  error: null,
  activeConsoleServer: null,
  consoleTabs: [],
  consoleLines: {},
  commandHistoryByServer: {},
  autoScrollByServer: {},
  creating: false,
  createError: null,
  download: null,
  restartingServers: [],
  serverCommands: {},
});

let consoleUnlisten: UnlistenFn | null = null;
let downloadUnlisten: UnlistenFn | null = null;
let pollHandle: ReturnType<typeof setInterval> | null = null;

function setServerError(error: unknown): void {
  serverState.error = error instanceof Error ? error.message : String(error);
}

function updateConsoleLines(serverId: string, nextLines: ConsoleEntry[]): void {
  serverState.consoleLines = {
    ...serverState.consoleLines,
    [serverId]: nextLines,
  };
}

function appendConsoleLine(payload: ConsoleLine): void {
  const entry = toConsoleEntry(payload);
  const existing = serverState.consoleLines[payload.server_id] ?? [];
  const last = existing.at(-1);

  if (last && last.raw === entry.raw) {
    const dedupedLast: ConsoleEntry = {
      ...last,
      repeats: last.repeats + 1,
      timestamp: entry.timestamp,
      timestampLabel: entry.timestampLabel,
    };
    updateConsoleLines(payload.server_id, [...existing.slice(0, -1), dedupedLast]);
    return;
  }

  const combined = [...existing, entry];
  const clipped = combined.length > MAX_CONSOLE_LINES ? combined.slice(-MAX_CONSOLE_LINES) : combined;
  updateConsoleLines(payload.server_id, clipped);
}

// Обработка батча строк консоли (оптимизация производительности)
function appendConsoleBatch(payload: { server_id: string; lines: string[]; timestamps: number[] }): void {
  const existing = serverState.consoleLines[payload.server_id] ?? [];
  
  // Преобразуем батч в entries
  const newEntries: ConsoleEntry[] = [];
  for (let i = 0; i < payload.lines.length; i++) {
    const entry = toConsoleEntry({
      server_id: payload.server_id,
      line: payload.lines[i],
      timestamp: payload.timestamps[i],
    });
    newEntries.push(entry);
  }
  
  // Объединяем с существующими и обрезаем
  const combined = [...existing, ...newEntries];
  const clipped = combined.length > MAX_CONSOLE_LINES ? combined.slice(-MAX_CONSOLE_LINES) : combined;
  updateConsoleLines(payload.server_id, clipped);
}

async function ensureEventListeners(): Promise<void> {
  if (!consoleUnlisten) {
    // Слушаем батчи (основной канал, оптимизированный)
    const batchUnlisten = await listen<{ server_id: string; lines: string[]; timestamps: number[] }>(
      "console_batch",
      ({ payload }) => {
        appendConsoleBatch(payload);
      }
    );
    
    // Слушаем одиночные строки (fallback для совместимости)
    const lineUnlisten = await listen<ConsoleLine>("console_line", ({ payload }) => {
      appendConsoleLine(payload);
    });
    
    // Сохраняем оба unlistener'а
    consoleUnlisten = () => {
      batchUnlisten();
      lineUnlisten();
    };
  }

  if (!downloadUnlisten) {
    downloadUnlisten = await listen<DownloadProgress>("download_progress", ({ payload }) => {
      serverState.download = payload;
    });
  }
}

export async function initServers(): Promise<void> {
  await ensureEventListeners();
  await loadServers();
}

export async function loadServers(): Promise<void> {
  serverState.loading = true;
  serverState.error = null;
  try {
    const list = await invoke<ServerConfig[]>("list_servers");
    serverState.servers = list;
  } catch (error) {
    setServerError(error);
  } finally {
    serverState.loading = false;
  }
}

export function startPollingServers(): void {
  if (pollHandle) {
    return;
  }

  pollHandle = setInterval(() => {
    void loadServers();
  }, 5000);
}

export function stopPollingServers(): void {
  if (!pollHandle) {
    return;
  }
  clearInterval(pollHandle);
  pollHandle = null;
}

export async function createServer(config: NewServerConfig): Promise<ServerConfig | null> {
  serverState.creating = true;
  serverState.createError = null;
  try {
    const created = await invoke<ServerConfig>("create_server", { config });
    serverState.servers = [...serverState.servers, created];
    return created;
  } catch (error) {
    serverState.createError = error instanceof Error ? error.message : String(error);
    return null;
  } finally {
    serverState.creating = false;
  }
}

export async function startServer(id: string): Promise<void> {
  serverState.error = null;
  try {
    await invoke("start_server", { id });
    clearServerCommandsCache(id); // Очищаем кэш команд при старте
    await loadServers();
  } catch (error) {
    setServerError(error);
  }
}

export async function stopServer(id: string): Promise<void> {
  serverState.error = null;
  try {
    await invoke("stop_server", { id });
    clearServerCommandsCache(id); // Очищаем кэш команд при остановке
    await loadServers();
  } catch (error) {
    setServerError(error);
  }
}

export async function restartServer(id: string): Promise<void> {
  // Проверяем, не перезапускается ли уже сервер
  if (serverState.restartingServers.includes(id)) {
    return;
  }
  
  serverState.error = null;
  
  // Добавляем сервер в список перезапускающихся
  serverState.restartingServers = [...serverState.restartingServers, id];
  
  try {
    await invoke("stop_server", { id });
    // Ждем немного, чтобы сервер полностью остановился
    await new Promise(resolve => setTimeout(resolve, 2000));
    await invoke("start_server", { id });
    await loadServers();
  } catch (error) {
    setServerError(error);
  } finally {
    // Убираем сервер из списка перезапускающихся
    serverState.restartingServers = serverState.restartingServers.filter(serverId => serverId !== id);
  }
}

export async function deleteServer(id: string): Promise<void> {
  serverState.error = null;
  try {
    await invoke("delete_server", { id });
    const tabs = serverState.consoleTabs.filter((tab) => tab !== id);
    serverState.consoleTabs = tabs;
    if (serverState.activeConsoleServer === id) {
      serverState.activeConsoleServer = tabs[0] ?? null;
    }
    await loadServers();
  } catch (error) {
    setServerError(error);
  }
}

export async function openServerFolder(id: string): Promise<void> {
  serverState.error = null;
  try {
    await invoke("open_server_folder", { id });
  } catch (error) {
    setServerError(error);
  }
}

export async function getServerProperties(id: string): Promise<ServerPropertyEntry[]> {
  try {
    return await invoke<ServerPropertyEntry[]>("get_server_properties", { id });
  } catch (error) {
    setServerError(error);
    return [];
  }
}

export async function saveServerProperties(
  id: string,
  entries: ServerPropertyEntry[],
): Promise<boolean> {
  try {
    await invoke("save_server_properties", { id, entries });
    await loadServers();
    return true;
  } catch (error) {
    setServerError(error);
    return false;
  }
}

export async function sendServerCommand(id: string, command: string): Promise<void> {
  serverState.error = null;
  try {
    await invoke("send_command", { id, command });
    const trimmed = command.trim();
    if (trimmed.length > 0) {
      const existing = serverState.commandHistoryByServer[id] ?? [];
      const deduped = existing.filter((entry) => entry !== trimmed);
      const next = [...deduped, trimmed].slice(-80);
      serverState.commandHistoryByServer = {
        ...serverState.commandHistoryByServer,
        [id]: next,
      };
    }
  } catch (error) {
    setServerError(error);
  }
}

export async function getServerCommands(id: string): Promise<string[]> {
  // Проверяем кэш
  if (serverState.serverCommands[id]) {
    return serverState.serverCommands[id];
  }
  
  try {
    const commands = await invoke<string[]>("get_server_commands", { id });
    // Кэшируем результат
    serverState.serverCommands = {
      ...serverState.serverCommands,
      [id]: commands
    };
    return commands;
  } catch (error) {
    console.error("Failed to get server commands:", error);
    // Возвращаем базовые команды в случае ошибки
    const fallbackCommands = [
      "help",
      "list", 
      "stop",
      "save-all",
      "reload",
      "restart"
    ];
    // Кэшируем fallback команды
    serverState.serverCommands = {
      ...serverState.serverCommands,
      [id]: fallbackCommands
    };
    return fallbackCommands;
  }
}

export function clearServerCommandsCache(id?: string): void {
  if (id) {
    const { [id]: _, ...rest } = serverState.serverCommands;
    serverState.serverCommands = rest;
  } else {
    serverState.serverCommands = {};
  }
}

export async function fetchVersions(core: CoreType): Promise<string[]> {
  try {
    return await invoke<string[]>("fetch_versions", { core });
  } catch (error) {
    setServerError(error);
    return [];
  }
}

export async function attachConsole(id: string): Promise<void> {
  try {
    await invoke("attach_console", { id });
  } catch (error) {
    setServerError(error);
  }
}

export function openConsoleTab(id: string): void {
  if (!serverState.consoleTabs.includes(id)) {
    serverState.consoleTabs = [...serverState.consoleTabs, id];
  }
  serverState.activeConsoleServer = id;

  if (serverState.autoScrollByServer[id] === undefined) {
    serverState.autoScrollByServer = {
      ...serverState.autoScrollByServer,
      [id]: true,
    };
  }
}

export function closeConsoleTab(id: string): void {
  const tabs = serverState.consoleTabs.filter((tab) => tab !== id);
  serverState.consoleTabs = tabs;
  if (serverState.activeConsoleServer === id) {
    serverState.activeConsoleServer = tabs[0] ?? null;
  }
}

export function clearConsole(serverId: string): void {
  updateConsoleLines(serverId, []);
}

export function getCommandHistory(serverId: string): string[] {
  return serverState.commandHistoryByServer[serverId] ?? [];
}

export function setAutoScroll(serverId: string, enabled: boolean): void {
  serverState.autoScrollByServer = {
    ...serverState.autoScrollByServer,
    [serverId]: enabled,
  };
}

export function getServerById(id: string | null): ServerConfig | null {
  if (!id) {
    return null;
  }
  return serverState.servers.find((server) => server.id === id) ?? null;
}

export function isServerRestarting(id: string): boolean {
  return serverState.restartingServers.includes(id);
}

export function clearRestartingState(): void {
  serverState.restartingServers = [];
}

export function shutdownServerStore(): void {
  stopPollingServers();
  if (consoleUnlisten) {
    void consoleUnlisten();
    consoleUnlisten = null;
  }
  if (downloadUnlisten) {
    void downloadUnlisten();
    downloadUnlisten = null;
  }
}
