use futures_util::StreamExt;
use parking_lot::RwLock as SyncRwLock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    env,
    ffi::OsStr,
    fs,
    io::{BufWriter, ErrorKind, Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command as StdCommand, Stdio},
    sync::{Arc, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tokio::{
    fs as tokio_fs,
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::{mpsc, Mutex as AsyncMutex},
};
use uuid::Uuid;
use zip::ZipArchive;

#[cfg(unix)]
mod pty;

const CONSOLE_EVENT: &str = "console_line";
const CONSOLE_BATCH_EVENT: &str = "console_batch";
const DOWNLOAD_PROGRESS_EVENT: &str = "download_progress";
const VERSION_CACHE_TTL_SECS: u64 = 300;
const MAX_CONSOLE_LINES: usize = 50;
const CONSOLE_CHANNEL_SIZE: usize = 64;
const BATCH_SIZE: usize = 8;
const BATCH_INTERVAL_MS: u64 = 8;
const BASE_SERVER_COMMANDS: &[&str] = &[
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
const EXTENDED_SERVER_COMMANDS: &[&str] = &[
    "say",
    "save-on",
    "save-off",
    "time set day",
    "time set night",
    "time add",
    "time query",
    "weather clear",
    "weather rain",
    "weather thunder",
    "gamerule keepInventory true",
    "gamerule keepInventory false",
    "gamerule doMobSpawning true",
    "gamerule doMobSpawning false",
    "difficulty peaceful",
    "difficulty easy",
    "difficulty normal",
    "difficulty hard",
    "gamemode survival",
    "gamemode creative",
    "gamemode adventure",
    "gamemode spectator",
    "tp",
    "teleport",
    "whitelist on",
    "whitelist off",
    "whitelist add",
    "whitelist remove",
    "whitelist list",
    "whitelist reload",
    "ban",
    "ban-ip",
    "pardon",
    "pardon-ip",
    "kick",
    "op",
    "deop",
    "reload confirm",
    "seed",
    "give",
    "clear",
    "kill",
    "effect",
    "enchant",
    "experience",
    "xp",
    "fill",
    "setblock",
    "summon",
    "tellraw",
    "title",
    "scoreboard",
    "team",
    "worldborder",
    "spawnpoint",
    "setworldspawn",
    "plugman list",
    "plugman load",
    "plugman unload",
    "plugman reload",
    "plugman enable",
    "plugman disable",
    "plugman check",
];

type VersionsCache = HashMap<String, (Instant, Vec<String>)>;

static VERSIONS_CACHE: OnceLock<SyncRwLock<VersionsCache>> = OnceLock::new();

#[derive(Debug, Clone)]
enum ConsoleEvent {
    Line {
        server_id: Arc<str>,
        line: Arc<str>,
        timestamp: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    id: String,
    name: String,
    core: String,
    version: String,
    port: u16,
    ram_mb: u32,
    #[serde(default)]
    jvm_args: String,
    path: PathBuf,
    running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    online_players: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_players: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewServerConfig {
    name: String,
    core: String,
    version: String,
    port: u16,
    ram_mb: u32,
    #[serde(default)]
    jvm_args: String,
    #[serde(default)]
    properties: ServerPropertiesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateServerProfileConfig {
    id: String,
    name: String,
    port: u16,
    ram_mb: u32,
    #[serde(default)]
    jvm_args: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerPropertiesConfig {
    motd: String,
    gamemode: String,
    difficulty: String,
    online_mode: bool,
    pvp: bool,
    view_distance: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerPropertyEntry {
    key: String,
    value: String,
}

impl Default for ServerPropertiesConfig {
    fn default() -> Self {
        Self {
            motd: String::from("A Lodestone Minecraft Server"),
            gamemode: String::from("survival"),
            difficulty: String::from("normal"),
            online_mode: true,
            pvp: true,
            view_distance: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    java_path: String,
    max_ram_mb: u32,
    extra_jvm_flags: String,
    minimize_to_tray: bool,
    autostart_servers: bool,
    #[serde(default)]
    kill_server_processes_on_exit: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            java_path: String::from("java"),
            max_ram_mb: 4096,
            extra_jvm_flags: String::new(),
            minimize_to_tray: false,
            autostart_servers: false,
            kill_server_processes_on_exit: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsoleLinePayload {
    server_id: String,
    line: String,
    timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
struct ConsoleBatchPayload {
    server_id: String,
    lines: Vec<String>,
    timestamps: Vec<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct DownloadProgressPayload {
    server_id: String,
    filename: String,
    downloaded_bytes: u64,
    total_bytes: u64,
    percent: f64,
    speed_mbps: f64,
    done: bool,
}

#[derive(Clone)]
struct RunningServer {
    #[cfg(unix)]
    pty_master: Arc<crate::pty::PtyMaster>,
    #[cfg(not(unix))]
    stdin: Arc<AsyncMutex<ChildStdin>>,
    #[allow(dead_code)]
    console_tx: mpsc::Sender<ConsoleEvent>,
    recent_lines: Arc<SyncRwLock<VecDeque<Arc<str>>>>,
}

#[derive(Clone, Default)]
struct AppState {
    running: Arc<AsyncMutex<HashMap<String, RunningServer>>>,
    servers_cache: Arc<SyncRwLock<Option<Vec<ServerConfig>>>>,
    tracked_server_pids: Arc<SyncRwLock<HashMap<String, u32>>>,
}

fn now_timestamp_secs() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}

fn home_dir() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("HOME") {
        return Ok(PathBuf::from(path));
    }

    if let Some(path) = env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(path));
    }

    Err(String::from("Cannot resolve user home directory"))
}

fn app_data_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(path) = env::var_os("APPDATA") {
            return Ok(PathBuf::from(path).join("Lodestone"));
        }
        if let Some(path) = env::var_os("LOCALAPPDATA") {
            return Ok(PathBuf::from(path).join("Lodestone"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(path) = env::var_os("XDG_DATA_HOME") {
            return Ok(PathBuf::from(path).join("lodestone"));
        }
    }

    Ok(home_dir()?.join(".lodestone"))
}

fn servers_root_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("servers"))
}

fn servers_file_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("servers.json"))
}

fn settings_file_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("settings.json"))
}

async fn ensure_app_dirs() -> Result<(), String> {
    let app_dir = app_data_dir()?;
    let legacy_dir = home_dir()?.join(".lodestone");

    if app_dir != legacy_dir && !app_dir.exists() && legacy_dir.exists() {
        let _ = tokio_fs::rename(&legacy_dir, &app_dir).await;
    }

    let servers_dir = servers_root_dir()?;
    tokio_fs::create_dir_all(&app_dir)
        .await
        .map_err(|err| format!("Failed to create app data directory: {err}"))?;
    tokio_fs::create_dir_all(&servers_dir)
        .await
        .map_err(|err| format!("Failed to create servers directory: {err}"))?;
    Ok(())
}

async fn load_servers_from_disk() -> Result<Vec<ServerConfig>, String> {
    let file_path = servers_file_path()?;
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let bytes = tokio_fs::read(&file_path)
        .await
        .map_err(|err| format!("Failed to read servers.json: {err}"))?;

    serde_json::from_slice::<Vec<ServerConfig>>(&bytes)
        .map_err(|err| format!("Failed to parse servers.json: {err}"))
}

async fn load_servers_cached(state: &AppState) -> Result<Vec<ServerConfig>, String> {
    {
        let cache = state.servers_cache.read();
        if let Some(ref servers) = *cache {
            return Ok(servers.clone());
        }
    }

    let servers = load_servers_from_disk().await?;
    {
        let mut cache = state.servers_cache.write();
        *cache = Some(servers.clone());
    }

    Ok(servers)
}

fn invalidate_servers_cache(state: &AppState) {
    let mut cache = state.servers_cache.write();
    *cache = None;
}

async fn save_servers_to_disk(
    servers: &[ServerConfig],
    state: Option<&AppState>,
) -> Result<(), String> {
    ensure_app_dirs().await?;
    let file_path = servers_file_path()?;
    let body = serde_json::to_vec_pretty(servers)
        .map_err(|err| format!("Failed to serialize server list: {err}"))?;
    tokio_fs::write(&file_path, body)
        .await
        .map_err(|err| format!("Failed to write servers.json: {err}"))?;

    if let Some(state) = state {
        invalidate_servers_cache(state);
    }

    Ok(())
}

async fn set_server_running_flag(
    state: &AppState,
    server_id: &str,
    running: bool,
) -> Result<(), String> {
    let mut servers = load_servers_cached(state).await?;
    for server in &mut servers {
        if server.id == server_id {
            server.running = running;
        }
    }
    save_servers_to_disk(&servers, Some(state)).await
}

async fn load_settings_from_disk() -> Result<AppSettings, String> {
    let file_path = settings_file_path()?;
    if !file_path.exists() {
        return Ok(AppSettings::default());
    }

    let bytes = tokio_fs::read(&file_path)
        .await
        .map_err(|err| format!("Failed to read settings.json: {err}"))?;

    serde_json::from_slice::<AppSettings>(&bytes)
        .map_err(|err| format!("Failed to parse settings.json: {err}"))
}

async fn save_settings_to_disk(settings: &AppSettings) -> Result<(), String> {
    ensure_app_dirs().await?;
    let file_path = settings_file_path()?;
    let body = serde_json::to_vec_pretty(settings)
        .map_err(|err| format!("Failed to serialize settings: {err}"))?;
    tokio_fs::write(&file_path, body)
        .await
        .map_err(|err| format!("Failed to write settings.json: {err}"))
}

fn load_settings_from_disk_sync() -> AppSettings {
    let file_path = match settings_file_path() {
        Ok(path) => path,
        Err(_) => return AppSettings::default(),
    };

    if !file_path.exists() {
        return AppSettings::default();
    }

    let bytes = match fs::read(&file_path) {
        Ok(content) => content,
        Err(_) => return AppSettings::default(),
    };

    serde_json::from_slice::<AppSettings>(&bytes).unwrap_or_default()
}

fn should_minimize_to_tray_on_close() -> bool {
    load_settings_from_disk_sync().minimize_to_tray
}

fn should_kill_server_processes_on_exit() -> bool {
    load_settings_from_disk_sync().kill_server_processes_on_exit
}

fn remember_server_pid(state: &AppState, server_id: &str, pid: u32) {
    if pid == 0 || pid == std::process::id() {
        return;
    }

    let mut tracked = state.tracked_server_pids.write();
    tracked.insert(server_id.to_string(), pid);
}

fn forget_server_pid(state: &AppState, server_id: &str) {
    let mut tracked = state.tracked_server_pids.write();
    tracked.remove(server_id);
}

fn tracked_server_pids_snapshot(state: &AppState) -> Vec<(String, u32)> {
    let tracked = state.tracked_server_pids.read();
    tracked
        .iter()
        .map(|(server_id, pid)| (server_id.clone(), *pid))
        .collect()
}

#[cfg(target_os = "windows")]
fn terminate_process_by_pid(pid: u32) -> Result<(), String> {
    if pid == 0 || pid == std::process::id() {
        return Ok(());
    }

    let pid_value = pid.to_string();
    let status = StdCommand::new("taskkill")
        .args(["/PID", pid_value.as_str(), "/T", "/F"])
        .status()
        .map_err(|err| format!("Failed to execute taskkill for PID {pid}: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "taskkill failed for PID {pid} with status {status}"
        ))
    }
}

#[cfg(unix)]
fn terminate_process_by_pid(pid: u32) -> Result<(), String> {
    if pid == 0 || pid == std::process::id() {
        return Ok(());
    }

    let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    if result == 0 {
        return Ok(());
    }

    let err = std::io::Error::last_os_error();
    if err.kind() == ErrorKind::NotFound {
        Ok(())
    } else {
        Err(format!("Failed to terminate PID {pid}: {err}"))
    }
}

#[cfg(not(any(unix, target_os = "windows")))]
fn terminate_process_by_pid(pid: u32) -> Result<(), String> {
    let _ = pid;
    Err(String::from(
        "Killing processes by PID is not supported on this platform",
    ))
}

fn kill_tracked_server_processes_if_needed(state: &AppState) {
    if !should_kill_server_processes_on_exit() {
        return;
    }

    let tracked = tracked_server_pids_snapshot(state);
    for (server_id, pid) in tracked {
        if let Err(err) = terminate_process_by_pid(pid) {
            eprintln!(
                "[lodestone] failed to terminate tracked server process {server_id} (pid {pid}): {err}"
            );
        }
    }

    state.tracked_server_pids.write().clear();
}

fn java_exec(settings: &AppSettings) -> String {
    let trimmed = settings.java_path.trim();
    if trimmed.is_empty() {
        String::from("java")
    } else {
        String::from(trimmed)
    }
}

fn split_jvm_flags(raw: &str) -> Vec<&str> {
    raw.split_whitespace().collect()
}

fn server_name_or_default(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        String::from("server")
    } else {
        String::from(trimmed)
    }
}

fn emit_console_line(app_handle: &AppHandle, server_id: &str, line: &str) {
    let payload = ConsoleLinePayload {
        server_id: server_id.to_string(),
        line: line.to_string(),
        timestamp: now_timestamp_secs(),
    };
    let _ = app_handle.emit(CONSOLE_EVENT, payload);
}

fn emit_download_progress(app_handle: &AppHandle, payload: DownloadProgressPayload) {
    let _ = app_handle.emit(DOWNLOAD_PROGRESS_EVENT, payload);
}

fn normalize_core(core: &str) -> String {
    core.trim().to_ascii_lowercase()
}

fn split_natural_chunks(value: &str) -> Vec<Cow<'_, str>> {
    let mut chunks: Vec<Cow<'_, str>> = Vec::new();
    let mut current = String::new();
    let mut current_is_digit: Option<bool> = None;

    for ch in value.trim().chars() {
        if !ch.is_ascii_alphanumeric() {
            if !current.is_empty() {
                chunks.push(Cow::Owned(current));
                current = String::new();
                current_is_digit = None;
            }
            continue;
        }

        let is_digit = ch.is_ascii_digit();
        if let Some(last_kind) = current_is_digit {
            if last_kind != is_digit {
                chunks.push(Cow::Owned(current));
                current = String::new();
            }
        }

        current.push(ch.to_ascii_lowercase());
        current_is_digit = Some(is_digit);
    }

    if !current.is_empty() {
        chunks.push(Cow::Owned(current));
    }

    chunks
}

fn compare_numeric_chunks(left: &str, right: &str) -> Ordering {
    let left_normalized = left.trim_start_matches('0');
    let right_normalized = right.trim_start_matches('0');
    let left_value = if left_normalized.is_empty() {
        "0"
    } else {
        left_normalized
    };
    let right_value = if right_normalized.is_empty() {
        "0"
    } else {
        right_normalized
    };

    left_value
        .len()
        .cmp(&right_value.len())
        .then_with(|| left_value.cmp(right_value))
}

fn is_prerelease_chunk(chunk: &str) -> bool {
    let value = chunk.to_ascii_lowercase();
    value.starts_with("pre")
        || value.starts_with("rc")
        || value.starts_with("snapshot")
        || value.starts_with("beta")
        || value.starts_with("alpha")
}

fn compare_minecraft_versions(left: &str, right: &str) -> Ordering {
    let left_chunks = split_natural_chunks(left);
    let right_chunks = split_natural_chunks(right);
    let max_len = left_chunks.len().max(right_chunks.len());

    for index in 0..max_len {
        match (left_chunks.get(index), right_chunks.get(index)) {
            (Some(left_chunk), Some(right_chunk)) => {
                let left_is_num = left_chunk.chars().all(|ch| ch.is_ascii_digit());
                let right_is_num = right_chunk.chars().all(|ch| ch.is_ascii_digit());

                let order = match (left_is_num, right_is_num) {
                    (true, true) => {
                        compare_numeric_chunks(left_chunk.as_ref(), right_chunk.as_ref())
                    }
                    (false, false) => left_chunk.cmp(right_chunk),
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                };

                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(left_chunk), None) => {
                return if is_prerelease_chunk(left_chunk.as_ref()) {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            (None, Some(right_chunk)) => {
                return if is_prerelease_chunk(right_chunk.as_ref()) {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
            }
            (None, None) => break,
        }
    }

    left.trim().cmp(right.trim())
}

fn sort_versions_desc(versions: &mut Vec<String>) {
    versions.sort_by(|left, right| compare_minecraft_versions(right, left));
    versions.dedup();
}

fn versions_cache() -> &'static SyncRwLock<HashMap<String, (Instant, Vec<String>)>> {
    VERSIONS_CACHE.get_or_init(|| SyncRwLock::new(HashMap::new()))
}

fn get_cached_versions(core: &str) -> Option<Vec<String>> {
    let cache = versions_cache().read();
    let (cached_at, versions) = cache.get(core)?;
    if cached_at.elapsed().as_secs() <= VERSION_CACHE_TTL_SECS {
        Some(versions.clone())
    } else {
        None
    }
}

fn put_cached_versions(core: &str, versions: &[String]) {
    let mut cache = versions_cache().write();
    cache.insert(core.to_string(), (Instant::now(), versions.to_vec()));
}

fn forge_supports_installer(mc_version: &str) -> bool {
    compare_minecraft_versions(mc_version, "1.11") != Ordering::Less
}

fn extract_port_from_endpoint(endpoint: &str) -> Option<u16> {
    endpoint
        .rsplit_once(':')
        .and_then(|(_, port)| port.parse::<u16>().ok())
}

#[cfg(target_os = "windows")]
fn find_windows_port_owner_pid(port: u16) -> Option<u32> {
    let output = StdCommand::new("netstat")
        .args(["-ano", "-p", "tcp"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let columns: Vec<&str> = line.split_whitespace().collect();
        if columns.len() < 4 {
            continue;
        }
        if !columns[0].eq_ignore_ascii_case("TCP") {
            continue;
        }
        if extract_port_from_endpoint(columns[1]) != Some(port) {
            continue;
        }
        if let Some(pid_col) = columns.last() {
            if let Ok(pid) = pid_col.parse::<u32>() {
                if pid != std::process::id() {
                    return Some(pid);
                }
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn find_windows_process_name(pid: u32) -> Option<String> {
    let filter = format!("PID eq {pid}");
    let output = StdCommand::new("tasklist")
        .args(["/FI", &filter, "/FO", "CSV", "/NH"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('"') {
            continue;
        }

        let columns: Vec<&str> = trimmed.trim_matches('"').split("\",\"").collect();
        if columns.len() < 2 {
            continue;
        }

        if columns[1].trim() == pid.to_string() {
            return Some(columns[0].trim().to_string());
        }
    }

    None
}

fn format_port_conflict_error(port: u16) -> String {
    #[cfg(target_os = "windows")]
    {
        if let Some(pid) = find_windows_port_owner_pid(port) {
            if let Some(process_name) = find_windows_process_name(pid) {
                return format!("Порт {port} уже занят процессом {process_name} (PID {pid}).");
            }

            return format!("Порт {port} уже занят процессом PID {pid}.");
        }
    }

    format!("Порт {port} уже используется.")
}

fn ensure_server_port_available(port: u16) -> Result<(), String> {
    match TcpListener::bind(("0.0.0.0", port)) {
        Ok(listener) => {
            drop(listener);
            Ok(())
        }
        Err(err) if err.kind() == ErrorKind::AddrInUse => Err(format_port_conflict_error(port)),
        Err(err) => Err(format!("Failed to check server port {port}: {err}")),
    }
}

async fn download_to_path(
    client: &Client,
    url: &str,
    destination: &Path,
    app_handle: &AppHandle,
    server_id: &str,
) -> Result<(), String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("Failed to start download from {url}: {err}"))?
        .error_for_status()
        .map_err(|err| format!("Download request failed for {url}: {err}"))?;

    let total_bytes = response.content_length().unwrap_or(0);
    let file_name = destination
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("server.jar");

    let mut stream = response.bytes_stream();
    let file = tokio_fs::File::create(destination)
        .await
        .map_err(|err| format!("Failed to create file {}: {err}", destination.display()))?;

    let std_file = file.into_std().await;
    let mut buffered_file = BufWriter::with_capacity(64 * 1024, std_file);

    let mut downloaded_bytes: u64 = 0;
    let started_at = Instant::now();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|err| format!("Download stream failure: {err}"))?;
        buffered_file
            .write_all(&chunk)
            .map_err(|err| format!("Failed writing {}: {err}", destination.display()))?;
        downloaded_bytes += chunk.len() as u64;

        let elapsed_secs = started_at.elapsed().as_secs_f64().max(0.001);
        let speed_mbps = downloaded_bytes as f64 / (1024.0 * 1024.0) / elapsed_secs;
        let percent = if total_bytes > 0 {
            ((downloaded_bytes as f64 / total_bytes as f64) * 100.0).min(100.0)
        } else {
            0.0
        };

        emit_download_progress(
            app_handle,
            DownloadProgressPayload {
                server_id: server_id.to_string(),
                filename: file_name.to_string(),
                downloaded_bytes,
                total_bytes,
                percent,
                speed_mbps,
                done: false,
            },
        );
    }

    buffered_file
        .flush()
        .map_err(|err| format!("Failed to flush {}: {err}", destination.display()))?;

    Ok(())
}

async fn fetch_json(client: &Client, url: &str) -> Result<Value, String> {
    client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("Request failed for {url}: {err}"))?
        .error_for_status()
        .map_err(|err| format!("Request failed for {url}: {err}"))?
        .json::<Value>()
        .await
        .map_err(|err| format!("Invalid JSON returned by {url}: {err}"))
}

fn extract_last_numeric(values: &[Value], label: &str) -> Result<u32, String> {
    values
        .iter()
        .filter_map(Value::as_u64)
        .max()
        .map(|value| value as u32)
        .ok_or_else(|| format!("No {label} build available"))
}

async fn latest_paper_like_build(
    client: &Client,
    project: &str,
    version: &str,
) -> Result<u32, String> {
    let url = format!("https://api.papermc.io/v2/projects/{project}/versions/{version}");
    let json = fetch_json(client, &url).await?;
    let builds = json
        .get("builds")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("Unexpected build response from {project} API"))?;

    extract_last_numeric(builds, project)
}

async fn vanilla_server_jar_url(client: &Client, version: &str) -> Result<String, String> {
    let manifest = fetch_json(
        client,
        "https://launchermeta.mojang.com/mc/game/version_manifest.json",
    )
    .await?;

    let versions = manifest
        .get("versions")
        .and_then(Value::as_array)
        .ok_or_else(|| String::from("Invalid vanilla version manifest response"))?;

    let details_url = versions
        .iter()
        .find(|entry| entry.get("id").and_then(Value::as_str) == Some(version))
        .and_then(|entry| entry.get("url"))
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| format!("Vanilla version {version} was not found"))?;

    let details = fetch_json(client, &details_url).await?;
    details
        .get("downloads")
        .and_then(|downloads| downloads.get("server"))
        .and_then(|server| server.get("url"))
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| format!("No server.jar download is available for vanilla {version}"))
}

async fn latest_fabric_installer_version(client: &Client) -> Result<String, String> {
    let installers = fetch_json(client, "https://meta.fabricmc.net/v2/versions/installer").await?;
    installers
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("version"))
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| String::from("Failed to resolve Fabric installer version"))
}

async fn url_is_available(client: &Client, url: &str) -> Result<bool, String> {
    let response = client
        .head(url)
        .send()
        .await
        .map_err(|err| format!("Failed to check download URL {url}: {err}"))?;
    Ok(response.status().is_success())
}

async fn forge_installer_url(client: &Client, mc_version: &str) -> Result<String, String> {
    if !forge_supports_installer(mc_version) {
        return Err(format!(
            "Forge for Minecraft {mc_version} is not supported by this launcher. Choose Minecraft 1.11+."
        ));
    }

    let promos = fetch_json(
        client,
        "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json",
    )
    .await?;

    let promos_map = promos
        .get("promos")
        .and_then(Value::as_object)
        .ok_or_else(|| String::from("Invalid Forge promotions response"))?;

    let latest_key = format!("{mc_version}-latest");
    let recommended_key = format!("{mc_version}-recommended");

    let mut seen = HashSet::<String>::new();
    let mut candidates = Vec::<String>::new();
    for key in [&latest_key, &recommended_key] {
        if let Some(version) = promos_map.get(key).and_then(Value::as_str) {
            let normalized = version.trim();
            if !normalized.is_empty() && seen.insert(normalized.to_string()) {
                candidates.push(normalized.to_string());
            }
        }
    }

    if candidates.is_empty() {
        return Err(format!(
            "No Forge build metadata found for Minecraft {mc_version}"
        ));
    }

    for forge_version in candidates {
        let installer_url = format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{mc_version}-{forge_version}/forge-{mc_version}-{forge_version}-installer.jar"
        );
        if url_is_available(client, &installer_url).await? {
            return Ok(installer_url);
        }
    }

    Err(format!(
        "No downloadable Forge installer was found for Minecraft {mc_version}. Try a newer Forge version."
    ))
}

async fn run_java_command(
    settings: &AppSettings,
    cwd: &Path,
    args: &[String],
) -> Result<(), String> {
    let mut command = Command::new(java_exec(settings));
    command.current_dir(cwd).args(args);
    let output = command
        .output()
        .await
        .map_err(|err| format!("Failed to execute Java: {err}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err(format!("Java command exited with status {}", output.status))
    } else {
        Err(format!("Java command failed: {stderr}"))
    }
}

fn choose_generated_server_jar(server_dir: &Path, core: &str) -> Result<PathBuf, String> {
    let entries = fs::read_dir(server_dir)
        .map_err(|err| format!("Failed to inspect {}: {err}", server_dir.display()))?;

    let mut best: Option<(i32, u64, PathBuf)> = None;

    for entry_result in entries {
        let entry =
            entry_result.map_err(|err| format!("Failed to read server directory: {err}"))?;
        let path = entry.path();
        let extension = path
            .extension()
            .and_then(OsStr::to_str)
            .map(|ext| ext.eq_ignore_ascii_case("jar"))
            .unwrap_or(false);
        if !extension {
            continue;
        }

        let name = path
            .file_name()
            .and_then(OsStr::to_str)
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        if name.contains("installer") {
            continue;
        }

        let mut score: i32 = 0;
        if name == "server.jar" {
            score += 400;
        }
        if name.contains("server") {
            score += 50;
        }
        if core == "fabric" && name.contains("fabric-server-launch") {
            score += 300;
        }
        if core == "fabric" && name.contains("launch") {
            score += 100;
        }
        if core == "quilt" && name.contains("quilt-server-launch") {
            score += 300;
        }
        if core == "quilt" && name.contains("quilt") {
            score += 200;
        }
        if core == "quilt" && name.contains("launch") {
            score += 100;
        }
        if core == "forge" && name.contains("shim") {
            score += 300;
        }
        if core == "forge" && name.contains("forge") {
            score += 200;
        }
        if core == "forge" && name.contains("universal") {
            score += 100;
        }

        let size = entry
            .metadata()
            .map_err(|err| format!("Failed to read metadata for {}: {err}", path.display()))?
            .len();

        match &best {
            Some((best_score, best_size, _)) => {
                if score > *best_score || (score == *best_score && size > *best_size) {
                    best = Some((score, size, path));
                }
            }
            None => {
                best = Some((score, size, path));
            }
        }
    }

    best.map(|(_, _, path)| path).ok_or_else(|| {
        format!(
            "Could not resolve a generated server JAR inside {}",
            server_dir.display()
        )
    })
}

fn normalized_enum(value: &str, allowed: &[&str], fallback: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    if allowed.contains(&normalized.as_str()) {
        normalized
    } else {
        String::from(fallback)
    }
}

fn sanitized_property_value(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim().to_string()
}

fn sanitized_property_key(key: &str) -> String {
    key.replace(['\r', '\n', '='], "").trim().to_string()
}

fn sanitized_jvm_args(args: &str) -> String {
    args.replace(['\r', '\n'], " ").trim().to_string()
}

fn sanitize_directory_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn generate_server_directory_name(name: &str, core: &str, version: &str) -> String {
    let safe_name = sanitize_directory_name(name);
    let safe_core = sanitize_directory_name(core);
    let safe_version = sanitize_directory_name(version);

    format!("{}-{}-{}", safe_name, safe_core, safe_version)
}

fn parse_server_properties(content: &str) -> Vec<ServerPropertyEntry> {
    let mut entries = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let normalized_key = sanitized_property_key(key);
            if normalized_key.is_empty() {
                continue;
            }

            entries.push(ServerPropertyEntry {
                key: normalized_key,
                value: value.trim().to_string(),
            });
        }
    }

    entries
}

fn stringify_server_properties(entries: &[ServerPropertyEntry]) -> String {
    let mut body = String::new();
    for entry in entries {
        body.push_str(&entry.key);
        body.push('=');
        body.push_str(&entry.value);
        body.push('\n');
    }
    body
}

async fn upsert_server_property(server_path: &Path, key: &str, value: &str) -> Result<(), String> {
    let normalized_key = sanitized_property_key(key);
    if normalized_key.is_empty() {
        return Ok(());
    }

    let normalized_value = sanitized_property_value(value);
    let properties_path = server_path.join("server.properties");
    let mut entries = if properties_path.exists() {
        let content = tokio_fs::read_to_string(&properties_path)
            .await
            .map_err(|err| {
                format!(
                    "Failed to read server properties {}: {err}",
                    properties_path.display()
                )
            })?;
        parse_server_properties(&content)
    } else {
        Vec::new()
    };

    if let Some(existing) = entries
        .iter_mut()
        .find(|entry| entry.key.eq_ignore_ascii_case(&normalized_key))
    {
        existing.key = normalized_key.clone();
        existing.value = normalized_value;
    } else {
        entries.push(ServerPropertyEntry {
            key: normalized_key,
            value: normalized_value,
        });
    }

    entries.sort_by(|a, b| a.key.cmp(&b.key));
    let body = stringify_server_properties(&entries);
    tokio_fs::write(&properties_path, body)
        .await
        .map_err(|err| {
            format!(
                "Failed to write server properties {}: {err}",
                properties_path.display()
            )
        })?;

    Ok(())
}

async fn place_generated_server_jar(
    generated_jar: &Path,
    server_jar_path: &Path,
    core_name: &str,
) -> Result<(), String> {
    if generated_jar == server_jar_path {
        return Ok(());
    }

    if let (Ok(source_real), Ok(target_real)) = (
        fs::canonicalize(generated_jar),
        fs::canonicalize(server_jar_path),
    ) {
        if source_real == target_real {
            return Ok(());
        }
    }

    if server_jar_path.exists() {
        tokio_fs::remove_file(server_jar_path)
            .await
            .map_err(|err| {
                format!(
                    "Failed to replace existing server.jar at {}: {err}",
                    server_jar_path.display()
                )
            })?;
    }

    match tokio_fs::rename(generated_jar, server_jar_path).await {
        Ok(_) => Ok(()),
        Err(_) => tokio_fs::copy(generated_jar, server_jar_path)
            .await
            .map(|_| ())
            .map_err(|err| {
                format!(
                    "Failed to copy generated {core_name} JAR {} to {}: {err}",
                    generated_jar.display(),
                    server_jar_path.display()
                )
            }),
    }
}

fn resolve_launch_jar_name(server_dir: &Path, core: &str) -> String {
    if core == "quilt" {
        let quilt_launcher = "quilt-server-launch.jar";
        if server_dir.join(quilt_launcher).exists() {
            return String::from(quilt_launcher);
        }
    }
    String::from("server.jar")
}

async fn write_start_scripts(
    server_dir: &Path,
    settings: &AppSettings,
    ram_mb: u32,
    core: &str,
    server_jvm_args: &str,
) -> Result<(), String> {
    let java = java_exec(settings);
    let global_args = settings.extra_jvm_flags.trim();
    let server_args = sanitized_jvm_args(server_jvm_args);

    let mut merged_args = String::new();
    if !global_args.is_empty() {
        merged_args.push_str(global_args);
    }
    if !server_args.is_empty() {
        if !merged_args.is_empty() {
            merged_args.push(' ');
        }
        merged_args.push_str(&server_args);
    }

    let extra_segment = if merged_args.is_empty() {
        String::new()
    } else {
        format!("{merged_args} ")
    };
    let launch_jar = resolve_launch_jar_name(server_dir, core);

    #[cfg(target_os = "windows")]
    {
        let bat_body = format!(
            "@echo off\r\n\"{java}\" -Xms{ram_mb}M -Xmx{ram_mb}M {extra_segment}-jar {launch_jar} nogui\r\npause\r\n"
        );
        let bat_path = server_dir.join("start.bat");
        tokio_fs::write(&bat_path, bat_body)
            .await
            .map_err(|err| format!("Failed to write {}: {err}", bat_path.display()))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let sh_body = format!(
            "#!/usr/bin/env sh\n\"{java}\" -Xms{ram_mb}M -Xmx{ram_mb}M {extra_segment}-jar {launch_jar} nogui\n"
        );
        let sh_path = server_dir.join("start.sh");
        tokio_fs::write(&sh_path, sh_body)
            .await
            .map_err(|err| format!("Failed to write {}: {err}", sh_path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&sh_path, perms).map_err(|err| {
                format!(
                    "Failed to set executable permissions on {}: {err}",
                    sh_path.display()
                )
            })?;
        }
    }

    Ok(())
}

async fn write_bootstrap_files(
    server_dir: &Path,
    settings: &AppSettings,
    ram_mb: u32,
    core: &str,
    port: u16,
    properties: &ServerPropertiesConfig,
    jvm_args: &str,
) -> Result<(), String> {
    let eula_path = server_dir.join("eula.txt");
    tokio_fs::write(&eula_path, "eula=true\n")
        .await
        .map_err(|err| format!("Failed to write {}: {err}", eula_path.display()))?;

    let props_path = server_dir.join("server.properties");
    let gamemode = normalized_enum(
        &properties.gamemode,
        &["survival", "creative", "adventure", "spectator"],
        "survival",
    );
    let difficulty = normalized_enum(
        &properties.difficulty,
        &["peaceful", "easy", "normal", "hard"],
        "normal",
    );
    let view_distance = properties.view_distance.clamp(3, 32);
    let motd = sanitized_property_value(&properties.motd);

    let props_body = format!(
        "server-port={port}\n\
         motd={motd}\n\
         gamemode={gamemode}\n\
         difficulty={difficulty}\n\
         online-mode={}\n\
         pvp={}\n\
         view-distance={view_distance}\n",
        properties.online_mode, properties.pvp
    );
    tokio_fs::write(&props_path, props_body)
        .await
        .map_err(|err| format!("Failed to write {}: {err}", props_path.display()))?;

    write_start_scripts(server_dir, settings, ram_mb, core, jvm_args).await?;

    Ok(())
}

async fn install_core_jar(
    client: &Client,
    app_handle: &AppHandle,
    server_id: &str,
    core: &str,
    version: &str,
    server_dir: &Path,
    settings: &AppSettings,
) -> Result<(), String> {
    let server_jar_path = server_dir.join("server.jar");

    match core {
        "paper" => {
            let build = latest_paper_like_build(client, "paper", version).await?;
            let url = format!(
                "https://api.papermc.io/v2/projects/paper/versions/{version}/builds/{build}/downloads/paper-{version}-{build}.jar"
            );
            download_to_path(client, &url, &server_jar_path, app_handle, server_id).await?;
        }
        "purpur" => {
            let url = format!("https://api.purpurmc.org/v2/purpur/{version}/latest/download");
            download_to_path(client, &url, &server_jar_path, app_handle, server_id).await?;
        }
        "folia" => {
            let build = latest_paper_like_build(client, "folia", version).await?;
            let url = format!(
                "https://api.papermc.io/v2/projects/folia/versions/{version}/builds/{build}/downloads/folia-{version}-{build}.jar"
            );
            download_to_path(client, &url, &server_jar_path, app_handle, server_id).await?;
        }
        "waterfall" => {
            let build = latest_paper_like_build(client, "waterfall", version).await?;
            let url = format!(
                "https://api.papermc.io/v2/projects/waterfall/versions/{version}/builds/{build}/downloads/waterfall-{version}-{build}.jar"
            );
            download_to_path(client, &url, &server_jar_path, app_handle, server_id).await?;
        }
        "vanilla" => {
            let url = vanilla_server_jar_url(client, version).await?;
            download_to_path(client, &url, &server_jar_path, app_handle, server_id).await?;
        }
        "fabric" => {
            let installer_version = latest_fabric_installer_version(client).await?;
            let installer_url = format!(
                "https://maven.fabricmc.net/net/fabricmc/fabric-installer/{installer_version}/fabric-installer-{installer_version}.jar"
            );
            let installer_path = server_dir.join("fabric-installer.jar");
            download_to_path(
                client,
                &installer_url,
                &installer_path,
                app_handle,
                server_id,
            )
            .await?;

            emit_download_progress(
                app_handle,
                DownloadProgressPayload {
                    server_id: server_id.to_string(),
                    filename: String::from("fabric-installer"),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    percent: 95.0,
                    speed_mbps: 0.0,
                    done: false,
                },
            );

            let installer_file_name = installer_path
                .file_name()
                .and_then(OsStr::to_str)
                .map(String::from)
                .ok_or_else(|| String::from("Fabric installer path is invalid"))?;

            run_java_command(
                settings,
                server_dir,
                &[
                    String::from("-jar"),
                    installer_file_name,
                    String::from("server"),
                    String::from("-mcversion"),
                    String::from(version),
                    String::from("-downloadMinecraft"),
                ],
            )
            .await?;

            let generated_jar = choose_generated_server_jar(server_dir, "fabric")?;
            place_generated_server_jar(&generated_jar, &server_jar_path, "Fabric").await?;
        }
        "quilt" => {
            let installer_url =
                String::from("https://quiltmc.org/api/v1/download-latest-installer/java-universal");
            let installer_path = server_dir.join("quilt-installer.jar");
            download_to_path(
                client,
                &installer_url,
                &installer_path,
                app_handle,
                server_id,
            )
            .await?;

            emit_download_progress(
                app_handle,
                DownloadProgressPayload {
                    server_id: server_id.to_string(),
                    filename: String::from("quilt-installer"),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    percent: 95.0,
                    speed_mbps: 0.0,
                    done: false,
                },
            );

            let installer_file_name = installer_path
                .file_name()
                .and_then(OsStr::to_str)
                .map(String::from)
                .ok_or_else(|| String::from("Quilt installer path is invalid"))?;

            run_java_command(
                settings,
                server_dir,
                &[
                    String::from("-jar"),
                    installer_file_name,
                    String::from("install"),
                    String::from("server"),
                    String::from(version),
                    String::from("--download-server"),
                    String::from("--create-scripts"),
                    format!("--install-dir={}", server_dir.display()),
                ],
            )
            .await?;

            let launcher_path = server_dir.join("quilt-server-launch.jar");
            if !launcher_path.exists() {
                return Err(format!(
                    "Quilt installation did not create {}",
                    launcher_path.display()
                ));
            }
        }
        "forge" => {
            let installer_url = forge_installer_url(client, version).await?;
            let installer_path = server_dir.join("forge-installer.jar");
            download_to_path(
                client,
                &installer_url,
                &installer_path,
                app_handle,
                server_id,
            )
            .await?;

            emit_download_progress(
                app_handle,
                DownloadProgressPayload {
                    server_id: server_id.to_string(),
                    filename: String::from("forge-installer"),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    percent: 95.0,
                    speed_mbps: 0.0,
                    done: false,
                },
            );

            let installer_file_name = installer_path
                .file_name()
                .and_then(OsStr::to_str)
                .map(String::from)
                .ok_or_else(|| String::from("Forge installer path is invalid"))?;

            run_java_command(
                settings,
                server_dir,
                &[
                    String::from("-jar"),
                    installer_file_name,
                    String::from("--installServer"),
                ],
            )
            .await?;

            let generated_jar = choose_generated_server_jar(server_dir, "forge")?;
            place_generated_server_jar(&generated_jar, &server_jar_path, "Forge").await?;
        }
        _ => {
            return Err(format!("Unsupported core: {core}"));
        }
    }

    Ok(())
}

fn spawn_console_reader<R>(server_id: Arc<str>, reader: R, console_tx: mpsc::Sender<ConsoleEvent>)
where
    R: AsyncRead + Send + Unpin + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(raw_line)) = lines.next_line().await {
            let cleaned = strip_log_prefix(&raw_line);
            let line: Arc<str> = cleaned.into();

            let timestamp = now_timestamp_secs();

            let event = ConsoleEvent::Line {
                server_id: server_id.clone(),
                line,
                timestamp,
            };

            if console_tx.send(event).await.is_err() {
                break;
            }
        }
    });
}
fn strip_log_prefix(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut pos = 0;

    while pos < bytes.len() && bytes[pos] == 0x1b {
        pos += 1;
        if pos < bytes.len() && bytes[pos] == b'[' {
            pos += 1;
            while pos < bytes.len() && bytes[pos] != b'm' {
                pos += 1;
            }
            if pos < bytes.len() {
                pos += 1;
            }
        }
    }

    if bytes.get(pos) == Some(&b'[') {
        pos += 1;
        while pos < bytes.len() && bytes[pos] != b']' {
            pos += 1;
        }
        if pos < bytes.len() {
            pos += 1;
        }
    }

    loop {
        if pos >= bytes.len() {
            break;
        }
        if bytes[pos] == b' ' {
            pos += 1;
        } else if bytes[pos] == 0x1b && pos + 1 < bytes.len() && bytes[pos + 1] == b'[' {
            pos += 2;
            while pos < bytes.len() && bytes[pos] != b'm' {
                pos += 1;
            }
            if pos < bytes.len() {
                pos += 1;
            }
        } else {
            break;
        }
    }

    if bytes.get(pos) == Some(&b'[') {
        pos += 1;
        while pos < bytes.len() && bytes[pos] != b']' {
            pos += 1;
        }
        if pos < bytes.len() {
            pos += 1;
        }
    }

    while pos < bytes.len() && (bytes[pos] == b':' || bytes[pos] == b'>' || bytes[pos] == b' ') {
        pos += 1;
    }

    while pos < bytes.len() && bytes[pos] == 0x1b {
        pos += 1;
        if pos < bytes.len() && bytes[pos] == b'[' {
            pos += 1;
            while pos < bytes.len() && bytes[pos] != b'm' {
                pos += 1;
            }
            if pos < bytes.len() {
                pos += 1;
            }
        }
    }

    if pos < line.len() {
        &line[pos..]
    } else {
        line
    }
}

#[cfg(unix)]
fn spawn_pty_reader(
    server_id: Arc<str>,
    master: Arc<crate::pty::PtyMaster>,
    tx: mpsc::Sender<ConsoleEvent>,
) {
    tokio::spawn(async move {
        let mut raw_buf = [0u8; 4096];

        let mut line_buf: Vec<u8> = Vec::with_capacity(256);

        loop {
            let n = match master.read(&mut raw_buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };

            let chunk = &raw_buf[..n];

            for &byte in chunk {
                if byte == b'\n' {
                    if line_buf.last() == Some(&b'\r') {
                        line_buf.pop();
                    }

                    let raw_line = String::from_utf8_lossy(&line_buf);
                    let cleaned = strip_log_prefix(&raw_line);
                    let line: Arc<str> = cleaned.into();

                    let event = ConsoleEvent::Line {
                        server_id: server_id.clone(),
                        line,
                        timestamp: now_timestamp_secs(),
                    };

                    if tx.send(event).await.is_err() {
                        return;
                    }

                    line_buf.clear();
                } else {
                    line_buf.push(byte);
                }
            }
        }
    });
}

fn spawn_console_processor(
    app_handle: AppHandle,
    server_id: String,
    mut console_rx: mpsc::Receiver<ConsoleEvent>,
    recent_lines: Arc<SyncRwLock<VecDeque<Arc<str>>>>,
) {
    tokio::spawn(async move {
        let mut buffer: Vec<(Arc<str>, u64)> = Vec::with_capacity(BATCH_SIZE);

        let mut interval = tokio::time::interval(Duration::from_millis(BATCH_INTERVAL_MS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {

                Some(event) = console_rx.recv() => {
                    match event {
                        ConsoleEvent::Line { server_id: event_server_id, line, timestamp } => {

                            if event_server_id.as_ref() != server_id {
                                continue;
                            }

                            buffer.push((line, timestamp));


                            if buffer.len() >= BATCH_SIZE {
                                flush_console_batch(&app_handle, &server_id, &mut buffer, &recent_lines);
                            }
                        }
                    }
                }


                _ = interval.tick() => {
                    if !buffer.is_empty() {
                        flush_console_batch(&app_handle, &server_id, &mut buffer, &recent_lines);
                    }
                }


                else => break,
            }
        }

        if !buffer.is_empty() {
            flush_console_batch(&app_handle, &server_id, &mut buffer, &recent_lines);
        }
    });
}

fn flush_console_batch(
    app_handle: &AppHandle,
    server_id: &str,
    buffer: &mut Vec<(Arc<str>, u64)>,
    recent_lines: &Arc<SyncRwLock<VecDeque<Arc<str>>>>,
) {
    if buffer.is_empty() {
        return;
    }

    {
        let mut cache = recent_lines.write();
        for (line, _) in buffer.iter() {
            if cache.len() >= MAX_CONSOLE_LINES {
                cache.pop_front();
            }
            cache.push_back(line.clone());
        }
    }

    let lines: Vec<String> = buffer.iter().map(|(line, _)| line.to_string()).collect();
    let timestamps: Vec<u64> = buffer.iter().map(|(_, ts)| *ts).collect();

    let payload = ConsoleBatchPayload {
        server_id: server_id.to_string(),
        lines,
        timestamps,
    };

    let _ = app_handle.emit(CONSOLE_BATCH_EVENT, payload);

    buffer.clear();
}

fn spawn_process_watcher(
    app_handle: AppHandle,
    state: AppState,
    server_id: String,
    child_handle: Arc<AsyncMutex<Child>>,
) {
    tokio::spawn(async move {
        let status_result = child_handle.lock().await.wait().await;

        {
            let mut running = state.running.lock().await;
            running.remove(&server_id);
        }
        forget_server_pid(&state, &server_id);
        let _ = set_server_running_flag(&state, &server_id, false).await;

        match status_result {
            Ok(status) => {
                if status.success() {
                    emit_console_line(
                        &app_handle,
                        &server_id,
                        &format!("[SYSTEM/SUCCESS] Server stopped ({status})"),
                    );
                } else {
                    emit_console_line(
                        &app_handle,
                        &server_id,
                        &format!("[SYSTEM/ERROR] Server exited unexpectedly ({status})"),
                    );
                }
            }
            Err(err) => {
                emit_console_line(
                    &app_handle,
                    &server_id,
                    &format!("[SYSTEM/ERROR] Failed to wait for process: {err}"),
                );
            }
        }
    });
}

fn running_ids(map: &HashMap<String, RunningServer>) -> HashSet<String> {
    map.keys().map(String::from).collect()
}

#[tauri::command]
async fn list_servers(state: State<'_, AppState>) -> Result<Vec<ServerConfig>, String> {
    let mut servers: Vec<ServerConfig> = load_servers_cached(&state).await.unwrap_or_default();

    let running_ids = {
        let running = state.running.lock().await;
        running_ids(&running)
    };

    for server in &mut servers {
        server.running = running_ids.contains(&server.id);

        if server.running {
            if let Ok((online_players, max_players)) =
                get_server_stats_internal(&state, &server.id).await
            {
                server.online_players = online_players;
                server.max_players = max_players;
            }
        } else {
            server.online_players = None;
            server.max_players = None;
        }
    }
    Ok(servers)
}

async fn get_server_stats_internal(
    state: &State<'_, AppState>,
    id: &str,
) -> Result<(Option<u32>, Option<u32>), String> {
    let running = {
        let running_map = state.running.lock().await;
        running_map.get(id).cloned()
    };

    let Some(server) = running else {
        return Ok((None, None));
    };

    let max_players = get_max_players_from_properties(state.inner(), id)
        .await
        .unwrap_or(20);

    match get_online_players_count(&server, id) {
        Ok(online_count) => Ok((Some(online_count), Some(max_players))),
        Err(_) => Ok((None, None)),
    }
}

fn get_online_players_count(server: &RunningServer, _server_id: &str) -> Result<u32, String> {
    let recent_lines = server.recent_lines.read();

    let start_idx = recent_lines.len().saturating_sub(20);
    for line in recent_lines.iter().skip(start_idx).rev() {
        if let Some(count) = parse_player_count_from_line(line) {
            return Ok(count);
        }
    }

    Ok(0)
}

fn parse_player_count_from_line(line: &str) -> Option<u32> {
    if line.contains("players online") && (line.contains("there are") || line.contains("There are"))
    {
        let mut found_are = false;
        for word in line.split_whitespace() {
            if found_are {
                if let Ok(count) = word.parse::<u32>() {
                    return Some(count);
                }
            }
            if word.eq_ignore_ascii_case("are") {
                found_are = true;
            }
        }
    }

    if line.contains("online players") || line.contains("Online players") {
        if let Some(start) = line.find('(') {
            if let Some(end) = line.find(')') {
                if start < end {
                    let count_str = &line[start + 1..end];
                    if let Ok(count) = count_str.parse::<u32>() {
                        return Some(count);
                    }
                }
            }
        }
    }

    None
}

async fn get_max_players_from_properties(state: &AppState, server_id: &str) -> Result<u32, String> {
    let servers = load_servers_cached(state).await?;
    let server = servers
        .iter()
        .find(|s| s.id == server_id)
        .ok_or("Server not found")?;

    let properties_path = server.path.join("server.properties");

    if !properties_path.exists() {
        return Ok(20);
    }

    let content = tokio_fs::read_to_string(&properties_path)
        .await
        .map_err(|e| format!("Failed to read server.properties: {}", e))?;

    for line in content.lines() {
        if line.starts_with("max-players=") {
            if let Some(value_str) = line.split('=').nth(1) {
                if let Ok(value) = value_str.trim().parse::<u32>() {
                    return Ok(value);
                }
            }
        }
    }

    Ok(20)
}

#[tauri::command]
async fn create_server(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    config: NewServerConfig,
) -> Result<ServerConfig, String> {
    ensure_app_dirs().await?;

    let core = normalize_core(&config.core);
    let supported = [
        "paper",
        "purpur",
        "fabric",
        "quilt",
        "forge",
        "folia",
        "waterfall",
        "vanilla",
    ];
    if !supported.contains(&core.as_str()) {
        return Err(format!("Unsupported core: {}", config.core));
    }

    let version = config.version.trim();
    if version.is_empty() {
        return Err(String::from("Server version is required"));
    }

    let id = Uuid::new_v4().to_string();
    let server_name = server_name_or_default(&config.name);
    let dir_name = generate_server_directory_name(&server_name, &core, version);
    let server_dir = servers_root_dir()?.join(&dir_name);

    let server_dir = if server_dir.exists() {
        let mut counter = 1;
        loop {
            let dir_with_suffix = servers_root_dir()?.join(format!("{}-{}", dir_name, counter));
            if !dir_with_suffix.exists() {
                break dir_with_suffix;
            }
            counter += 1;
            if counter > 100 {
                return Err(String::from("Failed to find available directory name"));
            }
        }
    } else {
        server_dir
    };

    tokio_fs::create_dir_all(&server_dir).await.map_err(|err| {
        format!(
            "Failed to create server directory {}: {err}",
            server_dir.display()
        )
    })?;

    let settings = load_settings_from_disk().await.unwrap_or_default();
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|err| format!("Failed to initialize HTTP client: {err}"))?;

    install_core_jar(
        &client,
        &app_handle,
        &id,
        &core,
        version,
        &server_dir,
        &settings,
    )
    .await?;

    write_bootstrap_files(
        &server_dir,
        &settings,
        config.ram_mb,
        &core,
        config.port,
        &config.properties,
        &config.jvm_args,
    )
    .await?;

    let server = ServerConfig {
        id: String::from(&id),
        name: server_name,
        core,
        version: String::from(version),
        port: config.port,
        ram_mb: config.ram_mb,
        jvm_args: sanitized_jvm_args(&config.jvm_args),
        path: server_dir,
        running: false,
        online_players: None,
        max_players: None,
    };

    let mut servers = load_servers_cached(&state).await?;
    servers.push(server.clone());
    save_servers_to_disk(&servers, Some(&state)).await?;

    emit_download_progress(
        &app_handle,
        DownloadProgressPayload {
            server_id: id.clone(),
            filename: String::from("server.jar"),
            downloaded_bytes: 1,
            total_bytes: 1,
            percent: 100.0,
            speed_mbps: 0.0,
            done: true,
        },
    );

    Ok(server)
}

#[tauri::command]
async fn start_server(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    {
        let running = state.running.lock().await;
        if running.contains_key(&id) {
            return Err(String::from("Server is already running"));
        }
    }

    let server = load_servers_cached(&state)
        .await?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| String::from("Server not found"))?;

    if let Err(port_error) = ensure_server_port_available(server.port) {
        let error_msg = format!("[SYSTEM/ERROR] {port_error}");
        emit_console_line(&app_handle, &server.id, &error_msg);
        return Err(port_error);
    }

    let settings = load_settings_from_disk().await.unwrap_or_default();
    let java = java_exec(&settings);
    let ram_limit = settings.max_ram_mb.max(256);
    let ram_mb = server.ram_mb.min(ram_limit);
    let extra_flags = split_jvm_flags(&settings.extra_jvm_flags);
    let profile_flags = split_jvm_flags(&server.jvm_args);

    let mut command = Command::new(java);
    command
        .current_dir(&server.path)
        .arg(format!("-Xms{ram_mb}M"))
        .arg(format!("-Xmx{ram_mb}M"));

    #[cfg(not(unix))]
    {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .arg("-Djline.terminal=dumb")
            .arg("-Dorg.jline.terminal.type=dumb");
    }

    for flag in extra_flags {
        command.arg(flag);
    }
    for flag in profile_flags {
        command.arg(flag);
    }

    let launch_jar = resolve_launch_jar_name(&server.path, &server.core);
    command.arg("-jar").arg(launch_jar).arg("nogui");

    #[cfg(unix)]
    let (pty_master, slave_fd) =
        crate::pty::PtyMaster::open().map_err(|e| format!("Failed to open PTY: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::fd::IntoRawFd;
        use std::os::unix::process::CommandExt;
        let slave_raw = slave_fd.into_raw_fd();

        unsafe {
            command.pre_exec(move || {
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::ioctl(slave_raw, libc::TIOCSCTTY, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
            command
                .stdin(Stdio::from_raw_fd(slave_raw))
                .stdout(Stdio::from_raw_fd(slave_raw))
                .stderr(Stdio::from_raw_fd(slave_raw));
        }
    }

    let mut child = command
        .spawn()
        .map_err(|err| format!("Failed to start server process: {err}"))?;
    let child_pid = child.id();

    #[cfg(unix)]
    let pty_master = Arc::new(pty_master);

    #[cfg(not(unix))]
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| String::from("Failed to capture server stdin"))?;

    #[cfg(not(unix))]
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| String::from("Failed to capture server stdout"))?;

    #[cfg(not(unix))]
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| String::from("Failed to capture server stderr"))?;

    let child_handle = Arc::new(AsyncMutex::new(child));

    if let Some(pid) = child_pid {
        remember_server_pid(state.inner(), &server.id, pid);
    }

    let (console_tx, console_rx) = mpsc::channel::<ConsoleEvent>(CONSOLE_CHANNEL_SIZE);

    let recent_lines = Arc::new(SyncRwLock::new(VecDeque::with_capacity(MAX_CONSOLE_LINES)));

    let running_server = RunningServer {
        #[cfg(unix)]
        pty_master: pty_master.clone(),
        #[cfg(not(unix))]
        stdin: Arc::new(AsyncMutex::new(stdin)),
        console_tx: console_tx.clone(),
        recent_lines: recent_lines.clone(),
    };

    {
        let mut running = state.running.lock().await;
        running.insert(server.id.clone(), running_server);
    }

    let _ = set_server_running_flag(&state, &server.id, true).await;

    let server_id_arc: Arc<str> = server.id.clone().into();

    #[cfg(unix)]
    spawn_pty_reader(
        server_id_arc.clone(),
        pty_master.clone(),
        console_tx.clone(),
    );

    #[cfg(not(unix))]
    {
        spawn_console_reader(server_id_arc.clone(), stdout, console_tx.clone());
        spawn_console_reader(server_id_arc.clone(), stderr, console_tx);
    }

    spawn_console_processor(
        app_handle.clone(),
        server.id.clone(),
        console_rx,
        recent_lines,
    );

    spawn_process_watcher(
        app_handle.clone(),
        state.inner().clone(),
        server.id.clone(),
        child_handle,
    );
    emit_console_line(&app_handle, &server.id, "[SYSTEM/SUCCESS] Server started");

    Ok(())
}

#[tauri::command]
async fn stop_server(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let running = {
        let running_map = state.running.lock().await;
        running_map.get(&id).cloned()
    };

    let Some(server) = running else {
        return Err(String::from("Server is not running"));
    };

    let mut stdin = server.stdin.lock().await;
    stdin
        .write_all(b"stop\n")
        .await
        .map_err(|err| format!("Failed to send stop command: {err}"))?;
    stdin
        .flush()
        .await
        .map_err(|err| format!("Failed to flush server stdin: {err}"))?;

    Ok(())
}

#[tauri::command]
async fn delete_server(state: State<'_, AppState>, id: String) -> Result<(), String> {
    {
        let running = state.running.lock().await;
        if running.contains_key(&id) {
            return Err(String::from("Stop the server before deleting it"));
        }
    }

    let mut servers = load_servers_cached(&state).await?;
    let index = servers
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| String::from("Server not found"))?;

    let server = servers.remove(index);
    if server.path.exists() {
        tokio_fs::remove_dir_all(&server.path)
            .await
            .map_err(|err| format!("Failed to remove {}: {err}", server.path.display()))?;
    }
    save_servers_to_disk(&servers, Some(&state)).await
}

#[tauri::command]
async fn update_server_profile(
    state: State<'_, AppState>,
    config: UpdateServerProfileConfig,
) -> Result<ServerConfig, String> {
    if config.port == 0 {
        return Err(String::from("Port must be between 1 and 65535"));
    }

    let mut servers = load_servers_cached(&state).await?;
    let index = servers
        .iter()
        .position(|item| item.id == config.id)
        .ok_or_else(|| String::from("Server not found"))?;

    let running_now = {
        let running = state.running.lock().await;
        running.contains_key(&config.id)
    };

    let normalized_name = server_name_or_default(&config.name);
    let normalized_ram = config.ram_mb.clamp(256, 262_144);
    let current_port = servers[index].port;

    if config.port != current_port {
        ensure_server_port_available(config.port)?;
    }

    servers[index].name = normalized_name;
    servers[index].port = config.port;
    servers[index].ram_mb = normalized_ram;
    servers[index].jvm_args = sanitized_jvm_args(&config.jvm_args);
    servers[index].running = running_now;

    let updated = servers[index].clone();

    upsert_server_property(&updated.path, "server-port", &updated.port.to_string()).await?;
    let settings = load_settings_from_disk().await.unwrap_or_default();
    write_start_scripts(
        &updated.path,
        &settings,
        updated.ram_mb,
        &updated.core,
        &updated.jvm_args,
    )
    .await?;
    save_servers_to_disk(&servers, Some(&state)).await?;

    Ok(updated)
}

#[tauri::command]
async fn open_server_folder(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let server = load_servers_cached(&state)
        .await?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| String::from("Server not found"))?;

    if !server.path.exists() {
        return Err(format!(
            "Server folder does not exist: {}",
            server.path.display()
        ));
    }

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = StdCommand::new("explorer");
        cmd.arg(&server.path);
        cmd
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut cmd = StdCommand::new("open");
        cmd.arg(&server.path);
        cmd
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut cmd = StdCommand::new("xdg-open");
        cmd.arg(&server.path);
        cmd
    };

    command.spawn().map_err(|err| {
        format!(
            "Failed to open server folder {}: {err}",
            server.path.display()
        )
    })?;

    Ok(())
}

#[tauri::command]
async fn get_server_properties(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<ServerPropertyEntry>, String> {
    let server = load_servers_cached(&state)
        .await?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| String::from("Server not found"))?;

    let properties_path = server.path.join("server.properties");
    if !properties_path.exists() {
        return Ok(Vec::new());
    }

    let content = tokio_fs::read_to_string(&properties_path)
        .await
        .map_err(|err| {
            format!(
                "Failed to read server properties {}: {err}",
                properties_path.display()
            )
        })?;

    Ok(parse_server_properties(&content))
}

#[tauri::command]
async fn save_server_properties(
    state: State<'_, AppState>,
    id: String,
    entries: Vec<ServerPropertyEntry>,
) -> Result<(), String> {
    let server = load_servers_cached(&state)
        .await?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| String::from("Server not found"))?;

    let mut normalized = Vec::<ServerPropertyEntry>::new();
    for entry in entries {
        let key = sanitized_property_key(&entry.key);
        if key.is_empty() {
            continue;
        }
        let value = sanitized_property_value(&entry.value);
        if let Some(existing) = normalized
            .iter_mut()
            .find(|item| item.key.eq_ignore_ascii_case(&key))
        {
            existing.key = key.clone();
            existing.value = value;
        } else {
            normalized.push(ServerPropertyEntry { key, value });
        }
    }

    normalized.sort_by(|a, b| a.key.cmp(&b.key));

    let properties_path = server.path.join("server.properties");
    let body = stringify_server_properties(&normalized);
    tokio_fs::write(&properties_path, body)
        .await
        .map_err(|err| {
            format!(
                "Failed to write server properties {}: {err}",
                properties_path.display()
            )
        })?;

    if let Some(server_port) = normalized.iter().find(|entry| entry.key == "server-port") {
        if let Ok(parsed_port) = server_port.value.parse::<u16>() {
            let mut servers = load_servers_cached(&state).await?;
            if let Some(server_entry) = servers.iter_mut().find(|item| item.id == id) {
                server_entry.port = parsed_port;
                save_servers_to_disk(&servers, Some(&state)).await?;
            }
        }
    }

    Ok(())
}

#[tauri::command]
async fn attach_console(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let exists = load_servers_cached(&state)
        .await?
        .into_iter()
        .any(|server| server.id == id);

    if exists {
        Ok(())
    } else {
        Err(String::from("Server not found"))
    }
}

#[tauri::command]
async fn send_command(
    state: State<'_, AppState>,
    id: String,
    command: String,
) -> Result<(), String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(String::from("Command cannot be empty"));
    }

    let normalized_command = trimmed.strip_prefix('/').unwrap_or(trimmed);

    let running = {
        let running_map = state.running.lock().await;
        running_map.get(&id).cloned()
    };

    let Some(server) = running else {
        return Err(String::from("Server is not running"));
    };

    let cmd_bytes = format!("{normalized_command}\n");

    #[cfg(unix)]
    server
        .pty_master
        .write_all(cmd_bytes.as_bytes())
        .await
        .map_err(|e| format!("Failed to send command: {e}"))?;

    #[cfg(not(unix))]
    {
        let mut stdin = server.stdin.lock().await;
        stdin
            .write_all(cmd_bytes.as_bytes())
            .await
            .map_err(|err| format!("Failed to send command to server: {err}"))?;
        stdin
            .flush()
            .await
            .map_err(|err| format!("Failed to flush server command: {err}"))?;
    }

    Ok(())
}

fn extract_plugin_name_from_yaml(yaml: &str) -> Option<String> {
    for raw_line in yaml.lines() {
        let without_comment = raw_line.split('#').next().unwrap_or("").trim_end();
        if without_comment.trim().is_empty() {
            continue;
        }

        let indent = without_comment
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .count();
        if indent > 0 {
            continue;
        }

        let trimmed = without_comment.trim_start();
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        if !key.trim().eq_ignore_ascii_case("name") {
            continue;
        }

        let cleaned = value
            .trim()
            .trim_matches(|ch| ch == '"' || ch == '\'')
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>()
            .to_ascii_lowercase();

        if !cleaned.is_empty() {
            return Some(cleaned);
        }
    }

    None
}

fn extract_commands_from_plugin_yaml(yaml: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut in_commands_section = false;
    let mut commands_section_indent = 0usize;
    let mut command_entry_indent: Option<usize> = None;

    for raw_line in yaml.lines() {
        let without_comment = raw_line.split('#').next().unwrap_or("").trim_end();
        if without_comment.trim().is_empty() {
            continue;
        }

        let indent = without_comment
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .count();
        let trimmed = without_comment.trim_start();

        if !in_commands_section {
            let Some((key, _)) = trimmed.split_once(':') else {
                continue;
            };
            if key.trim().eq_ignore_ascii_case("commands") {
                in_commands_section = true;
                commands_section_indent = indent;
                command_entry_indent = None;
            }
            continue;
        }

        if indent <= commands_section_indent {
            break;
        }

        let Some((raw_key, _)) = trimmed.split_once(':') else {
            continue;
        };
        let key = raw_key.trim();
        if key.is_empty() || key.starts_with('-') {
            continue;
        }

        match command_entry_indent {
            None => {
                command_entry_indent = Some(indent);
            }
            Some(expected_indent) if indent < expected_indent => break,
            Some(expected_indent) if indent > expected_indent => continue,
            Some(_) => {}
        }

        let normalized = key
            .trim_matches(|ch| ch == '"' || ch == '\'')
            .trim()
            .to_ascii_lowercase();
        if normalized.is_empty() || normalized.chars().any(|ch| ch.is_whitespace()) {
            continue;
        }

        commands.push(normalized);
    }

    commands
}

fn read_plugin_yaml_from_jar(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;

    for entry_name in ["plugin.yml", "paper-plugin.yml"] {
        if let Ok(mut entry) = archive.by_name(entry_name) {
            let mut content = String::new();
            if entry.read_to_string(&mut content).is_ok() {
                return Some(content);
            }
        }
    }

    None
}

fn collect_plugin_commands(server_path: &Path) -> Vec<String> {
    let plugins_dir = server_path.join("plugins");
    if !plugins_dir.exists() || !plugins_dir.is_dir() {
        return Vec::new();
    }

    let entries = match fs::read_dir(&plugins_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut commands = HashSet::<String>::new();

    for entry_result in entries {
        let Ok(entry) = entry_result else {
            continue;
        };
        let path = entry.path();

        let yaml_content = if path.is_file()
            && path
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"))
        {
            read_plugin_yaml_from_jar(&path)
        } else if path.is_dir() {
            let plugin_yml = path.join("plugin.yml");
            let paper_plugin_yml = path.join("paper-plugin.yml");
            if plugin_yml.exists() {
                fs::read_to_string(plugin_yml).ok()
            } else if paper_plugin_yml.exists() {
                fs::read_to_string(paper_plugin_yml).ok()
            } else {
                None
            }
        } else {
            None
        };

        let Some(yaml) = yaml_content else {
            continue;
        };

        let plugin_name = extract_plugin_name_from_yaml(&yaml);
        for command in extract_commands_from_plugin_yaml(&yaml) {
            commands.insert(command.clone());
            if let Some(name) = plugin_name.as_deref() {
                commands.insert(format!("{name}:{command}"));
            }
        }
    }

    let mut collected = commands.into_iter().collect::<Vec<_>>();
    collected.sort();
    collected
}

fn build_server_commands(include_extended: bool, plugin_commands: Vec<String>) -> Vec<String> {
    let mut commands = BASE_SERVER_COMMANDS
        .iter()
        .map(|command| (*command).to_string())
        .collect::<Vec<_>>();

    if include_extended {
        commands.extend(
            EXTENDED_SERVER_COMMANDS
                .iter()
                .map(|command| (*command).to_string()),
        );
    }

    commands.extend(plugin_commands);

    let mut seen = HashSet::<String>::new();
    commands.retain(|command| seen.insert(command.clone()));

    commands
}

#[tauri::command]
async fn get_server_commands(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<String>, String> {
    let running = {
        let running_map = state.running.lock().await;
        running_map.get(&id).cloned()
    };

    let server = load_servers_cached(&state)
        .await?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| String::from("Server not found"))?;

    let server_path = server.path.clone();
    let plugin_commands =
        tokio::task::spawn_blocking(move || collect_plugin_commands(&server_path))
            .await
            .unwrap_or_default();

    Ok(build_server_commands(running.is_some(), plugin_commands))
}

#[tauri::command]
async fn get_server_stats(
    state: State<'_, AppState>,
    id: String,
) -> Result<(Option<u32>, Option<u32>), String> {
    get_server_stats_internal(&state, &id).await
}

#[tauri::command]
async fn fetch_versions(core: String) -> Result<Vec<String>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| format!("Failed to initialize HTTP client: {err}"))?;

    let core = normalize_core(&core);
    if let Some(cached) = get_cached_versions(&core) {
        return Ok(cached);
    }

    let mut versions = match core.as_str() {
        "paper" | "folia" | "waterfall" => {
            let url = format!("https://api.papermc.io/v2/projects/{core}");
            let json = fetch_json(&client, &url).await?;
            json.get("versions")
                .and_then(Value::as_array)
                .ok_or_else(|| format!("Invalid {core} versions response"))?
                .iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect::<Vec<_>>()
        }
        "purpur" => {
            let json = fetch_json(&client, "https://api.purpurmc.org/v2/purpur").await?;
            json.get("versions")
                .and_then(Value::as_array)
                .ok_or_else(|| String::from("Invalid Purpur versions response"))?
                .iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect::<Vec<_>>()
        }
        "vanilla" => {
            let json = fetch_json(
                &client,
                "https://launchermeta.mojang.com/mc/game/version_manifest.json",
            )
            .await?;
            json.get("versions")
                .and_then(Value::as_array)
                .ok_or_else(|| String::from("Invalid vanilla versions response"))?
                .iter()
                .filter(|entry| {
                    entry
                        .get("type")
                        .and_then(Value::as_str)
                        .is_some_and(|kind| kind == "release" || kind == "snapshot")
                })
                .filter_map(|entry| entry.get("id").and_then(Value::as_str))
                .map(String::from)
                .collect::<Vec<_>>()
        }
        "fabric" => match fetch_json(&client, "https://meta.fabricmc.net/v2/versions/game").await {
            Ok(json) => json
                .as_array()
                .ok_or_else(|| String::from("Invalid Fabric versions response"))?
                .iter()
                .filter_map(|entry| entry.get("version").and_then(Value::as_str))
                .map(String::from)
                .collect::<Vec<_>>(),
            Err(_) => {
                let manifest = fetch_json(
                    &client,
                    "https://launchermeta.mojang.com/mc/game/version_manifest.json",
                )
                .await?;
                manifest
                    .get("versions")
                    .and_then(Value::as_array)
                    .ok_or_else(|| String::from("Invalid fallback Fabric versions response"))?
                    .iter()
                    .filter(|entry| {
                        entry
                            .get("type")
                            .and_then(Value::as_str)
                            .is_some_and(|kind| kind == "release" || kind == "snapshot")
                    })
                    .filter_map(|entry| entry.get("id").and_then(Value::as_str))
                    .map(String::from)
                    .collect::<Vec<_>>()
            }
        },
        "quilt" => {
            let json = fetch_json(&client, "https://meta.quiltmc.org/v3/versions/game").await?;
            json.as_array()
                .ok_or_else(|| String::from("Invalid Quilt versions response"))?
                .iter()
                .filter_map(|entry| entry.get("version").and_then(Value::as_str))
                .map(String::from)
                .collect::<Vec<_>>()
        }
        "forge" => {
            let json = fetch_json(
                &client,
                "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json",
            )
            .await?;
            let promos = json
                .get("promos")
                .and_then(Value::as_object)
                .ok_or_else(|| String::from("Invalid Forge versions response"))?;
            promos
                .keys()
                .filter_map(|key| key.rsplit_once('-').map(|(version, _)| version))
                .filter(|version| forge_supports_installer(version))
                .map(str::to_string)
                .collect::<Vec<_>>()
        }
        _ => return Err(format!("Unsupported core: {core}")),
    };

    sort_versions_desc(&mut versions);
    if !versions.is_empty() {
        put_cached_versions(&core, &versions);
    }
    Ok(versions)
}

#[tauri::command]
async fn get_settings() -> AppSettings {
    load_settings_from_disk().await.unwrap_or_default()
}

#[tauri::command]
async fn save_settings(settings: AppSettings) -> Result<(), String> {
    save_settings_to_disk(&settings).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            #[cfg(target_os = "windows")]
            {
                use std::fs;
                if let Ok(servers_dir) = servers_root_dir() {
                    if let Ok(entries) = fs::read_dir(&servers_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() {
                                let sh_file = path.join("start.sh");
                                if sh_file.exists() {
                                    let _ = fs::remove_file(&sh_file);
                                }
                            }
                        }
                    }
                }
            }

            let show_item =
                tauri::menu::MenuItem::with_id(app, "show", "Open Lodestone", true, None::<&str>)?;
            let quit_item =
                tauri::menu::MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&show_item, &quit_item])?;

            let app_handle = app.handle().clone();
            let _ = tauri::tray::TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        let state = app.state::<AppState>();
                        kill_tracked_server_processes_if_needed(state.inner());
                        app_handle.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if should_minimize_to_tray_on_close() {
                    api.prevent_close();
                    let _ = window.hide();
                } else if window.label() == "main" {
                    let state = window.state::<AppState>();
                    kill_tracked_server_processes_if_needed(state.inner());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_servers,
            create_server,
            start_server,
            stop_server,
            delete_server,
            update_server_profile,
            open_server_folder,
            get_server_properties,
            save_server_properties,
            attach_console,
            send_command,
            get_server_commands,
            get_server_stats,
            fetch_versions,
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
