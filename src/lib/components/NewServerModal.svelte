<script lang="ts">
  import Toggle from "$lib/components/Toggle.svelte";
  import CustomSelect from "$lib/components/CustomSelect.svelte";
  import { createServer, fetchVersions, serverState } from "$lib/stores/servers.svelte";
  import { i18nState, t } from "$lib/stores/i18n.svelte";
  import type { CoreType, NewServerConfig, ServerConfig } from "$lib/types";
  import PaperIcon from "../../icons/servers/Paper.svg?raw";
  import PurpurIcon from "../../icons/servers/Purpur.svg?raw";
  import FabricIcon from "../../icons/servers/Fabric.svg?raw";
  import ForgeIcon from "../../icons/servers/Forge.svg?raw";
  import FoliaIcon from "../../icons/servers/Folia.svg?raw";
  import VanillaIcon from "../../icons/Server.svg?raw";

  interface CoreOption {
    id: CoreType;
    iconSvg: string;
    color: string;
    name: string;
    description: string;
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
      iconSvg: PurpurIcon,
      color: "var(--core-purpur)",
      name: "Purpur",
      description:
        i18nState.language === "ru" ? "Форк Paper, больше настроек" : "Paper fork with extra settings",
    },
    {
      id: "fabric",
      iconSvg: FabricIcon,
      color: "var(--core-fabric)",
      name: "Fabric",
      description: i18nState.language === "ru" ? "Лёгкий загрузчик модов" : "Lightweight mod loader",
    },
    {
      id: "forge",
      iconSvg: ForgeIcon,
      color: "var(--core-forge)",
      name: "Forge",
      description: i18nState.language === "ru" ? "Классический загрузчик модов" : "Classic mod loader",
    },
    {
      id: "folia",
      iconSvg: FoliaIcon,
      color: "var(--core-folia)",
      name: "Folia",
      description: i18nState.language === "ru" ? "Многопоточный форк Paper" : "Multithreaded Paper fork",
    },
    {
      id: "vanilla",
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
  let port = $state(25565);
  let ramMb = $state(2048);
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

  const versionOptions = $derived.by(() => {
    if (loadingVersions) {
      return [{ value: "", label: t("modal_loading_versions"), disabled: true }];
    }
    if (versions.length === 0) {
      return [{ value: "", label: t("modal_no_versions"), disabled: true }];
    }
    return versions.map((version) => ({ value: version, label: version }));
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

  async function loadVersionsForCore(core: CoreType): Promise<void> {
    loadingVersions = true;
    modalError = null;
    versions = [];
    selectedVersion = "";
    try {
      const result = await fetchVersions(core);
      versions = result;
      selectedVersion = result[0] ?? "";
      if (!selectedVersion) {
        modalError = t("modal_no_versions");
      }
    } finally {
      loadingVersions = false;
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
    port = 25565;
    ramMb = 2048;
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
  }

  async function submitCreation(): Promise<void> {
    if (!selectedVersion) {
      modalError = t("modal_select_version_error");
      return;
    }

    modalError = null;
    failed = false;
    creationStarted = true;
    currentStep = 2;

    const payload: NewServerConfig = {
      name: serverName.trim(),
      core: selectedCore,
      version: selectedVersion,
      port,
      ram_mb: ramMb,
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
          {#each coreOptions as option}
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

          <div class="field">
            <span class="field-label">{t("field_version")}</span>
            <CustomSelect
              value={selectedVersion}
              options={versionOptions}
              disabled={loadingVersions || versions.length === 0}
              onChange={(value) => {
                selectedVersion = value;
              }}
            />
          </div>

          <label class="field">
            <span class="field-label">{t("field_port")}</span>
            <input class="input" type="number" min={1} max={65535} bind:value={port} />
          </label>

          <div class="field">
            <span class="field-label">{t("field_ram_mb")}</span>
            <div class="slider-row">
              <input class="range-input" type="range" min={512} max={16384} step={256} bind:value={ramMb} />
              <span class="tag">{ramMb} MB</span>
            </div>
          </div>

          <label class="field span-2">
            <span class="field-label">{t("field_motd")}</span>
            <input class="input" bind:value={motd} placeholder="A Lodestone Minecraft Server" />
          </label>

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
            {t("modal_create")}
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
              ⭳
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
    gap: 10px;
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
