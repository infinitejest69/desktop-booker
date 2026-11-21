<script lang="ts">
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import { api } from "#lib/api.ts";
  import type { Guest } from "#lib/bindings/Guest.ts";

  let query = $state("");
  let results = $state<Guest[]>([]);

  $effect(() => {
    const q = query.trim();
    if (q.length < 2) {
      results = [];
      return;
    }
    api
      .searchGuests(q)
      .then((r) => {
        if (q === query.trim()) results = r;
      })
      .catch(() => (results = []));
  });
</script>

<div class="page">
  <h1>Guests</h1>
  <p class="page-intro">Find a guest to see their stays and spend, or to erase their details on request.</p>
  <Field>
    <FieldLabel>Search</FieldLabel>
    <Input type="search" bind:value={query} placeholder="Name, email or phone" />
  </Field>
  {#if results.length}
    <table class="list">
      <thead><tr><th>Name</th><th>Email</th><th>Phone</th></tr></thead>
      <tbody>
        {#each results as g (g.id)}
          <tr>
            <td><a href={`/guests/${g.id}`}>{`${g.first_name} ${g.last_name}`.trim()}</a></td>
            <td>{g.email}</td>
            <td>{g.phone}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if query.trim().length >= 2}
    <p class="muted">No guests match.</p>
  {/if}
</div>
