<script lang="ts">
  import { onMount } from "svelte";
  import { confirm } from "@ui-sdk/svelte/alert-dialog";
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectItemIndicator,
    SelectTrigger,
    SelectValue,
  } from "@ui-sdk/svelte/select";
  import { Switch, SwitchThumb } from "@ui-sdk/svelte/switch";
  import { toast } from "@ui-sdk/svelte/toast";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { api, errorMessage, newId } from "#lib/api.ts";
  import type { Dashboard } from "#lib/bindings/Dashboard.ts";
  import type { ReportResult } from "#lib/bindings/ReportResult.ts";
  import type { ReportSpec } from "#lib/bindings/ReportSpec.ts";
  import type { SavedReport } from "#lib/bindings/SavedReport.ts";
  import ViewSwitch from "#lib/nav/ViewSwitch.svelte";
  import { reportViews } from "#lib/nav/views.ts";
  import ReportChart from "#lib/reports/ReportChart.svelte";
  import { useSession } from "#lib/session/session.svelte.ts";

  const MAX_ITEMS = 12;
  const session = useSession();
  const allowed = $derived(session.can("view_reports"));

  let dashboards = $state<Dashboard[]>([]);
  let reports = $state<SavedReport[]>([]);
  let selected = $state<string | null>(page.url.searchParams.get("id"));
  /** A copy being edited, or null when just looking. */
  let draft = $state<Dashboard | null>(null);
  let adding = $state<string | null>(null);
  let results = $state(new Map<string, ReportResult | string>());
  let loaded = $state(false);

  const current = $derived(dashboards.find((d) => d.id === selected) ?? null);
  const reportById = $derived(new Map(reports.map((r) => [r.id, r])));
  const dashboardItems = $derived(dashboards.map((d) => ({ value: d.id, label: d.name })));
  const addable = $derived(
    reports
      .filter((r) => !draft?.items.some((i) => i.report_id === r.id))
      .map((r) => ({ value: r.id, label: r.spec.name })),
  );

  async function load() {
    try {
      [dashboards, reports] = await Promise.all([api.listDashboards(), api.listSavedReports()]);
      if (!dashboards.some((d) => d.id === selected)) selected = dashboards[0]?.id ?? null;
    } catch (error) {
      toast.error("Couldn't load dashboards", { description: errorMessage(error) });
    } finally {
      loaded = true;
    }
  }

  onMount(() => {
    if (allowed) void load();
  });

  // Run every report on the dashboard being looked at.
  $effect(() => {
    const board = current;
    if (!board || draft) return;
    for (const item of board.items) {
      const report = reportById.get(item.report_id);
      if (!report || results.has(report.id)) continue;
      api
        .runReport($state.snapshot(report.spec) as ReportSpec)
        .then((r) => (results = new Map(results).set(report.id, r)))
        .catch((error) => (results = new Map(results).set(report.id, errorMessage(error))));
    }
  });

  function choose(id: string | null) {
    if (!id) return;
    selected = id;
    draft = null;
    void goto(`/reports/dashboards?id=${id}`, { replace: true, reset: false });
  }

  function startNew() {
    draft = { id: newId(), name: "", items: [] };
  }

  function edit() {
    if (current) draft = structuredClone($state.snapshot(current)) as Dashboard;
  }

  function move(index: number, by: -1 | 1) {
    if (!draft) return;
    const items = [...draft.items];
    const [item] = items.splice(index, 1);
    items.splice(index + by, 0, item);
    draft.items = items;
  }

  function add() {
    if (!draft || !adding) return;
    draft.items = [...draft.items, { report_id: adding, wide: false }];
    adding = null;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!draft) return;
    try {
      const board = $state.snapshot(draft) as Dashboard;
      await api.saveDashboard(board);
      dashboards = await api.listDashboards();
      toast.success(`Saved "${board.name}"`);
      draft = null;
      choose(board.id);
    } catch (error) {
      toast.error("Couldn't save the dashboard", { description: errorMessage(error) });
    }
  }

  async function remove() {
    if (!current) return;
    const name = current.name;
    const yes = await confirm({
      title: `Delete "${name}"?`,
      description: "Its reports stay saved; only this arrangement goes.",
      confirmLabel: "Delete dashboard",
      cancelLabel: "Cancel",
    });
    if (!yes) return;
    try {
      await api.deleteDashboard(current.id);
      toast.success(`Deleted "${name}"`);
      draft = null;
      selected = null;
      await load();
    } catch (error) {
      toast.error("Couldn't delete the dashboard", { description: errorMessage(error) });
    }
  }
</script>

<div class="page wide">
  <h1>Dashboards</h1>
  {#if !allowed}
    <p class="muted">Reports are for admins. Ask an admin to sign in.</p>
  {:else}
    <div class="row toolbar">
      <ViewSwitch views={reportViews} current="Dashboards" label="Reports" />
      {#if dashboards.length && !draft}
        <Field>
          <FieldLabel>Dashboard</FieldLabel>
          <Select items={dashboardItems} value={selected} onValueChange={choose}>
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              {#each dashboardItems as d (d.value)}
                <SelectItem value={d.value}>{d.label}<SelectItemIndicator /></SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </Field>
      {/if}
      {#if !draft}
        <div class="row end">
          {#if current}<Button onclick={edit}>Edit dashboard</Button>{/if}
          <Button data-variant="solid" onclick={startNew}>New dashboard</Button>
        </div>
      {/if}
    </div>

    {#if draft}
      <form class="panel stack editor" aria-label="Edit dashboard" onsubmit={save}>
        <Field>
          <FieldLabel>Dashboard name</FieldLabel>
          <Input bind:value={draft.name} required placeholder="Monday meeting" />
        </Field>
        <h2 class="small-heading">Reports on it, in order</h2>
        {#if draft.items.length}
          <ol class="items">
            {#each draft.items as item, i (item.report_id)}
              {@const name = reportById.get(item.report_id)?.spec.name ?? "Deleted report"}
              <li>
                <span class="item-name">{name}</span>
                <label data-slot="label" class="wide-switch">
                  Full width
                  <Switch bind:checked={item.wide}><SwitchThumb /></Switch>
                </label>
                <Button aria-label={`Move ${name} up`} disabled={i === 0} onclick={() => move(i, -1)}>↑</Button>
                <Button aria-label={`Move ${name} down`} disabled={i === draft.items.length - 1} onclick={() => move(i, 1)}
                  >↓</Button
                >
                <Button aria-label={`Remove ${name}`} onclick={() => draft && (draft.items = draft.items.filter((_, j) => j !== i))}
                  >Remove</Button
                >
              </li>
            {/each}
          </ol>
        {:else}
          <p class="muted">No reports on it yet.</p>
        {/if}
        {#if reports.length === 0}
          <p class="muted">Save a report under <a href="/reports/build">Build a report</a> first.</p>
        {:else if draft.items.length < MAX_ITEMS}
          <div class="row add">
            <Field>
              <FieldLabel>Add a saved report</FieldLabel>
              <Select items={addable} bind:value={adding} placeholder="Choose a report">
                <SelectTrigger><SelectValue /></SelectTrigger>
                <SelectContent>
                  {#each addable as r (r.value)}
                    <SelectItem value={r.value}>{r.label}<SelectItemIndicator /></SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </Field>
            <Button onclick={add} disabled={!adding}>Add</Button>
          </div>
        {:else}
          <p class="muted">A dashboard holds up to {MAX_ITEMS} reports.</p>
        {/if}
        <div class="row">
          <Button type="submit" data-variant="solid">Save dashboard</Button>
          <Button onclick={() => (draft = null)}>Cancel</Button>
          {#if current && draft.id === current.id}
            <Button data-variant="destructive" onclick={remove}>Delete dashboard</Button>
          {/if}
        </div>
      </form>
    {:else if current}
      {#if current.items.length}
        <div class="board">
          {#each current.items as item (item.report_id)}
            {@const report = reportById.get(item.report_id)}
            {@const result = results.get(item.report_id)}
            <section class="panel card" class:wide-card={item.wide} aria-label={report?.spec.name ?? "Report"}>
              <div class="card-head">
                <h2>{report?.spec.name}</h2>
                <a class="link-button" href={`/reports/build?id=${item.report_id}`}>Open</a>
              </div>
              {#if typeof result === "string"}
                <p class="muted">{result}</p>
              {:else if result}
                <ReportChart {result} small />
              {:else}
                <p class="muted">Working it out…</p>
              {/if}
            </section>
          {/each}
        </div>
      {:else}
        <p class="muted">This dashboard is empty. Choose <strong>Edit dashboard</strong> to add reports.</p>
      {/if}
    {:else if loaded}
      <p class="muted">No dashboards yet. A dashboard puts saved reports side by side; choose New dashboard to make one.</p>
    {/if}
  {/if}
</div>

<style>
  .wide {
    max-width: 90rem;
  }

  .toolbar {
    align-items: flex-end;
    margin-bottom: calc(var(--ui-spacing) * 4);
  }

  .toolbar :global([data-slot="field"]) {
    flex: 0 1 16rem;
  }

  .end {
    margin-left: auto;
  }

  .editor {
    max-width: 48rem;
  }

  .small-heading {
    margin: calc(var(--ui-spacing) * 2) 0 0;
    font-size: 1rem;
  }

  .items {
    margin: 0;
    padding-left: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 2);
  }

  .items li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--ui-spacing) * 2);
  }

  .item-name {
    margin-right: auto;
    font-weight: 600;
  }

  .wide-switch {
    display: flex;
    align-items: center;
    gap: calc(var(--ui-spacing) * 2);
  }

  .add {
    align-items: flex-end;
  }

  .board {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: calc(var(--ui-spacing) * 4);
  }

  @media (max-width: 60rem) {
    .board {
      grid-template-columns: 1fr;
    }
  }

  .card {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 3);
  }

  .wide-card {
    grid-column: 1 / -1;
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 2);
  }

  .card-head h2 {
    margin: 0;
    font-size: 1.05rem;
  }
</style>
