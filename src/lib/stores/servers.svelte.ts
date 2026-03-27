import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ConsoleEntry,
  ConsoleLine,
  CoreType,
  DownloadProgress,
  NewServerConfig,
  RamLimits,
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
  ramLimits: RamLimits;
}

const MAX_CONSOLE_LINES = 2500;
const FALLBACK_SERVER_COMMANDS = ["help", "list", "stop", "save-all", "reload", "restart"];
const SERVER_ORDER_STORAGE_KEY = "lodestone-server-order";

interface ConsoleBatch {
  server_id: string;
  lines: string[];
  timestamps: number[];
}


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
  ramLimits: {
    min_mb: 512,
    max_mb: 16384,
    total_mb: 0,
  },
});

let consoleLineUnlisten: UnlistenFn | null = null;
let consoleBatchUnlisten: UnlistenFn | null = null;
let downloadUnlisten: UnlistenFn | null = null;
let pollHandle: ReturnType<typeof setInterval> | null = null;

function setServerError(error: unknown): void {
  serverState.error = error instanceof Error ? error.message : String(error);
}

function readPersistedServerOrder(): string[] {
  if (typeof window === "undefined") {
    return [];
  }

  try {
    const raw = localStorage.getItem(SERVER_ORDER_STORAGE_KEY);
    if (!raw) {
      return [];
    }
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return [];
    }

    const seen = new Set<string>();
    const ordered: string[] = [];
    for (const value of parsed) {
      if (typeof value !== "string") {
        continue;
      }
      const id = value.trim();
      if (!id || seen.has(id)) {
        continue;
      }
      seen.add(id);
      ordered.push(id);
    }
    return ordered;
  } catch {
    return [];
  }
}

function persistServerOrder(order: string[]): void {
  if (typeof window === "undefined") {
    return;
  }

  try {
    localStorage.setItem(SERVER_ORDER_STORAGE_KEY, JSON.stringify(order));
  } catch {
    // Ignore storage failures (private mode, quota, etc.)
  }
}

function mergeServerOrder(order: string[], servers: ServerConfig[]): string[] {
  const existingIds = new Set(servers.map((server) => server.id));
  const seen = new Set<string>();
  const filtered: string[] = [];

  for (const id of order) {
    if (existingIds.has(id) && !seen.has(id)) {
      seen.add(id);
      filtered.push(id);
    }
  }

  const missing = servers.map((server) => server.id).filter((id) => !seen.has(id));
  return [...filtered, ...missing];
}

function syncServerOrder(servers: ServerConfig[]): void {
  if (serverState.serverOrder.length === 0) {
    const persisted = readPersistedServerOrder();
    serverState.serverOrder = mergeServerOrder(
      persisted.length > 0 ? persisted : servers.map((server) => server.id),
      servers,
    );
    persistServerOrder(serverState.serverOrder);
    return;
  }

  serverState.serverOrder = mergeServerOrder(serverState.serverOrder, servers);
  persistServerOrder(serverState.serverOrder);
}

function uniqueVersions(versions: string[]): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const version of versions) {
    if (seen.has(version)) {
      continue;
    }
    seen.add(version);
    result.push(version);
  }
  return result;
}

async function fetchFabricVersionsDirect(): Promise<string[]> {
  console.debug("[lodestone:fabric] direct fetch start");
  const response = await fetch("https://meta.fabricmc.net/v2/versions/game");
  if (!response.ok) {
    throw new Error(`Fabric API returned HTTP ${response.status}`);
  }

  const payload = (await response.json()) as Array<{ version?: string }>;
  const versions = uniqueVersions(
    payload
      .map((entry) => entry.version?.trim() ?? "")
      .filter((version) => version.length > 0),
  );

  console.debug(`[lodestone:fabric] direct fetch done, versions=${versions.length}`);
  return versions;
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

function appendConsoleBatch(payload: ConsoleBatch): void {
  const lines = Array.isArray(payload.lines) ? payload.lines : [];
  if (lines.length === 0) {
    return;
  }

  const existing = serverState.consoleLines[payload.server_id] ?? [];
  const next = existing.slice();
  const timestamps = Array.isArray(payload.timestamps) ? payload.timestamps : [];
  let hasChanges = false;

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (typeof line !== "string") {
      continue;
    }

    const entry = toConsoleEntry({
      server_id: payload.server_id,
      line,
      timestamp: timestamps[index] ?? Math.floor(Date.now() / 1000),
    });

    const last = next.at(-1);
    if (last && last.raw === entry.raw) {
      next[next.length - 1] = {
        ...last,
        repeats: last.repeats + 1,
        timestamp: entry.timestamp,
        timestampLabel: entry.timestampLabel,
      };
      hasChanges = true;
      continue;
    }

    next.push(entry);
    hasChanges = true;
  }

  if (!hasChanges) {
    return;
  }

  const clipped = next.length > MAX_CONSOLE_LINES ? next.slice(-MAX_CONSOLE_LINES) : next;
  updateConsoleLines(payload.server_id, clipped);
}

async function ensureEventListeners(): Promise<void> {
  if (!consoleLineUnlisten) {
    consoleLineUnlisten = await listen<ConsoleLine>("console_line", ({ payload }) => {
      appendConsoleLine(payload);
    });
  }

  if (!consoleBatchUnlisten) {
    consoleBatchUnlisten = await listen<ConsoleBatch>("console_batch", ({ payload }) => {
      appendConsoleBatch(payload);
    });
  }

  if (!downloadUnlisten) {
    downloadUnlisten = await listen<DownloadProgress>("download_progress", ({ payload }) => {
      serverState.download = payload;
    });
  }
}

export async function initServers(): Promise<void> {
  await ensureEventListeners();
  await loadRamLimits();
  await loadServers();
}

export async function loadRamLimits(): Promise<void> {
  try {
    const limits = await invoke<RamLimits>("get_ram_limits");
    if (
      Number.isFinite(limits.min_mb) &&
      Number.isFinite(limits.max_mb) &&
      limits.min_mb > 0 &&
      limits.max_mb >= limits.min_mb
    ) {
      serverState.ramLimits = limits;
    }
  } catch (error) {
    console.warn("Failed to load RAM limits:", error);
  }
}

export async function loadServers(): Promise<void> {
  serverState.loading = true;
  serverState.error = null;
  try {
    const list = await invoke<ServerConfig[]>("list_servers");
    serverState.servers = list;
    syncServerOrder(list);
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
    syncServerOrder(serverState.servers);
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
    clearServerCommandsCache(id);
    await loadServers();
  } catch (error) {
    setServerError(error);
  }
}

export async function stopServer(id: string): Promise<void> {
  serverState.error = null;
  try {
    await invoke("stop_server", { id });
    clearServerCommandsCache(id);
    await loadServers();
  } catch (error) {
    setServerError(error);
  }
}

export async function restartServer(id: string): Promise<void> {
  if (serverState.restartingServers.includes(id)) {
    return;
  }

  serverState.error = null;

  serverState.restartingServers = [...serverState.restartingServers, id];

  try {
    await invoke("stop_server", { id });
    clearServerCommandsCache(id);
    const waitForRunningState = async (
      expectedRunning: boolean,
      timeoutMs = 20000,
      pollMs = 250,
    ): Promise<boolean> => {
      const deadline = Date.now() + timeoutMs;
      while (Date.now() < deadline) {
        await loadServers();
        const server = serverState.servers.find((item) => item.id === id);
        if (!server) {
          return false;
        }
        if (server.running === expectedRunning) {
          return true;
        }
        await new Promise((resolve) => setTimeout(resolve, pollMs));
      }
      return false;
    };

    const stopped = await waitForRunningState(false);
    if (!stopped) {
      throw new Error("Timed out waiting for server to stop before restart");
    }

    await invoke("start_server", { id });
    await waitForRunningState(true, 12000, 250);
  } catch (error) {
    setServerError(error);
  } finally {
    serverState.restartingServers = serverState.restartingServers.filter(
      (serverId) => serverId !== id,
    );
  }
}

export async function deleteServer(id: string): Promise<void> {
  serverState.error = null;
  try {
    await invoke("delete_server", { id });
    clearServerCommandsCache(id);
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
    return true;
  } catch (error) {
    setServerError(error);
    return false;
  }
}

export async function getServerMotd(id: string): Promise<string> {
  try {
    return await invoke<string>("get_server_motd", { id });
  } catch (error) {
    setServerError(error);
    return "A Lodestone Minecraft Server";
  }
}

export async function updateServerProfile(
  config: UpdateServerProfileConfig,
): Promise<ServerConfig | null> {
  serverState.error = null;
  try {
    const updated = await invoke<ServerConfig>("update_server_profile", { config });
    serverState.servers = serverState.servers.map((server) =>
      server.id === updated.id ? updated : server,
    );
    syncServerOrder(serverState.servers);
    return updated;
  } catch (error) {
    setServerError(error);
    return null;
  }
}

export async function getServerCommands(id: string): Promise<string[]> {
  if (serverState.serverCommands[id]) {
    return serverState.serverCommands[id];
  }

  try {
    const commands = await invoke<string[]>("get_server_commands", { id });
    serverState.serverCommands = {
      ...serverState.serverCommands,
      [id]: commands,
    };
    return commands;
  } catch (error) {
    console.error("Failed to get server commands:", error);
    serverState.serverCommands = {
      ...serverState.serverCommands,
      [id]: FALLBACK_SERVER_COMMANDS,
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
  try {
    if (core === "fabric") {
      return await fetchFabricVersionsDirect();
    }
    return await invoke<string[]>("fetch_versions", { core });
  } catch (error) {
    if (core === "fabric") {
      console.error("[lodestone:fabric] direct fetch failed", error);
    }
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

export function updateServerOrder(newOrder: string[]): void {
  serverState.serverOrder = mergeServerOrder(newOrder, serverState.servers);
  persistServerOrder(serverState.serverOrder);
}

export function getOrderedServers(): ServerConfig[] {
  const serverMap = new Map(serverState.servers.map((server) => [server.id, server]));
  const ordered = serverState.serverOrder
    .map((id) => serverMap.get(id))
    .filter((server): server is ServerConfig => server !== undefined);

  if (ordered.length === serverState.servers.length) {
    return ordered;
  }

  const orderedIds = new Set(ordered.map((server) => server.id));
  const missing = serverState.servers.filter((server) => !orderedIds.has(server.id));
  return [...ordered, ...missing];
}

export function shutdownServerStore(): void {
  stopPollingServers();
  if (consoleLineUnlisten) {
    void consoleLineUnlisten();
    consoleLineUnlisten = null;
  }
  if (consoleBatchUnlisten) {
    void consoleBatchUnlisten();
    consoleBatchUnlisten = null;
  }
  if (downloadUnlisten) {
    void downloadUnlisten();
    downloadUnlisten = null;
  }
}
