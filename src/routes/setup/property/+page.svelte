<script lang="ts">
  import { onMount } from "svelte";
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectItemIndicator,
    SelectTrigger,
    SelectValue,
  } from "@ui-sdk/svelte/select";
  import { Textarea } from "@ui-sdk/svelte/textarea";
  import { toast } from "@ui-sdk/svelte/toast";
  import NumberField from "#lib/components/NumberField.svelte";
  import TextField from "#lib/components/TextField.svelte";
  import { api, errorMessage } from "#lib/api.ts";
  import type { PropertySettings } from "#lib/bindings/PropertySettings.ts";

  const currencies = [
    { value: "GBP", label: "GBP — Pound sterling" },
    { value: "EUR", label: "EUR — Euro" },
    { value: "USD", label: "USD — US dollar" },
  ];

  let settings = $state<PropertySettings | null>(null);
  let currency = $state<string | null>(null);

  onMount(async () => {
    try {
      settings = await api.getSettings();
      currency = settings.currency;
    } catch (error) {
      toast.error("Couldn't load hotel details", { description: errorMessage(error) });
    }
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!settings) return;
    try {
      await api.saveSettings({ ...settings, currency: currency ?? settings.currency });
      toast.success("Hotel details saved");
    } catch (error) {
      toast.error("Couldn't save", { description: errorMessage(error) });
    }
  }
</script>

<div class="page">
  <h1>Hotel details</h1>
  <p class="page-intro">Shown on invoices and confirmation emails.</p>

  {#if settings}
    <form class="stack panel" style="max-width: 36rem" onsubmit={save}>
      <TextField label="Hotel name" bind:value={settings.hotel_name} required />
      <Field>
        <FieldLabel>Address</FieldLabel>
        <Textarea bind:value={settings.address} rows={3} />
      </Field>
      <div class="row">
        <TextField label="Phone" type="tel" bind:value={settings.phone} />
        <TextField label="Email" type="email" bind:value={settings.email} />
      </div>
      <div class="row">
        <Field>
          <FieldLabel>Currency</FieldLabel>
          <Select items={currencies} bind:value={currency}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {#each currencies as c (c.value)}
                <SelectItem value={c.value}>
                  {c.label}
                  <SelectItemIndicator />
                </SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </Field>
        <TextField
          label="Time zone"
          bind:value={settings.timezone}
          description="For example Europe/London"
          required
        />
      </div>
      <div class="row">
        <TextField
          label="Check-in from"
          type="time"
          bind:value={settings.check_in_time}
          required
        />
        <TextField label="Check-out by" type="time" bind:value={settings.check_out_time} required />
      </div>
      <NumberField
        label="Lock the app after (minutes idle)"
        bind:value={settings.idle_lock_minutes}
        min={0}
        max={240}
        description="0 means never lock. Whoever is at the desk then signs in with their PIN."
      />
      <div>
        <Button type="submit" data-variant="solid">Save details</Button>
      </div>
    </form>
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</div>
