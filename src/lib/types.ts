export type CoreType =
  | "paper"
  | "purpur"
  | "fabric"
  | "quilt"
  | "forge"
  | "folia"
  | "waterfall"
  | "vanilla";

export interface ServerConfig {
  id: string;
  name: string;
  core: CoreType;
  version: string;
  port: number;
  ram_mb: number;
  jvm_args: string;
  path: string;
  running: boolean;
  online_players?: number;
  max_players?: number;
}

export interface NewServerConfig {
  name: string;
  core: CoreType;
  version: string;
  port: number;
  ram_mb: number;
  jvm_args: string;
  properties: ServerPropertiesConfig;
}

export interface UpdateServerProfileConfig {
  id: string;
  name: string;
  port: number;
  ram_mb: number;
  jvm_args: string;
}

export interface AppSettings {
  java_path: string;
  max_ram_mb: number;
  extra_jvm_flags: string;
  minimize_to_tray: boolean;
  autostart_servers: boolean;
}

export interface ServerPropertiesConfig {
  motd: string;
  gamemode: "survival" | "creative" | "adventure" | "spectator";
  difficulty: "peaceful" | "easy" | "normal" | "hard";
  online_mode: boolean;
  pvp: boolean;
  view_distance: number;
}

export interface ServerPropertyEntry {
  key: string;
  value: string;
}

export interface ConsoleLine {
  server_id: string;
  line: string;
  timestamp: number;
}

export interface DownloadProgress {
  server_id: string;
  filename: string;
  downloaded_bytes: number;
  total_bytes: number;
  percent: number;
  speed_mbps: number;
  done: boolean;
}

export type LogLevel = "INFO" | "SUCCESS" | "WARN" | "ERROR" | "DIM";

export interface ParsedConsoleLine {
  timestamp: string;
  tag: string;
  level: LogLevel;
  message: string;
  raw: string;
}

export interface ConsoleEntry {
  id: string;
  timestamp: number;
  timestampLabel: string;
  tag: string;
  level: LogLevel;
  message: string;
  raw: string;
  repeats: number;
}
