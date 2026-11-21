<script lang="ts">
  import { onMount } from "svelte";
  import type { ChangeDetails, ResizeBox, ResizeReason } from "@ui-sdk/core";
  import { confirm } from "@ui-sdk/svelte/alert-dialog";
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
  import { Switch, SwitchThumb } from "@ui-sdk/svelte/switch";
  import { Textarea } from "@ui-sdk/svelte/textarea";
  import { toast } from "@ui-sdk/svelte/toast";
  import OptionalNumberField from "#lib/components/OptionalNumberField.svelte";
  import TextField from "#lib/components/TextField.svelte";
  import { api, errorMessage, newId } from "#lib/api.ts";
  import type { Floor } from "#lib/bindings/Floor.ts";
  import type { GridRect } from "#lib/bindings/GridRect.ts";
  import type { RoomType } from "#lib/bindings/RoomType.ts";
  import type { Space } from "#lib/bindings/Space.ts";
  import type { SpaceKind } from "#lib/bindings/SpaceKind.ts";
  import type { Boxes } from "#lib/floor-plan/editor.ts";
  import {
    FloorPlanBox,
    FloorPlanEditor,
    FloorPlanHandle,
    FloorPlanMove,
  } from "#lib/floor-plan/editor-parts.ts";
  import { findFreeSpot, firstOverlap, fitsWithin } from "#lib/floor-plan/grid.ts";

  const CELL = 28;
  const kinds = [
    { value: "room", label: "Bedroom" },
    { value: "suite", label: "Function suite" },
  ];

  let floors = $state<Floor[]>([]);
  let roomTypes = $state<RoomType[]>([]);
  let allSpaces = $state<Space[]>([]);
  let floorId = $state<string | null>(null);
  let selected = $state<string | null>(null);
  let draft = $state<Space | null>(null);

  const floor = $derived(floors.find((f) => f.id === floorId) ?? null);
  const spaces = $derived(allSpaces.filter((s) => s.floor_id === floorId));
  const floorItems = $derived(floors.map((f) => ({ value: f.id, label: f.name })));
  const typeItems = $derived(roomTypes.map((t) => ({ value: t.id, label: `${t.name} (${t.code})` })));
  const typeCode = $derived(new Map(roomTypes.map((t) => [t.id, t.code])));
  const boxes = $derived<Boxes>(Object.fromEntries(spaces.map((s) => [s.id, toBox(s.rect)])));
  const labels = $derived(Object.fromEntries(spaces.map((s) => [s.id, s.name])));

  function toBox(r: GridRect): ResizeBox {
    return { x: r.x * CELL, y: r.y * CELL, width: r.w * CELL, height: r.h * CELL };
  }

  function toRect(b: ResizeBox): GridRect {
    return {
      x: Math.round(b.x / CELL),
      y: Math.round(b.y / CELL),
      w: Math.round(b.width / CELL),
      h: Math.round(b.height / CELL),
    };
  }

  async function loadAll() {
    try {
      [floors, roomTypes, allSpaces] = await Promise.all([
        api.listFloors(),
        api.listRoomTypes(),
        api.listSpaces(),
      ]);
      if (!floors.some((f) => f.id === floorId)) floorId = floors[0]?.id ?? null;
      if (selected && !allSpaces.some((s) => s.id === selected)) select(null);
    } catch (error) {
      toast.error("Couldn't load the layout", { description: errorMessage(error) });
    }
  }

  onMount(loadAll);

  function select(key: string | null) {
    selected = key;
    const space = allSpaces.find((s) => s.id === key);
    draft = space ? structuredClone($state.snapshot(space)) : null;
  }

  async function persist(space: Space, success?: string): Promise<boolean> {
    try {
      await api.saveSpace(space);
      allSpaces = allSpaces.some((s) => s.id === space.id)
        ? allSpaces.map((s) => (s.id === space.id ? space : s))
        : [...allSpaces, space];
      if (success) toast.success(success);
      return true;
    } catch (error) {
      toast.error(`Couldn't save ${space.name}`, { description: errorMessage(error) });
      await loadAll();
      return false;
    }
  }

  function onBoxesChange(next: Boxes, details: ChangeDetails<ResizeReason>) {
    if (!floor) return;
    const id = Object.keys(next).find((key) => {
      const before = boxes[key];
      const after = next[key];
      return (
        before.x !== after.x ||
        before.y !== after.y ||
        before.width !== after.width ||
        before.height !== after.height
      );
    });
    const space = spaces.find((s) => s.id === id);
    if (!id || !space) return;
    const rect = toRect(next[id]);
    if (!fitsWithin(rect, floor.grid_cols, floor.grid_rows)) {
      details.cancel();
      return;
    }
    const clash = firstOverlap(id, rect, spaces);
    if (clash) {
      details.cancel();
      toast.warning(`${space.name} would overlap ${clash.name}`);
      return;
    }
    const moved = { ...$state.snapshot(space), rect };
    if (draft?.id === id) draft.rect = rect;
    void persist(moved);
  }

  function suggestName(kind: SpaceKind): string {
    const taken = new Set(allSpaces.map((s) => s.name.trim().toLowerCase()));
    const prefix = kind === "suite" ? "Suite " : floor && floor.level === 0 ? "G" : `${floor?.level ?? ""}`;
    for (let n = 1; ; n++) {
      const name = kind === "suite" ? `${prefix}${n}` : `${prefix}${String(n).padStart(2, "0")}`;
      if (!taken.has(name.toLowerCase())) return name;
    }
  }

  async function add(kind: SpaceKind) {
    if (!floor) return;
    if (kind === "room" && roomTypes.length === 0) {
      toast.warning("Add a room type first", { description: "Setup → Room types" });
      return;
    }
    const size = kind === "room" ? { w: 3, h: 4 } : { w: 6, h: 4 };
    const spot = findFreeSpot(
      spaces.map((s) => s.rect),
      floor.grid_cols,
      floor.grid_rows,
      size.w,
      size.h,
    );
    if (!spot) {
      toast.warning("No free space on this floor", {
        description: "Make the grid bigger on the Floors page, or move rooms closer together.",
      });
      return;
    }
    const space: Space = {
      id: newId(),
      floor_id: floor.id,
      kind,
      name: suggestName(kind),
      room_type_id: kind === "room" ? roomTypes[0].id : null,
      suite_capacities: { theatre: null, boardroom: null, banquet: null, cabaret: null, reception: null },
      rect: spot,
      out_of_service: false,
      notes: "",
    };
    if (await persist(space, `Added ${space.name}`)) select(space.id);
  }

  function setKind(kind: string | null) {
    if (!draft || (kind !== "room" && kind !== "suite")) return;
    draft.kind = kind;
    draft.room_type_id = kind === "room" ? (draft.room_type_id ?? roomTypes[0]?.id ?? null) : null;
  }

  async function saveDraft(event: SubmitEvent) {
    event.preventDefault();
    if (draft) await persist($state.snapshot(draft), `Saved ${draft.name}`);
  }

  async function removeDraft() {
    if (!draft) return;
    const { id, name } = draft;
    const yes = await confirm({
      title: `Delete ${name}?`,
      description: "It's removed from the floor plan. Existing bookings will keep their history.",
      confirmLabel: "Delete",
      cancelLabel: "Cancel",
    });
    if (!yes) return;
    try {
      await api.deleteSpace(id);
      toast.success(`Deleted ${name}`);
      select(null);
      await loadAll();
    } catch (error) {
      toast.error(`Couldn't delete ${name}`, { description: errorMessage(error) });
    }
  }
</script>

<div class="page">
  <h1>Layout editor</h1>
  <p class="page-intro">
    Drag rooms to place them and drag the corner to resize. With the keyboard, Tab to a room, then
    use the arrow keys to move it (Shift moves further).
  </p>

  {#if floors.length === 0}
    <p class="muted">Add a floor on the <a href="/setup/floors">Floors</a> page first.</p>
  {:else}
    <div class="row toolbar">
      <Field>
        <FieldLabel>Floor</FieldLabel>
        <Select
          items={floorItems}
          value={floorId}
          onValueChange={(value) => {
            floorId = value;
            select(null);
          }}
        >
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            {#each floorItems as item (item.value)}
              <SelectItem value={item.value}>{item.label}<SelectItemIndicator /></SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </Field>
      <Button onclick={() => add("room")}>Add bedroom</Button>
      <Button onclick={() => add("suite")}>Add function suite</Button>
    </div>

    <div class="editor-layout">
      {#if floor}
        <div class="canvas-scroll">
          <FloorPlanEditor
            class="canvas"
            aria-label={`${floor.name} layout`}
            {boxes}
            {onBoxesChange}
            {selected}
            onSelectedChange={(key) => select(key)}
            width={floor.grid_cols * CELL}
            height={floor.grid_rows * CELL}
            step={CELL}
            {labels}
          >
            {#each spaces as space (space.id)}
              <FloorPlanBox
                value={space.id}
                class="space"
                data-kind={space.kind}
                data-out-of-service={space.out_of_service ? "" : undefined}
              >
                <FloorPlanMove value={space.id} class="space-move">
                  <span class="space-name">{space.name}</span>
                  <span class="space-meta">
                    {space.kind === "suite" ? "Suite" : (typeCode.get(space.room_type_id ?? "") ?? "")}
                  </span>
                </FloorPlanMove>
                {#if selected === space.id}
                  <FloorPlanHandle value={space.id} handle="se" class="space-handle" aria-label={`Resize ${space.name}`} />
                {/if}
              </FloorPlanBox>
            {/each}
          </FloorPlanEditor>
        </div>
      {/if}

      <aside class="panel" aria-labelledby="details-heading">
        {#if draft}
          <form class="stack" onsubmit={saveDraft}>
            <h2 id="details-heading">{draft.name || "Untitled"}</h2>
            <Field>
              <FieldLabel>Type of space</FieldLabel>
              <Select items={kinds} value={draft.kind} onValueChange={setKind}>
                <SelectTrigger><SelectValue /></SelectTrigger>
                <SelectContent>
                  {#each kinds as k (k.value)}
                    <SelectItem value={k.value}>{k.label}<SelectItemIndicator /></SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </Field>
            <TextField
              label={draft.kind === "room" ? "Room number" : "Suite name"}
              bind:value={draft.name}
              required
            />
            {#if draft.kind === "room"}
              <Field>
                <FieldLabel>Room type</FieldLabel>
                <Select items={typeItems} bind:value={draft.room_type_id}>
                  <SelectTrigger><SelectValue /></SelectTrigger>
                  <SelectContent>
                    {#each typeItems as t (t.value)}
                      <SelectItem value={t.value}>{t.label}<SelectItemIndicator /></SelectItem>
                    {/each}
                  </SelectContent>
                </Select>
              </Field>
            {:else}
              <fieldset class="capacities">
                <legend>Capacity by layout (leave empty if not offered)</legend>
                <div class="row">
                  <OptionalNumberField label="Theatre" bind:value={draft.suite_capacities.theatre} />
                  <OptionalNumberField label="Boardroom" bind:value={draft.suite_capacities.boardroom} />
                  <OptionalNumberField label="Banquet" bind:value={draft.suite_capacities.banquet} />
                  <OptionalNumberField label="Cabaret" bind:value={draft.suite_capacities.cabaret} />
                  <OptionalNumberField label="Reception" bind:value={draft.suite_capacities.reception} />
                </div>
              </fieldset>
            {/if}
            <label data-slot="label" class="switch-row">
              Out of service
              <Switch bind:checked={draft.out_of_service}><SwitchThumb /></Switch>
            </label>
            <Field>
              <FieldLabel>Notes</FieldLabel>
              <Textarea bind:value={draft.notes} rows={2} />
            </Field>
            <p class="muted">
              Column {draft.rect.x + 1}, row {draft.rect.y + 1}; {draft.rect.w} × {draft.rect.h} cells
            </p>
            <div class="row">
              <Button type="submit" data-variant="solid">Save</Button>
              <Button data-variant="destructive" onclick={removeDraft}>Delete</Button>
            </div>
          </form>
        {:else}
          <h2 id="details-heading">Details</h2>
          <p class="muted">Select a room or suite to edit it.</p>
        {/if}
      </aside>
    </div>
  {/if}
</div>

<style>
  .toolbar {
    margin-bottom: var(--app-gap);
  }

  .toolbar :global([data-slot="field"]) {
    flex: 0 1 16rem;
  }

  .editor-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 20rem;
    gap: calc(var(--ui-spacing) * 6);
    align-items: start;
  }

  .canvas-scroll {
    overflow: auto;
    border: 1px solid var(--ui-border);
    border-radius: var(--ui-radius);
    padding: calc(var(--ui-spacing) * 3);
  }

  .canvas-scroll :global(.canvas) {
    background-color: var(--ui-background);
    background-image:
      linear-gradient(to right, var(--ui-border) 1px, transparent 1px),
      linear-gradient(to bottom, var(--ui-border) 1px, transparent 1px);
    background-size: var(--cell) var(--cell);
    outline: 1px solid var(--ui-border);
  }

  .canvas-scroll :global(.space) {
    position: absolute;
    box-sizing: border-box;
    padding: 2px;
  }

  .canvas-scroll :global(.space-move) {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    border: 1.5px solid var(--edge);
    border-radius: 4px;
    background: var(--paper);
    color: inherit;
    font: inherit;
    cursor: grab;
    overflow: hidden;
  }

  .canvas-scroll :global(.space[data-kind="suite"] .space-move) {
    border-color: var(--heather);
    background: var(--heather-wash);
  }

  .canvas-scroll :global(.space[data-out-of-service] .space-move) {
    background-image: repeating-linear-gradient(
      45deg,
      transparent 0 6px,
      light-dark(oklch(0 0 0 / 0.08), oklch(1 0 0 / 0.1)) 6px 12px
    );
  }

  .canvas-scroll :global(.space[data-selected] .space-move) {
    border-color: var(--ui-primary);
    border-width: 3px;
  }

  .canvas-scroll :global(.space[data-dragging]) {
    z-index: 1;
    opacity: 0.85;
  }

  .canvas-scroll :global(.space-move:focus-visible) {
    outline: 2px solid var(--ui-ring-border);
    outline-offset: 1px;
  }

  .canvas-scroll :global(.space-name) {
    font-weight: 600;
  }

  .canvas-scroll :global(.space-meta) {
    font-size: 0.75rem;
    color: var(--ui-muted-foreground);
  }

  .canvas-scroll :global(.space-handle) {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 14px;
    height: 14px;
    padding: 0;
    border: 2px solid var(--ui-background);
    border-radius: 3px;
    background: var(--ui-primary);
  }

  .capacities {
    border: 1px solid var(--ui-border);
    border-radius: var(--ui-radius);
    padding: calc(var(--ui-spacing) * 3);
    margin: 0;
  }

  .capacities legend {
    color: var(--ui-muted-foreground);
    padding: 0 calc(var(--ui-spacing) * 1);
  }

  .capacities :global([data-slot="field"]) {
    flex: 1 1 5rem;
  }

  .switch-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  @media (max-width: 60rem) {
    .editor-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
