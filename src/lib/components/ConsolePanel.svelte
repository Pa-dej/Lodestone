<script lang="ts">
  import { tick } from "svelte";
  import { ansiToHtml } from "$lib/utils/ansi";
  import {
    attachConsole,
    clearConsole,
    getCommandHistory,
    getServerById,
    sendServerCommand,
    serverState,
    setAutoScroll,
  } from "$lib/stores/servers.svelte";

  interface Props {
    serverId: string | null;
  }

  let { serverId }: Props = $props();

  let commandInput = $state("");
  let scroller = $state<HTMLDivElement | null>(null);
  let historyCursor = $state<number | null>(null);
  let tabCandidates = $state<string[]>([]);
  let tabCandidateIndex = $state(0);
  let tabPrefix = $state("");

  const commandSuggestions: string[] = [
    "/help",
    "/list",
    "/say",
    "/stop",
    "/save-all",
    "/save-on",
    "/save-off",
    "/time set day",
    "/time set night",
    "/weather clear",
    "/weather rain",
    "/gamerule keepInventory true",
    "/difficulty",
    "/gamemode",
    "/tp",
    "/whitelist on",
    "/whitelist off",
    "/whitelist add",
    "/ban",
    "/pardon",
    "/kick",
    "/op",
    "/deop",
  ];

  const selectedServer = $derived(getServerById(serverId));
  const lines = $derived((serverId ? serverState.consoleLines[serverId] : []) ?? []);
  const isAutoScrollEnabled = $derived((serverId ? serverState.autoScrollByServer[serverId] : true) ?? true);
  const filteredSuggestions = $derived.by(() => {
    const input = commandInput.trim().toLowerCase();
    if (!input) {
      return commandSuggestions.slice(0, 8);
    }
    return commandSuggestions.filter((entry) => entry.startsWith(input)).slice(0, 8);
  });

  async function submitCommand(): Promise<void> {
    if (!serverId || !commandInput.trim()) {
      return;
    }
    await sendServerCommand(serverId, commandInput.trim());
    commandInput = "";
    historyCursor = null;
    tabCandidates = [];
    tabPrefix = "";
  }

  function handleScroll(): void {
    if (!serverId || !scroller) {
      return;
    }
    const offset = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight;
    setAutoScroll(serverId, offset < 24);
  }

  function applyTabCompletion(): void {
    const value = commandInput.trim();
    if (!value) {
      return;
    }

    const prefix = value.toLowerCase();
    if (prefix !== tabPrefix) {
      tabCandidates = commandSuggestions.filter((entry) => entry.startsWith(prefix));
      tabPrefix = prefix;
      tabCandidateIndex = 0;
    }

    if (tabCandidates.length === 0) {
      return;
    }

    commandInput = tabCandidates[tabCandidateIndex];
    tabCandidateIndex = (tabCandidateIndex + 1) % tabCandidates.length;
  }

  function handleHistoryNavigation(direction: "up" | "down"): void {
    if (!serverId) {
      return;
    }
    const history = getCommandHistory(serverId);
    if (history.length === 0) {
      return;
    }

    if (direction === "up") {
      if (historyCursor === null) {
        historyCursor = history.length - 1;
      } else {
        historyCursor = Math.max(0, historyCursor - 1);
      }
      commandInput = history[historyCursor];
      return;
    }

    if (historyCursor === null) {
      return;
    }
    const nextIndex = historyCursor + 1;
    if (nextIndex >= history.length) {
      historyCursor = null;
      commandInput = "";
      return;
    }
    historyCursor = nextIndex;
    commandInput = history[nextIndex];
  }

  function handleCommandKeydown(event: KeyboardEvent): void {
    if (event.key === "Tab") {
      event.preventDefault();
      applyTabCompletion();
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      handleHistoryNavigation("up");
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      handleHistoryNavigation("down");
      return;
    }

    tabCandidates = [];
    tabPrefix = "";
  }

  $effect(() => {
    if (!serverId) {
      return;
    }
    void attachConsole(serverId);
    historyCursor = null;
    tabCandidates = [];
    tabPrefix = "";
  });

  $effect(() => {
    lines.length;
    if (!isAutoScrollEnabled || !scroller) {
      return;
    }

    void tick().then(() => {
      if (scroller) {
        scroller.scrollTop = scroller.scrollHeight;
      }
    });
  });
</script>

{#if !selectedServer}
  <section class="panel empty-panel">
    <p class="empty-text">Откройте вкладку сервера для просмотра консоли.</p>
  </section>
{:else}
  <section class="panel console-panel">
    <header class="console-header">
      <div class="console-meta">
        <div class="panel-title">{selectedServer.name}</div>
        <div class="console-sub">
          {selectedServer.core} · {selectedServer.version} · :{selectedServer.port}
        </div>
      </div>
      <div class="console-controls">
        <span class="badge badge-dot" class:badge-running={selectedServer.running}>
          {selectedServer.running ? "Running" : "Stopped"}
        </span>
        <button type="button" class="btn btn-ghost btn-sm" onclick={() => clearConsole(selectedServer.id)}>
          Clear
        </button>
      </div>
    </header>

    <div class="terminal" bind:this={scroller} onscroll={handleScroll}>
      {#if lines.length === 0}
        <div class="terminal-empty">Пока нет вывода консоли.</div>
      {:else}
        {#each lines as line (line.id)}
          <div class={`terminal-line level-${line.level.toLowerCase()}`}>
            <span class="line-ts">[{line.timestampLabel}]</span>
            <span class="line-tag">[{line.tag}/{line.level}]</span>
            <span class="line-message">{@html ansiToHtml(line.message)}</span>
            {#if line.repeats > 1}
              <span class="line-repeat">[x{line.repeats}]</span>
            {/if}
          </div>
        {/each}
      {/if}
    </div>

    <form
      class="command-row"
      onsubmit={(event) => {
        event.preventDefault();
        void submitCommand();
      }}
    >
      <datalist id="command-suggestions">
        {#each filteredSuggestions as suggestion}
          <option value={suggestion}></option>
        {/each}
      </datalist>
      <input
        class="input command-input"
        bind:value={commandInput}
        placeholder="Введите команду..."
        list="command-suggestions"
        onkeydown={handleCommandKeydown}
      />
      <button type="submit" class="btn btn-primary">Enter</button>
    </form>
    <div class="command-hint">Tab: автодополнение · ↑/↓: история команд</div>
  </section>
{/if}

<style>
  .console-panel {
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 10px;
    height: 100%;
    min-height: 420px;
  }

  .empty-panel {
    height: 100%;
    display: grid;
    place-items: center;
  }

  .empty-text {
    color: var(--text-muted);
  }

  .console-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .console-sub {
    font-size: 11px;
    color: var(--text-muted);
  }

  .console-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .badge-running {
    color: var(--success-color);
    border-color: var(--success-border);
    background: var(--success-bg);
  }

  .terminal {
    border-radius: var(--r-md);
    background: var(--console-bg);
    border: 0.5px solid var(--border);
    overflow: auto;
    padding: 12px;
    font-size: 12px;
    line-height: 1.45;
    font-family: var(--font-mono);
  }

  .terminal-empty {
    color: var(--text-hint);
  }

  .terminal-line {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    color: var(--text-muted);
    margin-bottom: 2px;
    word-break: break-word;
  }

  .line-ts {
    color: var(--text-hint);
  }

  .line-tag {
    color: var(--text-hint);
  }

  .line-message {
    flex: 1;
    min-width: 120px;
  }

  .line-repeat {
    color: var(--text-hint);
  }

  .level-info .line-message {
    color: var(--text-muted);
  }

  .level-success .line-message {
    color: var(--success-color);
  }

  .level-warn .line-message {
    color: var(--warn-color);
  }

  .level-error .line-message {
    color: var(--error-color);
  }

  .level-dim .line-message {
    color: var(--text-hint);
  }

  .command-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
  }

  .command-hint {
    color: var(--text-hint);
    font-size: 11px;
  }
</style>
