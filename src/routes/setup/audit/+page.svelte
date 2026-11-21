<script lang="ts">
  import { onMount } from "svelte";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectItemIndicator,
    SelectTrigger,
    SelectValue,
  } from "@ui-sdk/svelte/select";
  import { toast } from "@ui-sdk/svelte/toast";
  import { api, errorMessage } from "#lib/api.ts";
  import type { AuditEntry } from "#lib/bindings/AuditEntry.ts";
  import type { User } from "#lib/bindings/User.ts";

  const ANY = "any";
  const kinds = [
    { value: ANY, label: "Everything" },
    { value: "booking", label: "Bookings" },
    { value: "rate_rule", label: "Rate rules" },
    { value: "board_option", label: "Board options" },
    { value: "room_type", label: "Room types" },
    { value: "space", label: "Rooms and suites" },
    { value: "floor", label: "Floors" },
    { value: "settings", label: "Hotel details" },
    { value: "user", label: "Users" },
  ];

  let users = $state<User[]>([]);
  let who = $state(ANY);
  let kind = $state(ANY);
  let entries = $state<AuditEntry[]>([]);
  let open = $state<string | null>(null);

  const people = $derived([{ value: ANY, label: "Anyone" }, ...users.map((u) => ({ value: u.id, label: u.name }))]);

  onMount(async () => {
    users = await api.listUsers().catch(() => []);
  });

  $effect(() => {
    const [u, k] = [who, kind];
    api
      .listAudit(u === ANY ? null : u, k === ANY ? null : k, 300)
      .then((e) => (entries = e))
      .catch((error) => toast.error("Couldn't load the audit log", { description: errorMessage(error) }));
  });

  function when(at: string): string {
    return new Date(at).toLocaleString("en-GB", {
      weekday: "short",
      day: "numeric",
      month: "short",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function pretty(json: string | null): string {
    if (!json) return "—";
    try {
      return JSON.stringify(JSON.parse(json), null, 2);
    } catch {
      return json;
    }
  }
</script>

<div class="page">
  <h1>Audit log</h1>
  <p class="page-intro">Every change, newest first, with who made it. Open a row to see what changed.</p>

  <div class="row filters">
    <Field>
      <FieldLabel>Person</FieldLabel>
      <Select items={people} value={who} onValueChange={(v) => v && (who = v)}>
        <SelectTrigger><SelectValue /></SelectTrigger>
        <SelectContent>
          {#each people as p (p.value)}
            <SelectItem value={p.value}>{p.label}<SelectItemIndicator /></SelectItem>
          {/each}
        </SelectContent>
      </Select>
    </Field>
    <Field>
      <FieldLabel>Kind of change</FieldLabel>
      <Select items={kinds} value={kind} onValueChange={(v) => v && (kind = v)}>
        <SelectTrigger><SelectValue /></SelectTrigger>
        <SelectContent>
          {#each kinds as k (k.value)}
            <SelectItem value={k.value}>{k.label}<SelectItemIndicator /></SelectItem>
          {/each}
        </SelectContent>
      </Select>
    </Field>
  </div>

  {#if entries.length}
    <table class="list panel-table">
      <thead><tr><th>When</th><th>Who</th><th>What happened</th><th><span class="muted">Details</span></th></tr></thead>
      <tbody>
        {#each entries as e (e.id)}
          <tr>
            <td class="nowrap">{when(e.at)}</td>
            <td>{e.user_name}</td>
            <td>{e.summary}</td>
            <td class="actions">
              <button type="button" class="link" aria-expanded={open === e.id} onclick={() => (open = open === e.id ? null : e.id)}>
                {open === e.id ? "Hide" : "Show"}
              </button>
            </td>
          </tr>
          {#if open === e.id}
            <tr class="detail">
              <td colspan="4">
                <div class="diff">
                  <div><h3>Before</h3><pre>{pretty(e.before)}</pre></div>
                  <div><h3>After</h3><pre>{pretty(e.after)}</pre></div>
                </div>
              </td>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="muted">No changes recorded yet.</p>
  {/if}
</div>

<style>
  .filters {
    margin-bottom: var(--app-gap);
  }

  .filters :global([data-slot="field"]) {
    flex: 0 1 16rem;
  }

  .panel-table {
    background: var(--paper);
  }

  .nowrap {
    white-space: nowrap;
  }

  .link {
    border: 0;
    background: none;
    color: var(--heather);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }

  .link:focus-visible {
    outline: 2px solid var(--heather);
    outline-offset: 2px;
  }

  .diff {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: calc(var(--ui-spacing) * 4);
  }

  .diff pre {
    max-height: 18rem;
    overflow: auto;
    margin: 0;
    padding: calc(var(--ui-spacing) * 3);
    background: var(--ground);
    border-radius: var(--ui-radius);
    font-size: 0.8rem;
  }

  .diff h3 {
    margin: 0 0 calc(var(--ui-spacing) * 1);
  }
</style>
