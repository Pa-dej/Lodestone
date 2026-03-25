<script lang="ts">
  import { onMount } from "svelte";

  interface SelectOption {
    value: string;
    label: string;
    disabled?: boolean;
  }

  interface Props {
    value: string;
    options: SelectOption[];
    disabled?: boolean;
    placeholder?: string;
    ariaLabel?: string;
    onChange?: (value: string) => void;
  }

  let {
    value,
    options,
    disabled = false,
    placeholder = "Select option",
    ariaLabel = "Select option",
    onChange = () => {},
  }: Props = $props();

  let open = $state(false);
  let root = $state<HTMLElement | null>(null);

  const selectedLabel = $derived.by(() => {
    const selected = options.find((option) => option.value === value);
    return selected?.label ?? placeholder;
  });

  function close(): void {
    open = false;
  }

  function toggle(): void {
    if (disabled) {
      return;
    }
    open = !open;
  }

  function select(option: SelectOption): void {
    if (disabled || option.disabled) {
      return;
    }
    onChange(option.value);
    open = false;
  }

  onMount(() => {
    const handleDocumentClick = (event: MouseEvent): void => {
      const target = event.target as Node | null;
      if (open && root && target && !root.contains(target)) {
        close();
      }
    };

    const handleEscape = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        close();
      }
    };

    window.addEventListener("click", handleDocumentClick);
    window.addEventListener("keydown", handleEscape);
    return () => {
      window.removeEventListener("click", handleDocumentClick);
      window.removeEventListener("keydown", handleEscape);
    };
  });
</script>

<div class="custom-select" class:open class:disabled bind:this={root}>
  <button
    type="button"
    class="custom-select-trigger"
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label={ariaLabel}
    disabled={disabled}
    onclick={toggle}
  >
    <span>{selectedLabel}</span>
    <span class="custom-select-arrow" aria-hidden="true">
      <svg viewBox="0 0 10 8" fill="none">
        <path d="M1 2L5 6L9 2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
      </svg>
    </span>
  </button>

  <div class="custom-select-dropdown" role="listbox">
    {#each options as option (option.value)}
      <button
        type="button"
        class="custom-select-option"
        class:selected={option.value === value}
        disabled={option.disabled}
        onclick={() => select(option)}
      >
        {option.label}
      </button>
    {/each}
  </div>
</div>

<style>
  .custom-select {
    position: relative;
  }

  .custom-select.disabled {
    opacity: 0.6;
    pointer-events: none;
  }

  .custom-select-trigger {
    background: var(--surface);
    border: 0.5px solid var(--border);
    border-radius: var(--r-md);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 13px;
    padding: 9px 32px 9px 12px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: space-between;
    user-select: none;
    transition: border-color 0.15s ease, background 0.15s ease;
    position: relative;
    width: 100%;
    text-align: left;
  }

  .custom-select-trigger:hover {
    border-color: var(--text-hint);
  }

  .custom-select.open .custom-select-trigger {
    border-color: var(--accent);
    background: var(--surface-2);
  }

  .custom-select-arrow {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    pointer-events: none;
    display: flex;
    align-items: center;
    transition: transform 0.2s cubic-bezier(0.34, 1.2, 0.64, 1), color 0.15s ease;
  }

  .custom-select-arrow :global(svg) {
    width: 10px;
    height: 8px;
    display: block;
  }

  .custom-select.open .custom-select-arrow {
    transform: translateY(-50%) rotate(180deg);
    color: var(--accent);
  }

  .custom-select-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    background: var(--surface);
    border: 0.5px solid var(--border);
    border-radius: var(--r-md);
    overflow: hidden;
    z-index: 210;
    opacity: 0;
    transform: translateY(-6px) scaleY(0.92);
    transform-origin: top center;
    pointer-events: none;
    transition: opacity 0.18s ease, transform 0.2s cubic-bezier(0.34, 1.2, 0.64, 1);
    max-height: 220px;
    overflow-y: auto;
  }

  .custom-select.open .custom-select-dropdown {
    opacity: 1;
    transform: translateY(0) scaleY(1);
    pointer-events: auto;
  }

  .custom-select-option {
    width: 100%;
    padding: 9px 12px;
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--text-muted);
    cursor: pointer;
    transition: background 0.12s ease, color 0.12s ease;
    display: flex;
    align-items: center;
    gap: 8px;
    text-align: left;
    border: none;
    background: transparent;
  }

  .custom-select-option:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text);
  }

  .custom-select-option.selected {
    color: var(--accent);
    background: var(--accent-bg);
  }

  .custom-select-option.selected::before {
    content: "";
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--accent);
    flex-shrink: 0;
  }

  .custom-select-option:not(.selected)::before {
    content: "";
    width: 4px;
    height: 4px;
    flex-shrink: 0;
  }

  .custom-select-option:disabled {
    opacity: 0.55;
    pointer-events: none;
  }
</style>
