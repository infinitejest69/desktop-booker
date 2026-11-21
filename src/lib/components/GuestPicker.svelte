<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import TextField from "./TextField.svelte";
  import { api } from "../api";
  import type { Guest } from "../bindings/Guest";

  interface Props {
    /** The guest being edited; picking a match replaces it. */
    guest: Guest;
    searchLabel?: string;
  }

  let { guest = $bindable(), searchLabel = "Find an existing guest" }: Props = $props();
  let query = $state("");
  let results = $state<Guest[]>([]);

  async function find() {
    const q = query.trim();
    results = q.length < 2 ? [] : await api.searchGuests(q).catch(() => []);
  }

  function pick(g: Guest) {
    guest = structuredClone($state.snapshot(g));
    results = [];
    query = "";
  }
</script>

<div class="stack">
  <div>
    <Field>
      <FieldLabel>{searchLabel}</FieldLabel>
      <Input type="search" bind:value={query} oninput={find} placeholder="Name, email or phone" />
    </Field>
    {#if results.length}
      <ul class="results" aria-label="Matching guests">
        {#each results as g (g.id)}
          <li>
            <Button onclick={() => pick(g)}>
              {g.first_name} {g.last_name}{g.email ? ` (${g.email})` : ""}
            </Button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
  <div class="row">
    <TextField label="First name" bind:value={guest.first_name} />
    <TextField label="Last name" bind:value={guest.last_name} required />
  </div>
  <div class="row">
    <TextField label="Email" type="email" bind:value={guest.email} />
    <TextField label="Phone" type="tel" bind:value={guest.phone} />
  </div>
</div>

<style>
  .results {
    list-style: none;
    margin: calc(var(--ui-spacing) * 2) 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: calc(var(--ui-spacing) * 2);
  }
</style>
