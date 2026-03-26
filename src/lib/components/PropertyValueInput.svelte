<script lang="ts">
  import Toggle from "./Toggle.svelte";
  import CustomSelect from "./CustomSelect.svelte";

  interface Props {
    value: string;
    propertyKey: string;
    onChange: (value: string) => void;
  }

  let { value, propertyKey, onChange }: Props = $props();

  const valueType = $derived.by(() => {
    const key = propertyKey.toLowerCase();
    const val = value.toLowerCase().trim();

    if (key === "gamemode") return "gamemode";
    if (key === "difficulty") return "difficulty";

    if (
      val === "true" ||
      val === "false" ||
      key.includes("enable") ||
      key.includes("allow") ||
      key.includes("spawn") ||
      key.includes("online-mode") ||
      key.includes("pvp") ||
      key.includes("hardcore") ||
      key.includes("whitelist") ||
      key.includes("white-list") ||
      key.includes("flight") ||
      key.includes("nether") ||
      key.includes("command-block") ||
      key.includes("status") ||
      key.includes("monsters") ||
      key.includes("animals") ||
      key.includes("npcs") ||
      key.includes("enforce")
    ) {
      return "boolean";
    }

    if (
      key.includes("port") ||
      key.includes("max-") ||
      key.includes("distance") ||
      key.includes("protection") ||
      key.includes("timeout") ||
      key.includes("rate") ||
      key.includes("size") ||
      key.includes("limit") ||
      key.includes("count") ||
      key.includes("level") ||
      key.includes("players")
    ) {
      const parsed = Number.parseInt(val, 10);
      if (Number.isFinite(parsed) || val === "") {
        return "number";
      }
    }

    return "text";
  });

  const booleanValue = $derived(
    value.toLowerCase() === "true" || value === "1" || value.toLowerCase() === "yes",
  );

  const gamemodeOptions = [
    { value: "survival", label: "Survival" },
    { value: "creative", label: "Creative" },
    { value: "adventure", label: "Adventure" },
    { value: "spectator", label: "Spectator" },
  ];

  const difficultyOptions = [
    { value: "peaceful", label: "Peaceful" },
    { value: "easy", label: "Easy" },
    { value: "normal", label: "Normal" },
    { value: "hard", label: "Hard" },
  ];

  function handleBooleanToggle(checked: boolean): void {
    onChange(checked ? "true" : "false");
  }

  function handleNumberInput(event: Event): void {
    const input = event.currentTarget as HTMLInputElement;
    const numValue = Number.parseInt(input.value, 10);
    if (Number.isFinite(numValue)) {
      onChange(String(numValue));
    } else if (input.value === "") {
      onChange("");
    }
  }

  function handleTextInput(event: Event): void {
    const input = event.currentTarget as HTMLInputElement;
    onChange(input.value);
  }

  const numberConstraints = $derived.by(() => {
    const key = propertyKey.toLowerCase();
    if (key.includes("port")) return { min: 1, max: 65535 };
    if (key === "max-players") return { min: 1, max: 500 };
    if (key.includes("distance")) return { min: 2, max: 32 };
    if (key.includes("protection")) return { min: 0, max: 32 };
    if (key.includes("timeout")) return { min: 0, max: 60000 };
    return { min: 0, max: 999999 };
  });
</script>

{#if valueType === "gamemode"}
  <CustomSelect value={value} options={gamemodeOptions} onChange={onChange} />
{:else if valueType === "difficulty"}
  <CustomSelect value={value} options={difficultyOptions} onChange={onChange} />
{:else if valueType === "boolean"}
  <div class="property-widget">
    <Toggle checked={booleanValue} onToggle={handleBooleanToggle} />
  </div>
{:else if valueType === "number"}
  <input
    class="input"
    type="number"
    min={numberConstraints.min}
    max={numberConstraints.max}
    value={value}
    oninput={handleNumberInput}
  />
{:else}
  <input class="input" type="text" value={value} oninput={handleTextInput} />
{/if}

<style>
  .property-widget {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    min-height: 32px;
  }
</style>
