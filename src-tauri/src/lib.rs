use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tokio::{
    fs as tokio_fs,
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::Mutex as AsyncMutex,
};
use uuid::Uuid;

const CONSOLE_EVENT: &str = "console_line";
const DOWNLOAD_PROGRESS_EVENT: &str = "download_progress";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    id: String,
    name: String,
    core: String,
    version: String,
    port: u16,
    ram_mb: u32,
    path: PathBuf,
    running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewServerConfig {
    name: String,
    core: String,
    version: String,
    port: u16,
    ram_mb: u32,
    #[serde(default)]
    properties: ServerPropertiesConfig,
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
pub struct AppSettings {
    java_path: String,
    max_ram_mb: u32,
    extra_jvm_flags: String,
    minimize_to_tray: bool,
    autostart_servers: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            java_path: String::from("java"),
            max_ram_mb: 4096,
            extra_jvm_flags: String::new(),
            minimize_to_tray: false,
            autostart_servers: false,
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
    stdin: Arc<AsyncMutex<ChildStdin>>,
}

#[derive(Clone, Default)]
struct AppState {
    running: Arc<AsyncMutex<HashMap<String, RunningServer>>>,
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

async fn save_servers_to_disk(servers: &[ServerConfig]) -> Result<(), String> {
    ensure_app_dirs().await?;
    let file_path = servers_file_path()?;
    let body = serde_json::to_vec_pretty(servers)
        .map_err(|err| format!("Failed to serialize server list: {err}"))?;
    tokio_fs::write(&file_path, body)
        .await
        .map_err(|err| format!("Failed to write servers.json: {err}"))
}

async fn set_server_running_flag(server_id: &str, running: bool) -> Result<(), String> {
    let mut servers = load_servers_from_disk().await?;
    for server in &mut servers {
        if server.id == server_id {
            server.running = running;
        }
    }
    save_servers_to_disk(&servers).await
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

fn java_exec(settings: &AppSettings) -> String {
    let trimmed = settings.java_path.trim();
    if trimmed.is_empty() {
        String::from("java")
    } else {
        String::from(trimmed)
    }
}

fn split_jvm_flags(raw: &str) -> Vec<String> {
    raw.split_whitespace().map(String::from).collect()
}

fn server_name_or_default(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        String::from("server")
    } else {
        String::from(trimmed)
    }
}

fn emit_console_line(app_handle: &AppHandle, server_id: &str, line: impl Into<String>) {
    let payload = ConsoleLinePayload {
        server_id: String::from(server_id),
        line: line.into(),
        timestamp: now_timestamp_secs(),
    };
    let _ = app_handle.emit(CONSOLE_EVENT, payload);
}

fn emit_download_progress(
    app_handle: &AppHandle,
    server_id: &str,
    filename: &str,
    downloaded_bytes: u64,
    total_bytes: u64,
    percent: f64,
    speed_mbps: f64,
    done: bool,
) {
    let payload = DownloadProgressPayload {
        server_id: String::from(server_id),
        filename: String::from(filename),
        downloaded_bytes,
        total_bytes,
        percent,
        speed_mbps,
        done,
    };
    let _ = app_handle.emit(DOWNLOAD_PROGRESS_EVENT, payload);
}

fn normalize_core(core: &str) -> String {
    core.trim().to_ascii_lowercase()
}

fn parse_filename_from_url(url: &str) -> String {
    let file_name = url.rsplit('/').next().unwrap_or("server.jar");
    if file_name.is_empty() {
        String::from("server.jar")
    } else {
        String::from(file_name)
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
        .map(String::from)
        .unwrap_or_else(|| parse_filename_from_url(url));

    let mut stream = response.bytes_stream();
    let mut file = tokio_fs::File::create(destination)
        .await
        .map_err(|err| format!("Failed to create file {}: {err}", destination.display()))?;

    let mut downloaded_bytes: u64 = 0;
    let started_at = Instant::now();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|err| format!("Download stream failure: {err}"))?;
        file.write_all(&chunk)
            .await
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
            server_id,
            &file_name,
            downloaded_bytes,
            total_bytes,
            percent,
            speed_mbps,
            false,
        );
    }

    file.flush()
        .await
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

async fn forge_installer_url(client: &Client, mc_version: &str) -> Result<String, String> {
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

    let forge_version = promos_map
        .get(&latest_key)
        .or_else(|| promos_map.get(&recommended_key))
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| format!("No Forge build metadata found for Minecraft {mc_version}"))?;

    Ok(format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{mc_version}-{forge_version}/forge-{mc_version}-{forge_version}-installer.jar"
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

async fn write_bootstrap_files(
    server_dir: &Path,
    settings: &AppSettings,
    ram_mb: u32,
    port: u16,
    properties: &ServerPropertiesConfig,
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

    let java = java_exec(settings);
    let extra = settings.extra_jvm_flags.trim();
    let extra_segment = if extra.is_empty() {
        String::new()
    } else {
        format!("{extra} ")
    };

    let bat_body = format!(
        "@echo off\r\n\"{java}\" -Xms{ram_mb}M -Xmx{ram_mb}M {extra_segment}-jar server.jar nogui\r\npause\r\n"
    );
    let sh_body =
        format!("#!/usr/bin/env sh\n\"{java}\" -Xms{ram_mb}M -Xmx{ram_mb}M {extra_segment}-jar server.jar nogui\n");

    let bat_path = server_dir.join("start.bat");
    let sh_path = server_dir.join("start.sh");

    tokio_fs::write(&bat_path, bat_body)
        .await
        .map_err(|err| format!("Failed to write {}: {err}", bat_path.display()))?;
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
                server_id,
                "fabric-installer",
                0,
                0,
                95.0,
                0.0,
                false,
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
            tokio_fs::copy(&generated_jar, &server_jar_path)
                .await
                .map_err(|err| {
                    format!(
                        "Failed to copy generated Fabric JAR {} to {}: {err}",
                        generated_jar.display(),
                        server_jar_path.display()
                    )
                })?;
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
                server_id,
                "forge-installer",
                0,
                0,
                95.0,
                0.0,
                false,
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
            tokio_fs::copy(&generated_jar, &server_jar_path)
                .await
                .map_err(|err| {
                    format!(
                        "Failed to copy generated Forge JAR {} to {}: {err}",
                        generated_jar.display(),
                        server_jar_path.display()
                    )
                })?;
        }
        _ => {
            return Err(format!("Unsupported core: {core}"));
        }
    }

    Ok(())
}

fn spawn_console_reader<R>(app_handle: AppHandle, server_id: String, reader: R)
where
    R: AsyncRead + Send + Unpin + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(raw_line)) = lines.next_line().await {
            let stripped = strip_ansi_escapes::strip(raw_line.as_bytes());
            let normalized = String::from_utf8_lossy(&stripped).to_string();
            emit_console_line(&app_handle, &server_id, normalized);
        }
    });
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
        let _ = set_server_running_flag(&server_id, false).await;

        match status_result {
            Ok(status) => {
                if status.success() {
                    emit_console_line(
                        &app_handle,
                        &server_id,
                        format!("[SYSTEM/SUCCESS] Server stopped ({status})"),
                    );
                } else {
                    emit_console_line(
                        &app_handle,
                        &server_id,
                        format!("[SYSTEM/ERROR] Server exited unexpectedly ({status})"),
                    );
                }
            }
            Err(err) => {
                emit_console_line(
                    &app_handle,
                    &server_id,
                    format!("[SYSTEM/ERROR] Failed to wait for process: {err}"),
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
    let mut servers = match load_servers_from_disk().await {
        Ok(items) => items,
        Err(_) => Vec::new(),
    };

    let running_ids = {
        let running = state.running.lock().await;
        running_ids(&running)
    };

    for server in &mut servers {
        server.running = running_ids.contains(&server.id);
    }
    Ok(servers)
}

#[tauri::command]
async fn create_server(
    app_handle: AppHandle,
    config: NewServerConfig,
) -> Result<ServerConfig, String> {
    ensure_app_dirs().await?;

    let core = normalize_core(&config.core);
    let supported = ["paper", "purpur", "fabric", "forge", "folia", "vanilla"];
    if !supported.contains(&core.as_str()) {
        return Err(format!("Unsupported core: {}", config.core));
    }

    let version = config.version.trim();
    if version.is_empty() {
        return Err(String::from("Server version is required"));
    }

    let id = Uuid::new_v4().to_string();
    let server_dir = servers_root_dir()?.join(&id);
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
        config.port,
        &config.properties,
    )
    .await?;

    let server = ServerConfig {
        id: String::from(&id),
        name: server_name_or_default(&config.name),
        core,
        version: String::from(version),
        port: config.port,
        ram_mb: config.ram_mb,
        path: server_dir,
        running: false,
    };

    let mut servers = load_servers_from_disk().await?;
    servers.push(server.clone());
    save_servers_to_disk(&servers).await?;

    emit_download_progress(&app_handle, &id, "server.jar", 1, 1, 100.0, 0.0, true);

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

    let server = load_servers_from_disk()
        .await?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| String::from("Server not found"))?;

    let settings = load_settings_from_disk().await.unwrap_or_default();
    let java = java_exec(&settings);
    let ram_limit = settings.max_ram_mb.max(256);
    let ram_mb = server.ram_mb.min(ram_limit);
    let extra_flags = split_jvm_flags(&settings.extra_jvm_flags);

    let mut command = Command::new(java);
    command
        .current_dir(&server.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg(format!("-Xms{ram_mb}M"))
        .arg(format!("-Xmx{ram_mb}M"));

    for flag in extra_flags {
        command.arg(flag);
    }

    command.arg("-jar").arg("server.jar").arg("nogui");

    let mut child = command
        .spawn()
        .map_err(|err| format!("Failed to start server process: {err}"))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| String::from("Failed to capture server stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| String::from("Failed to capture server stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| String::from("Failed to capture server stderr"))?;

    let child_handle = Arc::new(AsyncMutex::new(child));
    let running_server = RunningServer {
        stdin: Arc::new(AsyncMutex::new(stdin)),
    };

    {
        let mut running = state.running.lock().await;
        running.insert(server.id.clone(), running_server);
    }

    let _ = set_server_running_flag(&server.id, true).await;

    spawn_console_reader(app_handle.clone(), server.id.clone(), stdout);
    spawn_console_reader(app_handle.clone(), server.id.clone(), stderr);
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

    let mut servers = load_servers_from_disk().await?;
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
    save_servers_to_disk(&servers).await
}

#[tauri::command]
async fn attach_console(id: String) -> Result<(), String> {
    let exists = load_servers_from_disk()
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
    if command.trim().is_empty() {
        return Err(String::from("Command cannot be empty"));
    }

    let running = {
        let running_map = state.running.lock().await;
        running_map.get(&id).cloned()
    };

    let Some(server) = running else {
        return Err(String::from("Server is not running"));
    };

    let mut stdin = server.stdin.lock().await;
    stdin
        .write_all(format!("{}\n", command.trim()).as_bytes())
        .await
        .map_err(|err| format!("Failed to send command to server: {err}"))?;
    stdin
        .flush()
        .await
        .map_err(|err| format!("Failed to flush server command: {err}"))?;

    Ok(())
}

#[tauri::command]
async fn fetch_versions(core: String) -> Result<Vec<String>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| format!("Failed to initialize HTTP client: {err}"))?;

    let core = normalize_core(&core);
    let versions = match core.as_str() {
        "paper" | "folia" => {
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
                .filter_map(|entry| entry.get("id").and_then(Value::as_str))
                .map(String::from)
                .collect::<Vec<_>>()
        }
        "fabric" => {
            let json = fetch_json(&client, "https://meta.fabricmc.net/v2/versions/game").await?;
            json.as_array()
                .ok_or_else(|| String::from("Invalid Fabric versions response"))?
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
            let mut versions = promos
                .keys()
                .filter_map(|key| key.split('-').next())
                .map(String::from)
                .collect::<Vec<_>>();
            versions.sort_by(|a, b| b.cmp(a));
            versions.dedup();
            versions
        }
        _ => return Err(format!("Unsupported core: {core}")),
    };

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
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_servers,
            create_server,
            start_server,
            stop_server,
            delete_server,
            attach_console,
            send_command,
            fetch_versions,
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
