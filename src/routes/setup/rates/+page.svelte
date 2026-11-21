<script lang="ts">
  import { onMount } from "svelte";
  import { confirm } from "@ui-sdk/svelte/alert-dialog";
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldDescription, FieldLabel } from "@ui-sdk/svelte/field";
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
  import { Toggle, ToggleGroup } from "@ui-sdk/svelte/toggle";
  import MoneyField from "#lib/components/MoneyField.svelte";
  import NumberField from "#lib/components/NumberField.svelte";
  import TextField from "#lib/components/TextField.svelte";
  import { api, errorMessage, newId } from "#lib/api.ts";
  import { addDays, formatDay, today } from "#lib/dates.ts";
  import { formatMoney } from "#lib/format.ts";
  import type { Adjustment } from "#lib/bindings/Adjustment.ts";
  import type { BoardOption } from "#lib/bindings/BoardOption.ts";
  import type { RateRule } from "#lib/bindings/RateRule.ts";
  import type { RoomType } from "#lib/bindings/RoomType.ts";

  const weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const kinds = [
    { value: "set_price", label: "Set the price to" },
    { value: "add_amount", label: "Add or take off" },
    { value: "percent", label: "Change by percent" },
  ];
  const ALL_TYPES = "all";

  let roomTypes = $state<RoomType[]>([]);
  let rules = $state<RateRule[]>([]);
  let boards = $state<BoardOption[]>([]);
  let rule = $state<RateRule>(blankRule());
  let board = $state<BoardOption>(blankBoard());
  let days = $state<string[]>([]);
  let amount = $state(0);
  let percent = $state(0);

  const typeItems = $derived([
    { value: ALL_TYPES, label: "All room types" },
    ...roomTypes.map((t) => ({ value: t.id, label: t.name })),
  ]);
  const ruleIsNew = $derived(!rules.some((r) => r.id === rule.id));
  const boardIsNew = $derived(!boards.some((b) => b.id === board.id));

  function blankRule(): RateRule {
    const start = today();
    return {
      id: newId(),
      name: "",
      room_type_id: null,
      start,
      end: addDays(start, 90),
      weekdays: [],
      priority: 10,
      adjustment: { kind: "add_amount", amount: 0 },
      active: true,
    };
  }

  function blankBoard(): BoardOption {
    return { id: newId(), code: "", name: "", adult_price: 0, child_price: 0 };
  }

  function editRule(r: RateRule) {
    rule = structuredClone($state.snapshot(r));
    days = r.weekdays.map(String);
    amount = r.adjustment.kind === "percent" ? 0 : r.adjustment.amount;
    percent = r.adjustment.kind === "percent" ? r.adjustment.percent : 0;
  }

  function resetRule() {
    editRule(blankRule());
  }

  async function load() {
    try {
      [roomTypes, rules, boards] = await Promise.all([
        api.listRoomTypes(),
        api.listRateRules(),
        api.listBoardOptions(),
      ]);
    } catch (error) {
      toast.error("Couldn't load rates", { description: errorMessage(error) });
    }
  }

  onMount(load);

  function describe(a: Adjustment): string {
    switch (a.kind) {
      case "set_price":
        return `Price ${formatMoney(a.amount)}`;
      case "add_amount":
        return `${a.amount >= 0 ? "+" : "−"}${formatMoney(Math.abs(a.amount))}`;
      case "percent":
        return `${a.percent >= 0 ? "+" : "−"}${Math.abs(a.percent)}%`;
    }
  }

  function typeName(id: string | null): string {
    return id ? (roomTypes.find((t) => t.id === id)?.name ?? "?") : "All";
  }

  function dayNames(list: number[]): string {
    return list.length ? list.map((d) => weekdays[d - 1]).join(", ") : "Every day";
  }

  async function saveRule(event: SubmitEvent) {
    event.preventDefault();
    const adjustment: Adjustment =
      rule.adjustment.kind === "percent"
        ? { kind: "percent", percent }
        : { kind: rule.adjustment.kind, amount };
    const toSave = {
      ...$state.snapshot(rule),
      weekdays: days.map(Number).sort((a, b) => a - b),
      adjustment,
    };
    try {
      await api.saveRateRule(toSave);
      toast.success(ruleIsNew ? `Added ${rule.name}` : `Saved ${rule.name}`);
      await load();
      resetRule();
    } catch (error) {
      toast.error("Couldn't save rule", { description: errorMessage(error) });
    }
  }

  async function removeRule(r: RateRule) {
    const yes = await confirm({
      title: `Delete ${r.name}?`,
      description: "Existing bookings keep the prices they were booked at.",
      confirmLabel: "Delete rule",
      cancelLabel: "Cancel",
    });
    if (!yes) return;
    try {
      await api.deleteRateRule(r.id);
      toast.success(`Deleted ${r.name}`);
      await load();
      if (rule.id === r.id) resetRule();
    } catch (error) {
      toast.error("Couldn't delete rule", { description: errorMessage(error) });
    }
  }

  async function saveBoard(event: SubmitEvent) {
    event.preventDefault();
    try {
      await api.saveBoardOption($state.snapshot(board));
      toast.success(boardIsNew ? `Added ${board.name}` : `Saved ${board.name}`);
      await load();
      board = blankBoard();
    } catch (error) {
      toast.error("Couldn't save board option", { description: errorMessage(error) });
    }
  }

  async function removeBoard(b: BoardOption) {
    const yes = await confirm({
      title: `Delete ${b.name}?`,
      description: "Existing bookings keep their board and prices.",
      confirmLabel: "Delete",
      cancelLabel: "Cancel",
    });
    if (!yes) return;
    try {
      await api.deleteBoardOption(b.id);
      toast.success(`Deleted ${b.name}`);
      await load();
    } catch (error) {
      toast.error("Couldn't delete board option", { description: errorMessage(error) });
    }
  }
</script>

<div class="page">
  <h1>Rates</h1>
  <p class="page-intro">
    Each room type has a nightly rate (set on Room types). Rules below change it for seasons,
    weekends or offers. Lower priority numbers apply first.
  </p>

  <div class="split">
    <section aria-labelledby="rules-heading">
      <h2 id="rules-heading" class="muted" style="margin-bottom: 0.5rem">Rate rules</h2>
      {#if rules.length}
        <table class="list">
          <thead>
            <tr>
              <th>Name</th><th>Rooms</th><th>Dates</th><th>Days</th><th>Change</th><th>Priority</th>
              <th><span class="muted">Actions</span></th>
            </tr>
          </thead>
          <tbody>
            {#each rules as r (r.id)}
              <tr aria-current={r.id === rule.id ? "true" : undefined} class:inactive={!r.active}>
                <td>{r.name}{r.active ? "" : " (paused)"}</td>
                <td>{typeName(r.room_type_id)}</td>
                <td>{formatDay(r.start, { day: "numeric", month: "short" })} – {formatDay(r.end, { day: "numeric", month: "short", year: "numeric" })}</td>
                <td>{dayNames(r.weekdays)}</td>
                <td>{describe(r.adjustment)}</td>
                <td>{r.priority}</td>
                <td class="actions">
                  <Button onclick={() => editRule(r)}>Edit</Button>
                  <Button data-variant="destructive" onclick={() => removeRule(r)}>Delete</Button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="muted">No rules yet: every night uses the room type's rate.</p>
      {/if}
    </section>

    <form class="stack panel" onsubmit={saveRule} aria-labelledby="rule-form-heading">
      <h2 id="rule-form-heading">{ruleIsNew ? "Add a rule" : `Edit ${rule.name}`}</h2>
      <TextField label="Name" bind:value={rule.name} placeholder="Summer season" required />
      <Field>
        <FieldLabel>Applies to</FieldLabel>
        <Select
          items={typeItems}
          value={rule.room_type_id ?? ALL_TYPES}
          onValueChange={(v) => (rule.room_type_id = v === ALL_TYPES || v === null ? null : v)}
        >
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            {#each typeItems as t (t.value)}
              <SelectItem value={t.value}>{t.label}<SelectItemIndicator /></SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </Field>
      <div class="row">
        <Field>
          <FieldLabel>First night</FieldLabel>
          <Input type="date" bind:value={rule.start} required />
        </Field>
        <Field>
          <FieldLabel>Last night</FieldLabel>
          <Input type="date" bind:value={rule.end} required />
        </Field>
      </div>
      <Field>
        <FieldLabel>On these nights</FieldLabel>
        <ToggleGroup multiple aria-label="Nights of the week" bind:value={days} class="days">
          {#each weekdays as name, i (name)}
            <Toggle value={String(i + 1)}>{name}</Toggle>
          {/each}
        </ToggleGroup>
        <FieldDescription>None selected means every night.</FieldDescription>
      </Field>
      <Field>
        <FieldLabel>Change</FieldLabel>
        <Select
          items={kinds}
          value={rule.adjustment.kind}
          onValueChange={(v) => {
            if (v === "percent") rule.adjustment = { kind: "percent", percent };
            else if (v === "set_price" || v === "add_amount") rule.adjustment = { kind: v, amount };
          }}
        >
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            {#each kinds as k (k.value)}
              <SelectItem value={k.value}>{k.label}<SelectItemIndicator /></SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </Field>
      {#if rule.adjustment.kind === "percent"}
        <NumberField label="Percent" bind:value={percent} min={-100} max={500} description="Use a minus for a discount, e.g. -10" />
      {:else if rule.adjustment.kind === "set_price"}
        <MoneyField label="Price per night" bind:value={amount} />
      {:else}
        <MoneyField label="Amount per night" bind:value={amount} min={-10000} description="Use a minus for a discount" />
      {/if}
      <NumberField label="Priority" bind:value={rule.priority} min={-100} max={1000} />
      <label data-slot="label" class="switch-row">
        Active
        <Switch bind:checked={rule.active}><SwitchThumb /></Switch>
      </label>
      <div class="row">
        <Button type="submit" data-variant="solid">{ruleIsNew ? "Add rule" : "Save rule"}</Button>
        {#if !ruleIsNew}<Button onclick={resetRule}>Cancel</Button>{/if}
      </div>
    </form>
  </div>

  <h2 style="margin-top: 2.5rem">Board options</h2>
  <p class="page-intro">Meal plans offered with a room, priced per person per night.</p>
  <div class="split">
    <section aria-label="Board options">
      {#if boards.length}
        <table class="list">
          <thead>
            <tr><th>Code</th><th>Name</th><th>Adult</th><th>Child</th><th><span class="muted">Actions</span></th></tr>
          </thead>
          <tbody>
            {#each boards as b (b.id)}
              <tr aria-current={b.id === board.id ? "true" : undefined}>
                <td>{b.code}</td>
                <td>{b.name}</td>
                <td>{formatMoney(b.adult_price)}</td>
                <td>{formatMoney(b.child_price)}</td>
                <td class="actions">
                  <Button onclick={() => (board = { ...b })}>Edit</Button>
                  <Button data-variant="destructive" onclick={() => removeBoard(b)}>Delete</Button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="muted">No board options yet. Add Room only (RO) and Bed &amp; breakfast (BB) to start.</p>
      {/if}
    </section>
    <form class="stack panel" onsubmit={saveBoard} aria-labelledby="board-form-heading">
      <h2 id="board-form-heading">{boardIsNew ? "Add a board option" : `Edit ${board.name}`}</h2>
      <div class="row">
        <TextField label="Code" bind:value={board.code} placeholder="BB" maxlength={6} required />
        <TextField label="Name" bind:value={board.name} placeholder="Bed & breakfast" required />
      </div>
      <div class="row">
        <MoneyField label="Per adult per night" bind:value={board.adult_price} />
        <MoneyField label="Per child per night" bind:value={board.child_price} />
      </div>
      <div class="row">
        <Button type="submit" data-variant="solid">{boardIsNew ? "Add board option" : "Save"}</Button>
        {#if !boardIsNew}<Button onclick={() => (board = blankBoard())}>Cancel</Button>{/if}
      </div>
    </form>
  </div>
</div>

<style>
  :global(.days) {
    display: flex;
    flex-wrap: wrap;
    gap: calc(var(--ui-spacing) * 1);
  }

  .switch-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  tr.inactive td {
    color: var(--ui-muted-foreground);
  }
</style>
