<script lang="ts">
  import { tick } from "svelte";
  import { ansiToHtml } from "$lib/utils/ansi";
  import {
    attachConsole,
    clearConsole,
    getCommandHistory,
    getServerById,
    getServerCommands,
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
  let showCompletions = $state(false);
  let lastTabTime = $state(0);
  let completionStartPos = $state(0);
  let commandSuggestions = $state<string[]>([]);
  let selectedCompletionIndex = $state(-1);

  const selectedServer = $derived(getServerById(serverId));
  const lines = $derived((serverId ? serverState.consoleLines[serverId] : []) ?? []);
  const isAutoScrollEnabled = $derived((serverId ? serverState.autoScrollByServer[serverId] : true) ?? true);

  async function loadServerCommands(): Promise<void> {
    if (!serverId) {
      commandSuggestions = [];
      return;
    }
    
    try {
      const commands = await getServerCommands(serverId);
      commandSuggestions = commands;
    } catch (error) {
      console.error("Failed to load server commands:", error);
      // Fallback к базовым командам
      commandSuggestions = [
        "help",
        "list",
        "stop", 
        "save-all",
        "reload",
        "restart"
      ];
    }
  }

  async function submitCommand(): Promise<void> {
    if (!serverId || !commandInput.trim()) {
      return;
    }
    await sendServerCommand(serverId, commandInput.trim());
    commandInput = "";
    historyCursor = null;
    tabCandidates = [];
    tabPrefix = "";
    showCompletions = false;
    selectedCompletionIndex = -1;
  }

  function handleScroll(): void {
    if (!serverId || !scroller) {
      return;
    }
    const offset = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight;
    setAutoScroll(serverId, offset < 24);
  }

  function applyTabCompletion(): void {
    const currentTime = Date.now();
    const isDoubleTab = currentTime - lastTabTime < 500; // 500ms для двойного Tab
    
    const value = commandInput.trim();
    if (!value) {
      lastTabTime = currentTime;
      return;
    }

    // Находим позицию для автодополнения (последнее слово)
    const words = value.split(/\s+/);
    const currentWord = words[words.length - 1];
    const beforeCurrentWord = value.substring(0, value.lastIndexOf(currentWord));
    
    // Нормализуем поисковый термин (убираем "/" если есть)
    const searchTerm = currentWord.startsWith('/') ? currentWord.slice(1) : currentWord;
    
    // Ищем подходящие команды
    const matches = commandSuggestions.filter((cmd) => 
      cmd.toLowerCase().startsWith(searchTerm.toLowerCase())
    );

    if (matches.length === 0) {
      showCompletions = false;
      lastTabTime = currentTime;
      return;
    }

    if (matches.length === 1) {
      // Единственное совпадение - автоматически дополняем
      const completion = matches[0];
      commandInput = beforeCurrentWord + completion;
      showCompletions = false;
      tabCandidates = [];
    } else {
      // Несколько совпадений
      if (!isDoubleTab && !showCompletions) {
        // Первый Tab - дополняем общую часть
        const commonPrefix = findCommonPrefix(matches);
        if (commonPrefix.length > searchTerm.length) {
          commandInput = beforeCurrentWord + commonPrefix;
          showCompletions = false;
        } else {
          // Общей части нет больше - показываем варианты
          showCompletions = true;
          tabCandidates = matches;
          selectedCompletionIndex = -1;
        }
      } else {
        // Второй Tab или уже показаны варианты - показываем/обновляем все варианты
        showCompletions = true;
        tabCandidates = matches;
        selectedCompletionIndex = -1; // Сбрасываем выбор при показе списка
      }
    }
    
    lastTabTime = currentTime;
  }

  function findCommonPrefix(strings: string[]): string {
    if (strings.length === 0) return "";
    if (strings.length === 1) return strings[0];
    
    let prefix = strings[0];
    for (let i = 1; i < strings.length; i++) {
      while (strings[i].toLowerCase().indexOf(prefix.toLowerCase()) !== 0) {
        prefix = prefix.substring(0, prefix.length - 1);
        if (prefix === "") return "";
      }
    }
    return prefix;
  }

  function handleCommandInput(): void {
    // Показываем автодополнение при вводе
    const value = commandInput.trim();
    if (value.length > 0) {
      const words = value.split(/\s+/);
      const currentWord = words[words.length - 1];
      
      // Нормализуем поисковый термин (убираем "/" если есть)
      const searchTerm = currentWord.startsWith('/') ? currentWord.slice(1) : currentWord;
      
      // Ищем команды
      const matches = commandSuggestions.filter((cmd) => 
        cmd.toLowerCase().startsWith(searchTerm.toLowerCase())
      );
      
      if (matches.length > 0 && matches.length <= 8) {
        tabCandidates = matches;
        showCompletions = true;
        selectedCompletionIndex = -1; // Сбрасываем выбор при автоматическом показе
      } else {
        showCompletions = false;
      }
    } else {
      showCompletions = false;
    }
  }

  function selectCompletion(completion: string): void {
    const value = commandInput.trim();
    const words = value.split(/\s+/);
    const currentWord = words[words.length - 1];
    const beforeCurrentWord = value.substring(0, value.lastIndexOf(currentWord));
    
    commandInput = beforeCurrentWord + completion;
    showCompletions = false;
    tabCandidates = [];
    selectedCompletionIndex = -1;
    lastTabTime = 0;
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

    if (event.key === "Escape") {
      showCompletions = false;
      tabCandidates = [];
      selectedCompletionIndex = -1;
      lastTabTime = 0;
      return;
    }

    if (event.key === "Enter" && showCompletions && selectedCompletionIndex >= 0) {
      // Выбираем выделенную команду
      event.preventDefault();
      selectCompletion(tabCandidates[selectedCompletionIndex]);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (showCompletions && tabCandidates.length > 0) {
        // Навигация по подсказкам
        selectedCompletionIndex = selectedCompletionIndex <= 0 
          ? tabCandidates.length - 1 
          : selectedCompletionIndex - 1;
      } else {
        // Навигация по истории команд
        handleHistoryNavigation("up");
      }
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (showCompletions && tabCandidates.length > 0) {
        // Навигация по подсказкам
        selectedCompletionIndex = selectedCompletionIndex >= tabCandidates.length - 1 
          ? 0 
          : selectedCompletionIndex + 1;
      } else {
        // Навигация по истории команд
        handleHistoryNavigation("down");
      }
      return;
    }

    // Скрываем автодополнение при вводе
    showCompletions = false;
    tabCandidates = [];
    tabPrefix = "";
    selectedCompletionIndex = -1;
    lastTabTime = 0;
  }

  $effect(() => {
    if (!serverId) {
      return;
    }
    void attachConsole(serverId);
    void loadServerCommands(); // Загружаем команды при смене сервера
    historyCursor = null;
    tabCandidates = [];
    tabPrefix = "";
    showCompletions = false;
    selectedCompletionIndex = -1;
  });

  // Перезагружаем команды при изменении состояния сервера (запуск/остановка)
  $effect(() => {
    if (selectedServer?.running !== undefined) {
      void loadServerCommands();
    }
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
      <div class="command-input-container">
        <input
          class="input command-input"
          bind:value={commandInput}
          placeholder="Введите команду..."
          onkeydown={handleCommandKeydown}
          oninput={handleCommandInput}
        />
        {#if showCompletions && tabCandidates.length > 0}
          <div class="completions-popup">
            <div class="completions-header">
              Доступные команды ({tabCandidates.length}):
            </div>
            <div class="completions-grid">
              {#each tabCandidates as completion, index}
                <button
                  type="button"
                  class="completion-item"
                  class:completion-item-selected={index === selectedCompletionIndex}
                  onclick={() => selectCompletion(completion)}
                >
                  {completion}
                </button>
              {/each}
            </div>
            <div class="completions-hint">
              Нажмите Tab еще раз или кликните для выбора • Esc для закрытия
            </div>
          </div>
        {/if}
      </div>
      <button type="submit" class="btn btn-primary">Enter</button>
    </form>
    <div class="command-hint">Tab: автодополнение • Двойной Tab: показать все варианты • ↑/↓: навигация по подсказкам/истории • Enter: выбрать • Esc: закрыть</div>
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
    position: relative;
  }

  .command-input-container {
    position: relative;
  }

  .completions-popup {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    background: var(--panel-bg);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    margin-bottom: 8px;
    max-height: 300px;
    overflow: hidden;
    z-index: 1000;
  }

  .completions-header {
    padding: 8px 12px;
    background: var(--bg-subtle);
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted);
  }

  .completions-grid {
    max-height: 200px;
    overflow-y: auto;
    padding: 4px;
  }

  .completion-item {
    display: block;
    width: 100%;
    padding: 6px 12px;
    text-align: left;
    background: none;
    border: none;
    border-radius: var(--r-sm);
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    transition: background-color 0.1s ease;
  }

  .completion-item:hover {
    background: var(--bg-subtle);
  }

  .completion-item:active {
    background: var(--bg-active);
  }

  .completion-item-selected {
    background: var(--accent-bg) !important;
    color: var(--accent) !important;
    border: 1px solid var(--accent);
  }

  .completions-hint {
    padding: 6px 12px;
    background: var(--bg-subtle);
    border-top: 1px solid var(--border);
    font-size: 10px;
    color: var(--text-hint);
  }

  .command-hint {
    color: var(--text-hint);
    font-size: 11px;
  }
</style>
