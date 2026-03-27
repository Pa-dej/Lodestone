<script lang="ts">
  import Toggle from "$lib/components/Toggle.svelte";
  import CustomSelect from "$lib/components/CustomSelect.svelte";
  import { createServer, fetchVersions, serverState } from "$lib/stores/servers.svelte";
  import { i18nState, t } from "$lib/stores/i18n.svelte";
  import type { CoreType, NewServerConfig, ServerConfig } from "$lib/types";
  import PaperIcon from "../../icons/servers/Paper.svg?raw";
  import PurpurIcon from "../../icons/servers/Purpur.svg?raw";
  import FabricIcon from "../../icons/servers/Fabric.svg?raw";
  import QuiltIcon from "../../icons/servers/Quilt.svg?raw";
  import ForgeIcon from "../../icons/servers/Forge.svg?raw";
  import FoliaIcon from "../../icons/servers/Folia.svg?raw";
  import VelocityIcon from "../../icons/servers/Velocity.svg?raw";
  import WaterfallIcon from "../../icons/servers/Waterfall.svg?raw";
  import BungeeCordIcon from "../../icons/servers/BungeeCord.svg?raw";
  import VanillaIcon from "../../icons/Server.svg?raw";

  interface CoreOption {
    id: CoreType;
    group: "server" | "proxy";
    iconSvg: string;
    color: string;
    name: string;
    description: string;
    deprecated?: boolean;
  }

  interface Props {
    open: boolean;
    onClose: () => void;
    onCreated?: (server: ServerConfig) => void;
  }

  let { open, onClose, onCreated = () => {} }: Props = $props();

  const coreOptions = $derived<CoreOption[]>([
    {
      id: "paper",
      group: "server",
      iconSvg: PaperIcon,
      color: "var(--core-paper)",
      name: "Paper",
      description:
        i18nState.language === "ru"
          ? "Высокая производительность, плагины"
          : "High performance, plugins",
    },
    {
      id: "purpur",
      group: "server",
      iconSvg: PurpurIcon,
      color: "var(--core-purpur)",
      name: "Purpur",
      description:
        i18nState.language === "ru" ? "Форк Paper, больше настроек" : "Paper fork with extra settings",
    },
    {
      id: "fabric",
      group: "server",
      iconSvg: FabricIcon,
      color: "var(--core-fabric)",
      name: "Fabric",
      description: i18nState.language === "ru" ? "Лёгкий загрузчик модов" : "Lightweight mod loader",
    },
    {
      id: "quilt",
      group: "server",
      iconSvg: QuiltIcon,
      color: "var(--core-quilt)",
      name: "Quilt",
      description:
        i18nState.language === "ru" ? "Современный загрузчик модов" : "Modern mod loader",
    },
    {
      id: "forge",
      group: "server",
      iconSvg: ForgeIcon,
      color: "var(--core-forge)",
      name: "Forge",
      description: i18nState.language === "ru" ? "Классический загрузчик модов" : "Classic mod loader",
    },
    {
      id: "folia",
      group: "server",
      iconSvg: FoliaIcon,
      color: "var(--core-folia)",
      name: "Folia",
      description: i18nState.language === "ru" ? "Многопоточный форк Paper" : "Multithreaded Paper fork",
    },
    {
      id: "velocity",
      group: "proxy",
      iconSvg: VelocityIcon,
      color: "var(--core-velocity)",
      name: "Velocity",
      description: i18nState.language === "ru" ? "Современный быстрый прокси" : "Modern fast proxy",
    },
    {
      id: "waterfall",
      group: "proxy",
      iconSvg: WaterfallIcon,
      color: "var(--core-waterfall)",
      name: "Waterfall",
      description:
        i18nState.language === "ru"
          ? "Bungee-прокси"
          : "Bungee proxy",
    },
    {
      id: "bungeecord",
      group: "proxy",
      iconSvg: BungeeCordIcon,
      color: "var(--core-bungeecord)",
      name: "BungeeCord",
      description: i18nState.language === "ru" ? "Классический прокси" : "Classic proxy",
    },
    {
      id: "vanilla",
      group: "server",
      iconSvg: VanillaIcon,
      color: "var(--core-vanilla)",
      name: "Vanilla",
      description: i18nState.language === "ru" ? "Официальный сервер Mojang" : "Official Mojang server",
    },
  ]);

  let currentStep = $state(0);
  let selectedCore = $state<CoreType>("paper");
  let serverName = $state("my-server");
  let versions = $state<string[]>([]);
  let selectedVersion = $state("");
  let releaseOnly = $state(true);
  let port = $state(25565);
  let ramMb = $state(2048);
  let jvmArgs = $state("");
  let motd = $state("A Lodestone Minecraft Server");
  let gamemode = $state<"survival" | "creative" | "adventure" | "spectator">("survival");
  let difficulty = $state<"peaceful" | "easy" | "normal" | "hard">("normal");
  let onlineMode = $state(true);
  let pvpEnabled = $state(true);
  let viewDistance = $state(10);
  let loadingVersions = $state(false);
  let modalError = $state<string | null>(null);
  let finished = $state(false);
  let failed = $state(false);
  let creationStarted = $state(false);
  let previousOpen = false;
  let versionsRequestId = 0;
  const VERSION_FETCH_TIMEOUT_MS = 45000;
  const BUNGEE_LATEST_VERSION = "latest";

  const serverCoreOptions = $derived(coreOptions.filter((option) => option.group === "server"));
  const proxyCoreOptions = $derived(coreOptions.filter((option) => option.group === "proxy"));
  const isProxyCore = $derived(["velocity", "waterfall", "bungeecord"].includes(selectedCore));
  const minRamMb = $derived(Math.max(512, serverState.ramLimits.min_mb || 512));
  const maxRamMb = $derived(Math.max(minRamMb, serverState.ramLimits.max_mb || 16384));

  const releaseVersionPattern = /^(?:\d+\.\d+(?:\.\d+){0,2}|\d{2}\.\d{2}(?:\.\d+)?)$/;

  function isReleaseVersion(version: string): boolean {
    const normalized = version.trim().toLowerCase();
    if (!normalized) {
      return false;
    }
    if (normalized.includes("snapshot") || normalized.includes("pre") || normalized.includes("rc")) {
      return false;
    }
    return releaseVersionPattern.test(normalized);
  }

  const visibleVersions = $derived.by(() => {
    if (!releaseOnly) {
      return versions;
    }
    return versions.filter((version) => isReleaseVersion(version));
  });

  const versionOptions = $derived.by(() => {
    if (loadingVersions) {
      return [{ value: "", label: t("modal_loading_versions"), disabled: true }];
    }
    if (visibleVersions.length === 0) {
      return [{ value: "", label: t("modal_no_versions"), disabled: true }];
    }
    return visibleVersions.map((version) => ({ value: version, label: version }));
  });

  const gamemodeOptions = $derived([
    { value: "survival", label: i18nState.language === "ru" ? "Выживание" : "Survival" },
    { value: "creative", label: i18nState.language === "ru" ? "Креатив" : "Creative" },
    { value: "adventure", label: i18nState.language === "ru" ? "Приключение" : "Adventure" },
    { value: "spectator", label: i18nState.language === "ru" ? "Наблюдатель" : "Spectator" },
  ]);

  const difficultyOptions = $derived([
    { value: "peaceful", label: i18nState.language === "ru" ? "Мирная" : "Peaceful" },
    { value: "easy", label: i18nState.language === "ru" ? "Лёгкая" : "Easy" },
    { value: "normal", label: i18nState.language === "ru" ? "Нормальная" : "Normal" },
    { value: "hard", label: i18nState.language === "ru" ? "Сложная" : "Hard" },
  ]);

  const progressPercent = $derived.by(() => {
    if (finished) {
      return 100;
    }
    const value = serverState.download?.percent ?? 0;
    return Number.isFinite(value) ? Math.max(0, Math.min(value, 100)) : 0;
  });

  const downloadFilename = $derived(serverState.download?.filename ?? "server.jar");
  const downloadSize = $derived.by(() => {
    const bytes = serverState.download?.total_bytes ?? 0;
    if (bytes <= 0) {
      return "—";
    }
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  });

  const speedText = $derived.by(() => {
    const speed = serverState.download?.speed_mbps ?? 0;
    return `${speed.toFixed(2)} MB/s`;
  });

  const showWaterfallWarning = $derived(selectedCore === "waterfall");
  const isBungeeCord = $derived(selectedCore === "bungeecord");
  const createButtonLabel = $derived.by(() =>
    isBungeeCord
      ? i18nState.language === "ru"
        ? "Скачать последний"
        : "Download latest"
      : t("modal_create"),
  );

  async function fetchVersionsWithTimeout(core: CoreType): Promise<string[]> {
    const request = fetchVersions(core).catch(() => []);
    const timeout = new Promise<string[]>((resolve) => {
      setTimeout(() => resolve([]), VERSION_FETCH_TIMEOUT_MS);
    });
    return Promise.race([request, timeout]);
  }

  async function loadVersionsForCore(core: CoreType): Promise<void> {
    const requestId = ++versionsRequestId;
    if (core === "bungeecord") {
      loadingVersions = false;
      modalError = null;
      versions = [BUNGEE_LATEST_VERSION];
      selectedVersion = BUNGEE_LATEST_VERSION;
      return;
    }

    loadingVersions = true;
    modalError = null;
    versions = [];
    selectedVersion = "";
    try {
      const result = await fetchVersionsWithTimeout(core);
      if (requestId !== versionsRequestId) {
        return;
      }
      versions = result;
      const filtered = releaseOnly ? result.filter((version) => isReleaseVersion(version)) : result;
      selectedVersion = filtered[0] ?? "";
      if (!selectedVersion) {
        modalError = t("modal_no_versions");
      }
    } finally {
      if (requestId === versionsRequestId) {
        loadingVersions = false;
      }
    }
  }

  async function moveToConfig(): Promise<void> {
    currentStep = 1;
    await loadVersionsForCore(selectedCore);
  }

  function moveBack(): void {
    if (currentStep > 0) {
      currentStep -= 1;
    }
  }

  function closeIfAllowed(): void {
    if (serverState.creating) {
      return;
    }
    onClose();
  }

  function resetState(): void {
    currentStep = 0;
    selectedCore = "paper";
    serverName = "my-server";
    versions = [];
    selectedVersion = "";
    releaseOnly = true;
    port = 25565;
    ramMb = Math.min(maxRamMb, Math.max(minRamMb, 2048));
    jvmArgs = "";
    motd = "A Lodestone Minecraft Server";
    gamemode = "survival";
    difficulty = "normal";
    onlineMode = true;
    pvpEnabled = true;
    viewDistance = 10;
    loadingVersions = false;
    modalError = null;
    finished = false;
    failed = false;
    creationStarted = false;
    versionsRequestId += 1;
  }

  async function submitCreation(): Promise<void> {
    const normalizedName = serverName.trim();
    if (!normalizedName) {
      modalError = t("field_server_name");
      return;
    }

    if (!selectedVersion) {
      modalError = t("modal_select_version_error");
      return;
    }

    if (!Number.isFinite(ramMb) || ramMb < minRamMb || ramMb > maxRamMb) {
      modalError = `${t("field_ram_mb")}: ${minRamMb}-${maxRamMb}`;
      return;
    }

    modalError = null;
    failed = false;
    creationStarted = true;
    currentStep = 2;

    const payload: NewServerConfig = {
      name: normalizedName,
      core: selectedCore,
      version: selectedVersion,
      port,
      ram_mb: Math.round(ramMb),
      jvm_args: jvmArgs.trim(),
      properties: {
        motd: motd.trim() || "A Lodestone Minecraft Server",
        gamemode,
        difficulty,
        online_mode: onlineMode,
        pvp: pvpEnabled,
        view_distance: viewDistance,
      },
    };

    const created = await createServer(payload);
    if (!created) {
      failed = true;
      modalError = serverState.createError ?? t("modal_create_error");
      return;
    }

    finished = true;
    onCreated(created);
  }

  function stepState(index: number): "pending" | "current" | "done" {
    if (index < currentStep || (finished && index <= 2)) {
      return "done";
    }
    if (index === currentStep) {
      return "current";
    }
    return "pending";
  }

  $effect(() => {
    if (open && !previousOpen) {
      resetState();
    }
    previousOpen = open;
  });

  $effect(() => {
    if (!open || currentStep !== 1 || loadingVersions || isBungeeCord) {
      return;
    }

    if (visibleVersions.length === 0) {
      selectedVersion = "";
      modalError = t("modal_no_versions");
      return;
    }

    if (!selectedVersion || !visibleVersions.includes(selectedVersion)) {
      selectedVersion = visibleVersions[0] ?? "";
    }

    if (modalError === t("modal_no_versions")) {
      modalError = null;
    }
  });
</script>

{#if open}
  <div class="modal-overlay" role="presentation">
    <div class="modal panel" role="dialog" aria-modal="true" aria-label={t("modal_new_server")}>
      <header class="modal-header">
        <h2 class="panel-title">{t("modal_new_server")}</h2>
        <button type="button" class="btn-icon" onclick={closeIfAllowed}>✕</button>
      </header>

      <div class="step-indicator">
        {#each [0, 1, 2] as index}
          <span class={`step-dot ${stepState(index)}`}></span>
        {/each}
      </div>

      {#if currentStep === 0}
        <div class="core-grid">
          <div class="core-column">
            <span class="core-column-label">{i18nState.language === "ru" ? "Сервер" : "Server"}</span>
            {#each serverCoreOptions as option}
              <button
                type="button"
                class="core-option"
                class:selected={option.id === selectedCore}
                onclick={() => (selectedCore = option.id)}
              >
                <span class="core-icon" style={`color:${option.color}`}>{@html option.iconSvg}</span>
                <span class="core-content">
                  <span class="core-name">{option.name}</span>
                  <span class="core-description">{option.description}</span>
                </span>
                <span class="core-check">{option.id === selectedCore ? "✓" : ""}</span>
              </button>
            {/each}
          </div>

          <div class="core-column">
            <span class="core-column-label">{i18nState.language === "ru" ? "Прокси" : "Proxy"}</span>
            {#each proxyCoreOptions as option}
              <button
                type="button"
                class="core-option"
                class:selected={option.id === selectedCore}
                onclick={() => (selectedCore = option.id)}
              >
                <span class="core-icon" style={`color:${option.color}`}>{@html option.iconSvg}</span>
                <span class="core-content">
                  <span class="core-name">{option.name}</span>
                  <span class="core-description">{option.description}</span>
                </span>
                {#if option.deprecated}
                  <span class="core-badge">EOL</span>
                {:else}
                  <span class="core-check">{option.id === selectedCore ? "✓" : ""}</span>
                {/if}
              </button>
            {/each}
          </div>
        </div>

        <footer class="modal-footer">
          <button type="button" class="btn btn-primary" onclick={moveToConfig}>{t("modal_next")}</button>
        </footer>
      {/if}

      {#if currentStep === 1}
        <div class="form-grid">
          <label class="field">
            <span class="field-label">{t("field_server_name")}</span>
            <input class="input" bind:value={serverName} placeholder="my-server" />
          </label>

          {#if isBungeeCord}
            <div class="field">
              <span class="field-label">{t("field_version")}</span>
              <div class="input static-version">
                {i18nState.language === "ru"
                  ? "Последний успешный билд (auto)"
                  : "Latest successful build (auto)"}
              </div>
            </div>
          {:else}
            <div class="field">
              <div class="field-label-row">
                <span class="field-label">{t("field_version")}</span>
                <label class="release-only">
                  <input type="checkbox" bind:checked={releaseOnly} />
                  <span>{t("field_release_only")}</span>
                </label>
              </div>
              <CustomSelect
                value={selectedVersion}
                options={versionOptions}
                disabled={loadingVersions || visibleVersions.length === 0}
                onChange={(value) => {
                  selectedVersion = value;
                }}
              />
            </div>
          {/if}

          {#if showWaterfallWarning}
            <div class="alert alert-warning span-2">
              <span class="alert-icon">⚠</span>
              <div class="alert-text">
                <div class="alert-title">{t("waterfall_deprecated_title")}</div>
                <div class="alert-sub">{t("waterfall_deprecated_text")}</div>
              </div>
            </div>
          {/if}

          <label class="field">
            <span class="field-label">{t("field_port")}</span>
            <input class="input" type="number" min={1} max={65535} bind:value={port} />
          </label>

          <div class="field">
            <span class="field-label">{t("field_ram_mb")}</span>
            <div class="slider-row">
              <input class="range-input" type="range" min={minRamMb} max={maxRamMb} step={256} bind:value={ramMb} />
              <span class="tag">{ramMb} MB</span>
            </div>
          </div>

          <label class="field span-2">
            <span class="field-label">{t("field_jvm_args")}</span>
            <input class="input" bind:value={jvmArgs} placeholder="-XX:+UseG1GC -XX:+ParallelRefProcEnabled" />
          </label>

          <label class="field span-2">
            <span class="field-label">{t("field_motd")}</span>
            <input class="input" bind:value={motd} placeholder="A Lodestone Minecraft Server" />
          </label>

          {#if !isProxyCore}
            <div class="field">
              <span class="field-label">{t("field_gamemode")}</span>
              <CustomSelect
                value={gamemode}
                options={gamemodeOptions}
                onChange={(value) => {
                  gamemode = value as typeof gamemode;
                }}
              />
            </div>

            <div class="field">
              <span class="field-label">{t("field_difficulty")}</span>
              <CustomSelect
                value={difficulty}
                options={difficultyOptions}
                onChange={(value) => {
                  difficulty = value as typeof difficulty;
                }}
              />
            </div>

            <div class="field span-2">
              <span class="field-label">{t("field_view_distance")}</span>
              <div class="slider-row">
                <input class="range-input" type="range" min={3} max={32} step={1} bind:value={viewDistance} />
                <span class="tag">{viewDistance} {t("unit_chunks")}</span>
              </div>
            </div>

            <div class="field span-2 toggles-inline">
              <Toggle
                label={t("field_online_mode")}
                description={t("field_online_mode_desc")}
                checked={onlineMode}
                onToggle={(value) => (onlineMode = value)}
              />
              <Toggle
                label={t("field_pvp")}
                description={t("field_pvp_desc")}
                checked={pvpEnabled}
                onToggle={(value) => (pvpEnabled = value)}
              />
            </div>
          {/if}
        </div>

        {#if modalError}
          <div class="alert alert-danger">
            <span class="alert-icon">✕</span>
            <div class="alert-text">
              <div class="alert-title">{t("error_title")}</div>
              <div class="alert-sub">{modalError}</div>
            </div>
          </div>
        {/if}

        <footer class="modal-footer">
          <button type="button" class="btn btn-ghost" onclick={moveBack}>{t("modal_back")}</button>
          <button type="button" class="btn btn-primary" onclick={submitCreation}>
            {createButtonLabel}
          </button>
        </footer>
      {/if}

      {#if currentStep === 2}
        <div class="download-step">
          <div class="download-icon">
            {#if finished}
              ✓
            {:else if failed}
              ✕
            {:else}
              <svg
                class="download-svg"
                width="36"
                height="36"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
              >
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <g class="arrow">
                  <line x1="12" y1="3" x2="12" y2="15" />
                  <polyline points="8,11 12,15 16,11" />
                </g>
              </svg>
            {/if}
          </div>

          <div class="download-text">
            {#if finished}
              <h3>{t("modal_done_title")}</h3>
              <p>{t("modal_done_subtitle")}</p>
            {:else if failed}
              <h3>{t("modal_failed_title")}</h3>
              <p>{modalError}</p>
            {:else}
              <h3>{t("modal_download_title")}</h3>
              <p>{downloadFilename} · {downloadSize}</p>
            {/if}
          </div>

          <div class="progress-wrap">
            <div class="progress-track">
              <div class="progress-fill" style={`width:${progressPercent}%`}></div>
            </div>
            <div class="progress-meta">
              <span>{progressPercent.toFixed(0)}%</span>
              <span>{speedText}</span>
            </div>
          </div>
        </div>

        <footer class="modal-footer">
          {#if creationStarted && !finished && !failed}
            <button type="button" class="btn btn-ghost" disabled>{t("modal_wait")}</button>
          {/if}
        </footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    display: grid;
    place-items: center;
    z-index: 200;
    padding: 16px;
  }

  .modal {
    width: min(760px, 100%);
    max-height: min(90vh, 760px);
    overflow: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .step-indicator {
    display: flex;
    gap: 8px;
    justify-content: center;
  }

  .step-dot {
    width: 10px;
    height: 10px;
    border-radius: 999px;
    border: 0.5px solid var(--border);
    background: transparent;
    transition: box-shadow var(--tr), background var(--tr), border-color var(--tr);
  }

  .step-dot.current {
    border-color: var(--accent);
    background: var(--accent);
    box-shadow: 0 0 0 4px var(--accent-glow);
  }

  .step-dot.done {
    border-color: var(--accent);
    background: var(--accent);
  }

  .core-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }

  .core-column {
    display: grid;
    gap: 8px;
    align-content: start;
  }

  .core-column-label {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: 0 2px;
  }

  .core-option {
    display: grid;
    grid-template-columns: 36px 1fr 22px;
    align-items: center;
    gap: 12px;
    border-radius: var(--r-md);
    border: 0.5px solid var(--border);
    background: var(--surface);
    color: var(--text);
    text-align: left;
    padding: 12px;
    cursor: pointer;
    transition: border-color var(--tr), background var(--tr);
  }

  .core-option:hover {
    border-color: var(--text-hint);
  }

  .core-option.selected {
    border-color: var(--accent);
    background: var(--accent-bg);
  }

  .core-icon {
    width: 32px;
    height: 32px;
    border-radius: var(--r-md);
    background: var(--surface-2);
    display: grid;
    place-items: center;
  }

  .core-icon :global(svg) {
    width: 18px;
    height: 18px;
  }

  .core-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .core-name {
    font-size: 14px;
    color: var(--text);
  }

  .core-description {
    font-size: 12px;
    color: var(--text-muted);
  }

  .core-check {
    color: var(--accent);
    font-size: 16px;
    text-align: right;
  }

  .core-badge {
    font-size: 10px;
    color: #c46060;
    border: 0.5px solid rgba(196, 96, 96, 0.4);
    background: rgba(196, 96, 96, 0.12);
    border-radius: 999px;
    padding: 1px 7px;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field-label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .release-only {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
    font-size: 12px;
    user-select: none;
  }

  .release-only input {
    accent-color: var(--accent);
  }

  .static-version {
    color: var(--text-muted);
    cursor: default;
  }

  .span-2 {
    grid-column: span 2;
  }

  .slider-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .range-input {
    width: 100%;
    accent-color: var(--accent);
  }

  .toggles-inline {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .download-step {
    display: grid;
    gap: 14px;
    justify-items: center;
    text-align: center;
    padding: 12px 0;
  }

  .download-icon {
    width: 66px;
    height: 66px;
    border-radius: var(--r-lg);
    border: 0.5px solid var(--border);
    background: var(--surface-2);
    display: grid;
    place-items: center;
    font-size: 28px;
    color: var(--accent);
  }

  .download-svg {
    width: 36px;
    height: 36px;
    display: block;
  }

  .download-svg .arrow {
    animation: teleport 2s cubic-bezier(.65, 0, .35, 1) infinite;
    transform-origin: 12px 9px;
  }

  @keyframes teleport {
    0% {
      transform: translateY(0) scaleY(1) scaleX(1);
      opacity: 1;
    }
    30% {
      transform: translateY(5px) scaleY(.3) scaleX(.6);
      opacity: 0;
    }
    31% {
      transform: translateY(-7px) scaleY(.2) scaleX(.5);
      opacity: 0;
    }
    55% {
      transform: translateY(0) scaleY(1.1) scaleX(.95);
      opacity: 1;
    }
    65% {
      transform: translateY(1px) scaleY(.95) scaleX(1.02);
      opacity: 1;
    }
    75% {
      transform: translateY(0) scaleY(1) scaleX(1);
      opacity: 1;
    }
    100% {
      transform: translateY(0) scaleY(1) scaleX(1);
      opacity: 1;
    }
  }

  .download-text h3 {
    font-family: var(--font-display);
    font-style: italic;
    font-weight: 300;
    font-size: 24px;
  }

  .download-text p {
    color: var(--text-muted);
    font-size: 12px;
    margin-top: 4px;
  }

  .progress-wrap {
    width: min(420px, 100%);
    display: grid;
    gap: 6px;
  }

  .progress-meta {
    display: flex;
    justify-content: space-between;
    color: var(--text-muted);
    font-size: 11px;
  }

  @media (max-width: 680px) {
    .core-grid {
      grid-template-columns: 1fr;
    }

    .form-grid {
      grid-template-columns: 1fr;
    }

    .span-2 {
      grid-column: auto;
    }

    .toggles-inline {
      grid-template-columns: 1fr;
    }
  }
</style>
