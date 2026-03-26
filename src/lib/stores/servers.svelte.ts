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
  UpdateServerProfileConfig,
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
  serverOrder: string[];
}

const MAX_CONSOLE_LINES = 2500;
const FALLBACK_SERVER_COMMANDS = [
  "help",
  "list",
  "plugins",
  "pl",
  "plugin list",
  "plugin add",
  "plugin install",
  "plugin remove",
  "plugin delete",
  "plugin update",
  "stop",
  "save-all",
  "reload",
  "restart",
];
const FABRIC_EMERGENCY_VERSIONS = [
  "1.21.11",
  "1.21.10",
  "1.21.9",
  "1.21.8",
  "1.21.7",
  "1.21.6",
  "1.21.5",
  "1.21.4",
  "1.21.3",
  "1.21.2",
  "1.21.1",
  "1.21",
  "1.20.6",
  "1.20.5",
  "1.20.4",
  "1.20.3",
  "1.20.2",
  "1.20.1",
  "1.20",
  "1.19.4",
  "1.19.3",
  "1.19.2",
  "1.19.1",
  "1.19",
  "1.18.2",
  "1.18.1",
  "1.18",
  "1.17.1",
  "1.17",
  "1.16.5",
] as const;
const VERSION_DEBUG_PREFIX = "[lodestone:versions]";

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
  serverOrder: [],
});

let consoleUnlisten: UnlistenFn | null = null;
let downloadUnlisten: UnlistenFn | null = null;
let pollHandle: ReturnType<typeof setInterval> | null = null;
const versionsCache: Partial<Record<CoreType, string[]>> = {};

function setServerError(error: unknown): void {
  serverState.error = error instanceof Error ? error.message : String(error);
}

function versionErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return `${error.name}: ${error.message}`;
  }
  return String(error);
}

function dedupeAndSortVersions(versions: string[]): string[] {
  return [...new Set(versions)].sort((a, b) =>
    b.localeCompare(a, undefined, { numeric: true, sensitivity: "base" }),
  );
}

async function fetchJson<T>(url: string): Promise<T> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status} for ${url}`);
  }
  return (await response.json()) as T;
}

async function fetchFabricVersionsDirect(): Promise<string[]> {
  console.debug(`${VERSION_DEBUG_PREFIX} Fabric fallback: try direct Fabric API`);
  try {
    const fabric = await fetchJson<Array<{ version?: string }>>(
      "https://meta.fabricmc.net/v2/versions/game",
    );
    const versions = fabric
      .map((entry) => entry.version?.trim() ?? "")
      .filter((entry) => entry.length > 0);
    if (versions.length > 0) {
      console.debug(
        `${VERSION_DEBUG_PREFIX} Fabric fallback: Fabric API returned ${versions.length} versions`,
      );
      return dedupeAndSortVersions(versions);
    }
    console.warn(`${VERSION_DEBUG_PREFIX} Fabric fallback: Fabric API returned empty list`);
  } catch (error) {
    console.warn(
      `${VERSION_DEBUG_PREFIX} Fabric fallback: Fabric API failed, switching to Mojang manifest`,
      error,
    );
  }

  console.debug(`${VERSION_DEBUG_PREFIX} Fabric fallback: try Mojang manifest`);
  const manifest = await fetchJson<{ versions?: Array<{ id?: string; type?: string }> }>(
    "https://launchermeta.mojang.com/mc/game/version_manifest.json",
  );
  const versions = (manifest.versions ?? [])
    .filter((entry) => entry.type === "release" || entry.type === "snapshot")
    .map((entry) => entry.id?.trim() ?? "")
    .filter((entry) => entry.length > 0);

  console.debug(
    `${VERSION_DEBUG_PREFIX} Fabric fallback: Mojang manifest returned ${versions.length} versions`,
  );
  return dedupeAndSortVersions(versions);
}

async function invokeVersionsWithRetry(core: CoreType, attempts = 6): Promise<string[]> {
  console.debug(`${VERSION_DEBUG_PREFIX} Start fetch_versions for core=${core}, attempts=${attempts}`);
  let lastError: unknown = null;
  for (let attempt = 1; attempt <= attempts; attempt++) {
    console.debug(`${VERSION_DEBUG_PREFIX} IPC attempt ${attempt}/${attempts} for core=${core}`);
    try {
      const versions = await invoke<string[]>("fetch_versions", { core });
      console.debug(
        `${VERSION_DEBUG_PREFIX} IPC success for core=${core} on attempt=${attempt}, versions=${versions.length}`,
      );
      return versions;
    } catch (error) {
      lastError = error;
      console.warn(
        `${VERSION_DEBUG_PREFIX} IPC failed for core=${core} on attempt=${attempt}: ${versionErrorMessage(error)}`,
        error,
      );
      if (attempt < attempts) {
        const delayMs = Math.min(200 * 2 ** (attempt - 1), 2000);
        console.debug(
          `${VERSION_DEBUG_PREFIX} Waiting ${delayMs}ms before next attempt for core=${core}`,
        );
        await new Promise((resolve) => setTimeout(resolve, delayMs));
      }
    }
  }

  if (core === "fabric") {
    console.warn(
      `${VERSION_DEBUG_PREFIX} All IPC attempts exhausted for Fabric. Switching to direct HTTP fallback.`,
    );
    try {
      const versions = await fetchFabricVersionsDirect();
      console.debug(
        `${VERSION_DEBUG_PREFIX} Fabric HTTP fallback success, versions=${versions.length}`,
      );
      return versions;
    } catch (error) {
      console.error(
        `${VERSION_DEBUG_PREFIX} Fabric HTTP fallback failed. Using emergency versions list.`,
        error,
      );
      return [...FABRIC_EMERGENCY_VERSIONS];
    }
  }

  console.error(
    `${VERSION_DEBUG_PREFIX} fetch_versions failed for core=${core} after ${attempts} attempts`,
    lastError,
  );
  throw lastError ?? new Error("Failed to fetch versions");
}

function shouldInvalidateServerCommandsCache(command: string): boolean {
  const normalized = command.trim().replace(/^\/+/, "").toLowerCase();
  if (!normalized) {
    return false;
  }

  return (
    normalized === "reload" ||
    normalized.startsWith("reload ") ||
    normalized.startsWith("plugin ") ||
    normalized === "plugins" ||
    normalized.startsWith("plugins ") ||
    normalized.startsWith("plugman ") ||
    normalized === "pl" ||
    normalized.startsWith("pl ")
  );
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
    
    // Загружаем сохраненный порядок из localStorage
    const savedOrder = localStorage.getItem('server_order');
    if (savedOrder) {
      try {
        const parsed = JSON.parse(savedOrder) as string[];
        const currentIds = new Set(list.map(s => s.id));
        // Фильтруем только существующие серверы
        const validOrder = parsed.filter(id => currentIds.has(id));
        // Добавляем новые серверы в конец
        const existingIds = new Set(validOrder);
        const newIds = list.map(s => s.id).filter(id => !existingIds.has(id));
        serverState.serverOrder = [...validOrder, ...newIds];
      } catch {
        // Если не удалось распарсить, используем порядок по умолчанию
        serverState.serverOrder = list.map(s => s.id);
      }
    } else if (serverState.serverOrder.length === 0) {
      // Инициализируем порядок серверов, если он пустой
      serverState.serverOrder = list.map(s => s.id);
    } else {
      // Добавляем новые серверы в конец
      const existingIds = new Set(serverState.serverOrder);
      const newIds = list.map(s => s.id).filter(id => !existingIds.has(id));
      if (newIds.length > 0) {
        serverState.serverOrder = [...serverState.serverOrder, ...newIds];
      }
      // Удаляем несуществующие серверы
      const currentIds = new Set(list.map(s => s.id));
      serverState.serverOrder = serverState.serverOrder.filter(id => currentIds.has(id));
    }
    
    // Сохраняем порядок в localStorage
    localStorage.setItem('server_order', JSON.stringify(serverState.serverOrder));
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

export async function updateServerProfile(config: UpdateServerProfileConfig): Promise<ServerConfig | null> {
  serverState.error = null;
  try {
    const updated = await invoke<ServerConfig>("update_server_profile", { config });
    serverState.servers = serverState.servers.map((server) =>
      server.id === updated.id ? updated : server
    );
    return updated;
  } catch (error) {
    setServerError(error);
    return null;
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

export async function sendServerCommand(id: string, command: string): Promise<boolean> {
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
    if (shouldInvalidateServerCommandsCache(trimmed)) {
      clearServerCommandsCache(id);
    }
    return true;
  } catch (error) {
    setServerError(error);
    return false;
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
    // Кэшируем fallback команды
    serverState.serverCommands = {
      ...serverState.serverCommands,
      [id]: FALLBACK_SERVER_COMMANDS
    };
    return FALLBACK_SERVER_COMMANDS;
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
  const cached = versionsCache[core];
  if (cached && cached.length > 0) {
    console.debug(`${VERSION_DEBUG_PREFIX} cache hit for core=${core}, versions=${cached.length}`);
    return cached;
  }

  console.debug(`${VERSION_DEBUG_PREFIX} cache miss for core=${core}`);
  try {
    const versions = await invokeVersionsWithRetry(core);
    console.debug(`${VERSION_DEBUG_PREFIX} fetch completed for core=${core}, versions=${versions.length}`);
    if (versions.length > 0) {
      versionsCache[core] = versions;
      console.debug(`${VERSION_DEBUG_PREFIX} cache store for core=${core}, versions=${versions.length}`);
    }
    return versions;
  } catch (error) {
    console.error(`${VERSION_DEBUG_PREFIX} final failure for core=${core}`, error);
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

export function updateServerOrder(newOrder: string[]): void {
  serverState.serverOrder = newOrder;
  // Сохраняем порядок в localStorage
  localStorage.setItem('server_order', JSON.stringify(newOrder));
}

export function getOrderedServers(): ServerConfig[] {
  const serverMap = new Map(serverState.servers.map(s => [s.id, s]));
  return serverState.serverOrder
    .map(id => serverMap.get(id))
    .filter((s): s is ServerConfig => s !== undefined);
}
