<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldDescription, FieldLabel } from "@ui-sdk/svelte/field";
  import { NumberInput, NumberInputControl, NumberInputGroup } from "@ui-sdk/svelte/number-input";
  import { toast } from "@ui-sdk/svelte/toast";
  import NumberField from "../components/NumberField.svelte";
  import { api, errorMessage } from "../api";
  import { poundsToPence } from "../format";
  import type { SuiteRates } from "../bindings/SuiteRates";

  interface Props {
    suiteId: string;
    suiteName: string;
  }

  let { suiteId, suiteName }: Props = $props();
  let rates = $state<SuiteRates | null>(null);

  $effect(() => {
    const id = suiteId;
    api
      .getSuiteRates(id)
      .then((r) => (rates = r))
      .catch(() => (rates = null));
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!rates) return;
    try {
      await api.saveSuiteRates(suiteId, $state.snapshot(rates));
      toast.success(`Saved the rates for ${suiteName}`);
    } catch (error) {
      toast.error("Couldn't save the rates", { description: errorMessage(error) });
    }
  }
</script>

{#snippet price(label: string, key: "hourly" | "half_day" | "full_day")}
  {#if rates}
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <NumberInput
        value={rates[key] === null ? null : rates[key] / 100}
        onValueChange={(v) => rates && (rates[key] = v === null ? null : poundsToPence(v))}
        min={0}
        step={0.01}
        formatOptions={{ style: "currency", currency: "GBP" }}
      >
        <NumberInputGroup><NumberInputControl placeholder="Not offered" /></NumberInputGroup>
      </NumberInput>
    </Field>
  {/if}
{/snippet}

{#if rates}
  <form class="stack rates" onsubmit={save} aria-labelledby="rates-heading">
    <h3 id="rates-heading">Hire rates</h3>
    <p class="muted">Leave a price empty if the suite isn't hired that way.</p>
    <div class="row align-top">
      {@render price("Per hour", "hourly")}
      <NumberField label="Minimum hours" bind:value={rates.min_hours} min={0} max={24} />
    </div>
    <div class="row align-top">
      {@render price("Half day", "half_day")}
      {@render price("Full day", "full_day")}
    </div>
    <div class="row align-top">
      <NumberField label="Setup (minutes)" bind:value={rates.setup_minutes} min={0} max={1439} />
      <NumberField label="Clear-down (minutes)" bind:value={rates.cleardown_minutes} min={0} max={1439} />
    </div>
    <Field>
      <FieldDescription>Setup and clear-down are the defaults for new bookings; nothing else can be booked then.</FieldDescription>
    </Field>
    <div><Button type="submit" data-variant="solid">Save rates</Button></div>
  </form>
{/if}

<style>
  .rates {
    margin-top: calc(var(--ui-spacing) * 5);
    padding-top: calc(var(--ui-spacing) * 4);
    border-top: 1px solid var(--ui-border);
  }

  .rates h3,
  .rates p {
    margin: 0;
  }
</style>
