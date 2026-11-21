<script lang="ts">
  import { onMount } from "svelte";
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldDescription, FieldLabel } from "@ui-sdk/svelte/field";
  import { NumberInput, NumberInputControl, NumberInputGroup } from "@ui-sdk/svelte/number-input";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectItemIndicator,
    SelectTrigger,
    SelectValue,
  } from "@ui-sdk/svelte/select";
  import { Switch, SwitchThumb } from "@ui-sdk/svelte/switch";
  import { Textarea } from "@ui-sdk/svelte/textarea";
  import { toast } from "@ui-sdk/svelte/toast";
  import TextField from "#lib/components/TextField.svelte";
  import { api, errorMessage, newId } from "#lib/api.ts";
  import type { ChargeCategory } from "#lib/bindings/ChargeCategory.ts";
  import type { InvoiceSettings } from "#lib/bindings/InvoiceSettings.ts";
  import type { TaxRate } from "#lib/bindings/TaxRate.ts";
  import { categoryLabel, percent } from "#lib/invoices/labels.ts";

  type RateKey =
    | "accommodation_rate"
    | "board_rate"
    | "suite_rate"
    | "food_rate"
    | "drink_rate"
    | "other_rate";

  const NONE = "none";
  let rates = $state<TaxRate[]>([]);
  let settings = $state<InvoiceSettings | null>(null);
  let newName = $state("");
  let newPercent = $state<number | null>(20);

  const categories: { key: RateKey; category: ChargeCategory }[] = [
    { key: "accommodation_rate", category: "accommodation" },
    { key: "board_rate", category: "board" },
    { key: "suite_rate", category: "suite" },
    { key: "food_rate", category: "food" },
    { key: "drink_rate", category: "drink" },
    { key: "other_rate", category: "other" },
  ];
  const rateItems = $derived([
    { value: NONE, label: "No tax" },
    ...rates.map((r) => ({ value: r.id, label: `${r.name} (${percent(r.basis_points)})` })),
  ]);

  async function load() {
    try {
      [rates, settings] = await Promise.all([api.listTaxRates(), api.getInvoiceSettings()]);
    } catch (error) {
      toast.error("Couldn't load invoice settings", { description: errorMessage(error) });
    }
  }
  onMount(load);

  async function addRate(event: SubmitEvent) {
    event.preventDefault();
    if (newPercent === null) return;
    try {
      await api.saveTaxRate({ id: newId(), name: newName, basis_points: Math.round(newPercent * 100) });
      toast.success(`Added ${newName}`);
      newName = "";
      rates = await api.listTaxRates();
    } catch (error) {
      toast.error("Couldn't add the tax rate", { description: errorMessage(error) });
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!settings) return;
    try {
      await api.saveInvoiceSettings($state.snapshot(settings));
      toast.success("Saved the invoice settings");
    } catch (error) {
      toast.error("Couldn't save", { description: errorMessage(error) });
    }
  }
</script>

<div class="page">
  <h1>Invoices and tax</h1>
  <p class="page-intro">The tax rates you charge, which applies to what, and what's printed on invoices.</p>

  <section class="panel stack" aria-labelledby="rates-heading">
    <h2 id="rates-heading">Tax rates</h2>
    {#if rates.length}
      <table class="list">
        <thead><tr><th>Name</th><th>Rate</th></tr></thead>
        <tbody>
          {#each rates as r (r.id)}
            <tr><td>{r.name}</td><td>{percent(r.basis_points)}</td></tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <p class="muted">No tax rates yet. Add your standard VAT rate first.</p>
    {/if}
    <form class="row align-top" onsubmit={addRate} aria-label="Add a tax rate">
      <TextField label="Name" bind:value={newName} required placeholder="Standard VAT" />
      <Field>
        <FieldLabel>Rate (%)</FieldLabel>
        <NumberInput bind:value={newPercent} min={0} max={100} step={0.01}>
          <NumberInputGroup><NumberInputControl /></NumberInputGroup>
        </NumberInput>
      </Field>
      <div class="add"><Button type="submit">Add tax rate</Button></div>
    </form>
  </section>

  {#if settings}
    <form class="panel stack settings" onsubmit={save} aria-labelledby="settings-heading">
      <h2 id="settings-heading">Invoices</h2>
      <label data-slot="label" class="switch-row">
        Prices include tax
        <Switch bind:checked={settings.prices_include_tax}><SwitchThumb /></Switch>
      </label>
      <p class="muted" style="margin: 0">
        {settings.prices_include_tax
          ? "A £120 room at 20% is £100 plus £20 VAT."
          : "A £120 room at 20% is charged £144 with VAT added."}
      </p>
      <fieldset class="stack">
        <legend>Tax charged on</legend>
        <div class="grid">
          {#each categories as c (c.key)}
            <Field>
              <FieldLabel>{categoryLabel[c.category]}</FieldLabel>
              <Select
                items={rateItems}
                value={settings[c.key] ?? NONE}
                onValueChange={(v) => {
                  if (settings) settings[c.key] = !v || v === NONE ? null : v;
                }}
              >
                <SelectTrigger><SelectValue /></SelectTrigger>
                <SelectContent>
                  {#each rateItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}<SelectItemIndicator /></SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </Field>
          {/each}
        </div>
      </fieldset>
      <TextField label="Invoice number prefix" bind:value={settings.prefix} description="e.g. INV- gives INV-000042" />
      <Field>
        <FieldLabel>Printed at the foot of invoices</FieldLabel>
        <Textarea
          bind:value={settings.footer}
          rows={4}
          placeholder={"VAT no. GB 123 4567 89\nBank: sort code 12-34-56, account 12345678"}
        />
        <FieldDescription>VAT number, bank details for transfers, a thank-you.</FieldDescription>
      </Field>
      <div><Button type="submit" data-variant="solid">Save invoice settings</Button></div>
    </form>
  {/if}
</div>

<style>
  .settings {
    margin-top: calc(var(--ui-spacing) * 4);
  }

  .add {
    align-self: flex-end;
  }

  .switch-row {
    display: flex;
    align-items: center;
    gap: calc(var(--ui-spacing) * 3);
    font-weight: 600;
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    font-weight: 600;
    margin-bottom: calc(var(--ui-spacing) * 2);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
    gap: calc(var(--ui-spacing) * 3);
  }
</style>
