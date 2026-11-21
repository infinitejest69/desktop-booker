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
  import NumberField from "#lib/components/NumberField.svelte";
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
  import {
    findFreeSpot,
    firstOverlap,
    fitsWithin,
    freeSpots,
    minGridSize,
    placeAt,
  } from "#lib/floor-plan/grid.ts";

  const CELL = 28;
  const MAX_GRID = 200;
  const kinds = [
    { value: "room", label: "Bedroom" },
    { value: "suite", label: "Function suite" },
  ];

  interface Template {
    id: string;
    label: string;
    kind: SpaceKind;
    w: number;
    h: number;
  }

  const templates: Template[] = [
    { id: "bedroom", label: "Bedroom", kind: "room", w: 3, h: 4 },
    { id: "family", label: "Family room", kind: "room", w: 4, h: 4 },
    { id: "suite", label: "Function suite", kind: "suite", w: 6, h: 4 },
  ];

  let floors = $state<Floor[]>([]);
  let roomTypes = $state<RoomType[]>([]);
  let allSpaces = $state<Space[]>([]);
  let floorId = $state<string | null>(null);
  let selected = $state<string | null>(null);
  let draft = $state<Space | null>(null);
  let floorDraft = $state<Floor | null>(null);
  let howMany = $state(4);
  let dropping = $state(false);

  // While dragging a grid edge: the size being previewed.
  let resizing = $state<{ cols: number; rows: number } | null>(null);

  const floor = $derived(floors.find((f) => f.id === floorId) ?? null);
  const spaces = $derived(allSpaces.filter((s) => s.floor_id === floorId));
  const floorItems = $derived(floors.map((f) => ({ value: f.id, label: f.name })));
  const typeItems = $derived(roomTypes.map((t) => ({ value: t.id, label: `${t.name} (${t.code})` })));
  const typeCode = $derived(new Map(roomTypes.map((t) => [t.id, t.code])));
  const boxes = $derived<Boxes>(Object.fromEntries(spaces.map((s) => [s.id, toBox(s.rect)])));
  const labels = $derived(Object.fromEntries(spaces.map((s) => [s.id, s.name])));
  const cols = $derived(resizing?.cols ?? floor?.grid_cols ?? 1);
  const rows = $derived(resizing?.rows ?? floor?.grid_rows ?? 1);
  const minSize = $derived(minGridSize(spaces.map((s) => s.rect)));

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
      syncFloorDraft();
    } catch (error) {
      toast.error("Couldn't load the floors", { description: errorMessage(error) });
    }
  }

  onMount(loadAll);

  function syncFloorDraft() {
    const f = floors.find((x) => x.id === floorId);
    floorDraft = f ? { ...f } : null;
  }

  function chooseFloor(id: string | null) {
    floorId = id;
    select(null);
    syncFloorDraft();
  }

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

  function newSpace(t: Template, rect: GridRect): Space | null {
    if (!floor) return null;
    if (t.kind === "room" && roomTypes.length === 0) {
      toast.warning("Add a room type first", { description: "My hotel → Room types" });
      return null;
    }
    const family = roomTypes.find((r) => r.max_occupancy >= 3) ?? roomTypes[0];
    return {
      id: newId(),
      floor_id: floor.id,
      kind: t.kind,
      name: suggestName(t.kind),
      room_type_id: t.kind === "room" ? (t.id === "family" ? family.id : roomTypes[0].id) : null,
      suite_capacities: { theatre: null, boardroom: null, banquet: null, cabaret: null, reception: null },
      rect,
      out_of_service: false,
      notes: "",
    };
  }

  function noRoom() {
    toast.warning("No free space for that on this floor", {
      description: "Drag the grid's edge to make it bigger, or move rooms closer together.",
    });
  }

  async function place(t: Template, rect?: GridRect) {
    if (!floor) return;
    const spot = rect ?? findFreeSpot(spaces.map((s) => s.rect), floor.grid_cols, floor.grid_rows, t.w, t.h);
    if (!spot) return noRoom();
    const clash = firstOverlap("", spot, spaces);
    if (clash) {
      toast.warning(`That spot overlaps ${clash.name}`, { description: "Drop it on an empty part of the grid." });
      return;
    }
    const space = newSpace(t, spot);
    if (space && (await persist(space, `Added ${space.name}`))) select(space.id);
  }

  async function addSeveral() {
    if (!floor) return;
    const t = templates[0];
    const spots = freeSpots(spaces.map((s) => s.rect), floor.grid_cols, floor.grid_rows, t.w, t.h, howMany);
    if (spots.length === 0) return noRoom();
    let added = 0;
    for (const spot of spots) {
      const space = newSpace(t, spot);
      if (!space || !(await persist(space))) break;
      added++;
    }
    toast.success(`Added ${added} bedroom${added === 1 ? "" : "s"}`, {
      description: added < howMany ? "That's all that fitted. Drag the grid's edge to make room for more." : undefined,
    });
  }

  function onDrop(event: DragEvent) {
    event.preventDefault();
    dropping = false;
    const t = templates.find((x) => x.id === event.dataTransfer?.getData("text/plain"));
    const canvas = (event.currentTarget as HTMLElement).querySelector(".canvas");
    if (!t || !canvas || !floor) return;
    const r = canvas.getBoundingClientRect();
    const cx = (event.clientX - r.left) / CELL;
    const cy = (event.clientY - r.top) / CELL;
    if (t.w > floor.grid_cols || t.h > floor.grid_rows) return noRoom();
    void place(t, placeAt(cx, cy, t.w, t.h, floor.grid_cols, floor.grid_rows));
  }

  // The plan fills the window down to its bottom edge, so both of its
  // scrollbars are always on screen, however big the grid is.
  function fitToWindow(el: HTMLElement) {
    const fit = () => {
      const top = el.getBoundingClientRect().top + window.scrollY;
      el.style.maxHeight = `${Math.max(240, window.innerHeight - top - 24)}px`;
    };
    fit();
    window.addEventListener("resize", fit);
    return () => window.removeEventListener("resize", fit);
  }

  // ---- grid size: drag the right edge, bottom edge or corner ----

  async function saveGrid(nextCols: number, nextRows: number) {
    if (!floor) return;
    const c = Math.min(Math.max(nextCols, minSize.cols), MAX_GRID);
    const r = Math.min(Math.max(nextRows, minSize.rows), MAX_GRID);
    if (c === floor.grid_cols && r === floor.grid_rows) return;
    const next = { ...$state.snapshot(floor), grid_cols: c, grid_rows: r };
    try {
      await api.saveFloor(next);
      floors = floors.map((f) => (f.id === next.id ? next : f));
      syncFloorDraft();
    } catch (error) {
      toast.error("Couldn't resize the grid", { description: errorMessage(error) });
    }
  }

  function startResize(event: PointerEvent, edge: "e" | "s" | "se") {
    if (!floor) return;
    event.preventDefault();
    const handle = event.currentTarget as HTMLElement;
    handle.setPointerCapture(event.pointerId);
    const start = { x: event.clientX, y: event.clientY, cols: floor.grid_cols, rows: floor.grid_rows };
    const clamp = (v: number, min: number) => Math.min(Math.max(v, min), MAX_GRID);
    resizing = { cols: start.cols, rows: start.rows };
    const move = (e: PointerEvent) => {
      resizing = {
        cols: edge === "s" ? start.cols : clamp(start.cols + Math.round((e.clientX - start.x) / CELL), minSize.cols),
        rows: edge === "e" ? start.rows : clamp(start.rows + Math.round((e.clientY - start.y) / CELL), minSize.rows),
      };
    };
    const end = () => {
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", end);
      handle.removeEventListener("pointercancel", end);
      const done = resizing;
      resizing = null;
      if (done) void saveGrid(done.cols, done.rows);
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", end);
    handle.addEventListener("pointercancel", end);
  }

  function resizeKey(event: KeyboardEvent, edge: "e" | "s") {
    if (!floor) return;
    const step = event.shiftKey ? 5 : 1;
    const delta: Record<string, number> = edge === "e" ? { ArrowRight: step, ArrowLeft: -step } : { ArrowDown: step, ArrowUp: -step };
    const d = delta[event.key];
    if (d === undefined) return;
    event.preventDefault();
    void saveGrid(floor.grid_cols + (edge === "e" ? d : 0), floor.grid_rows + (edge === "s" ? d : 0));
  }

  // ---- floors ----

  async function addFloor() {
    const level = floors.length ? Math.max(...floors.map((f) => f.level)) + 1 : 0;
    const taken = new Set(floors.map((f) => f.name.toLowerCase()));
    let name = level === 0 ? "Ground floor" : `Floor ${level}`;
    for (let n = 2; taken.has(name.toLowerCase()); n++) name = `Floor ${level} (${n})`;
    const f: Floor = { id: newId(), name, level, grid_cols: 24, grid_rows: 10 };
    try {
      await api.saveFloor(f);
      floors = [...floors, f].sort((a, b) => a.level - b.level);
      chooseFloor(f.id);
      toast.success(`Added ${name}`, { description: "Rename it in the panel on the right." });
    } catch (error) {
      toast.error("Couldn't add a floor", { description: errorMessage(error) });
    }
  }

  async function saveFloorDraft(event: SubmitEvent) {
    event.preventDefault();
    if (!floorDraft) return;
    try {
      await api.saveFloor($state.snapshot(floorDraft));
      toast.success(`Saved ${floorDraft.name}`);
      await loadAll();
    } catch (error) {
      toast.error("Couldn't save the floor", { description: errorMessage(error) });
    }
  }

  async function deleteFloor() {
    if (!floor) return;
    const yes = await confirm({
      title: `Delete ${floor.name}?`,
      description: "Only an empty floor can be deleted. Move or delete its rooms first.",
      confirmLabel: "Delete floor",
      cancelLabel: "Cancel",
    });
    if (!yes) return;
    try {
      await api.deleteFloor(floor.id);
      toast.success(`Deleted ${floor.name}`);
      floorId = null;
      await loadAll();
    } catch (error) {
      toast.error("Couldn't delete the floor", { description: errorMessage(error) });
    }
  }

  // ---- the selected room or suite ----

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

<div class="page wide">
  <h1>Floors and layout</h1>
  <p class="page-intro">
    Drag pieces from the palette onto a floor, drag rooms to move them and drag a room's corner to
    resize it. Drag the grid's edges to make the floor bigger or smaller. With the keyboard, Tab to
    a room and use the arrow keys (Shift moves further).
  </p>

  <div class="row toolbar">
    {#if floors.length}
      <Field>
        <FieldLabel>Floor</FieldLabel>
        <Select items={floorItems} value={floorId} onValueChange={chooseFloor}>
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            {#each floorItems as item (item.value)}
              <SelectItem value={item.value}>{item.label}<SelectItemIndicator /></SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </Field>
    {/if}
    <Button data-variant={floors.length ? undefined : "solid"} onclick={addFloor}>Add floor</Button>
  </div>

  {#if floor}
    <section class="palette" aria-label="Add rooms and suites">
      <div class="pick">
      <p class="muted">Drag onto the plan, or click to place in the first free space:</p>
      <div class="row pieces">
        {#each templates as t (t.id)}
          <button
            type="button"
            class="piece"
            data-kind={t.kind}
            draggable="true"
            ondragstart={(e) => {
              e.dataTransfer?.setData("text/plain", t.id);
              if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
            }}
            onclick={() => place(t)}
          >
            <span class="swatch" style={`--w: ${t.w}; --h: ${t.h}`} aria-hidden="true"></span>
            {t.label} <span class="muted">{t.w} × {t.h}</span>
          </button>
        {/each}
      </div>
      </div>
      <form class="row several" onsubmit={(e) => { e.preventDefault(); void addSeveral(); }}>
        <NumberField label="Bedrooms to add" bind:value={howMany} min={1} max={40} />
        <Button type="submit">Add {howMany} bedrooms</Button>
      </form>
    </section>
  {/if}

  <div class="editor-layout">
    {#if floor}
      <div
        class="canvas-scroll"
        role="region"
        aria-label={`${floor.name} plan`}
        data-dropping={dropping ? "" : undefined}
        {@attach fitToWindow}
        ondragover={(e) => {
          e.preventDefault();
          dropping = true;
        }}
        ondragleave={() => (dropping = false)}
        ondrop={onDrop}
      >
        <div class="frame" style={`width: ${cols * CELL}px; height: ${rows * CELL}px`}>
          <FloorPlanEditor
            class="canvas"
            aria-label={`${floor.name} layout`}
            {boxes}
            {onBoxesChange}
            {selected}
            onSelectedChange={(key) => select(key)}
            width={cols * CELL}
            height={rows * CELL}
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
          <button
            type="button"
            class="grid-handle edge-e"
            aria-label={`Floor width: ${cols} cells. Drag or use the left and right arrow keys.`}
            onpointerdown={(e) => startResize(e, "e")}
            onkeydown={(e) => resizeKey(e, "e")}
          ></button>
          <button
            type="button"
            class="grid-handle edge-s"
            aria-label={`Floor depth: ${rows} cells. Drag or use the up and down arrow keys.`}
            onpointerdown={(e) => startResize(e, "s")}
            onkeydown={(e) => resizeKey(e, "s")}
          ></button>
          <button
            type="button"
            class="grid-handle corner"
            tabindex="-1"
            aria-hidden="true"
            onpointerdown={(e) => startResize(e, "se")}
          ></button>
          {#if resizing}
            <span class="size-tip">{cols} × {rows}</span>
          {/if}
        </div>
      </div>
    {:else}
      <p class="muted">No floors yet. Add the first one to start laying out rooms.</p>
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
            <Button onclick={() => select(null)}>Floor settings</Button>
          </div>
        </form>
      {:else if floorDraft}
        <form class="stack" onsubmit={saveFloorDraft}>
          <h2 id="details-heading">{floor?.name ?? "Floor"}</h2>
          <p class="muted">Select a room or suite on the plan to edit it.</p>
          <TextField label="Floor name" bind:value={floorDraft.name} required />
          <NumberField
            label="Level"
            bind:value={floorDraft.level}
            min={-5}
            max={50}
            description="0 is the ground floor; floors are listed by level."
          />
          <div class="row">
            <NumberField label="Width (cells)" bind:value={floorDraft.grid_cols} min={minSize.cols} max={MAX_GRID} />
            <NumberField label="Depth (cells)" bind:value={floorDraft.grid_rows} min={minSize.rows} max={MAX_GRID} />
          </div>
          <div class="row">
            <Button type="submit" data-variant="solid">Save floor</Button>
            <Button data-variant="destructive" onclick={deleteFloor}>Delete floor</Button>
          </div>
        </form>
      {:else}
        <h2 id="details-heading">Details</h2>
        <p class="muted">Add a floor to begin.</p>
      {/if}
    </aside>
  </div>
</div>

<style>
  .wide {
    max-width: 96rem;
  }

  .palette {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 4);
    margin-bottom: var(--app-gap);
    padding: calc(var(--ui-spacing) * 4);
    background: var(--paper);
    border: 1px solid var(--rule);
    border-radius: var(--panel-radius);
  }

  .palette p {
    margin: 0 0 calc(var(--ui-spacing) * 2);
  }

  .pieces {
    align-items: center;
  }

  .piece {
    display: inline-flex;
    align-items: center;
    gap: calc(var(--ui-spacing) * 2);
    padding: calc(var(--ui-spacing) * 2) calc(var(--ui-spacing) * 3);
    border: 1.5px solid var(--edge);
    border-radius: var(--ui-radius);
    background: var(--paper);
    color: var(--ink);
    font: inherit;
    font-weight: 700;
    cursor: grab;
  }

  .piece[data-kind="suite"] {
    border-color: var(--heather);
    background: var(--heather-wash);
  }

  .piece:focus-visible {
    outline: 2px solid var(--heather);
    outline-offset: 2px;
  }

  .swatch {
    width: calc(var(--w) * 5px);
    height: calc(var(--h) * 5px);
    border: 1.5px solid currentColor;
    border-radius: 2px;
  }

  .several :global([data-slot="field"]) {
    flex: 0 1 9rem;
  }

  .canvas-scroll {
    min-height: 12rem;
    background: var(--paper);
  }

  .canvas-scroll[data-dropping] {
    outline: 2px dashed var(--heather);
    outline-offset: -2px;
  }

  .frame {
    position: relative;
    margin: 0 18px 18px 0;
  }

  .grid-handle {
    position: absolute;
    padding: 0;
    border: 0;
    background: transparent;
    touch-action: none;
  }

  .grid-handle::after {
    content: "";
    position: absolute;
    border-radius: 3px;
    background: var(--edge);
  }

  .grid-handle:hover::after,
  .grid-handle:focus-visible::after {
    background: var(--heather);
  }

  .grid-handle:focus-visible {
    outline: 2px solid var(--heather);
    outline-offset: 1px;
  }

  .edge-e {
    top: 0;
    right: -14px;
    width: 12px;
    height: 100%;
    cursor: ew-resize;
  }

  .edge-e::after {
    inset: 50% auto auto 4px;
    width: 4px;
    height: 36px;
    translate: 0 -50%;
  }

  .edge-s {
    left: 0;
    bottom: -14px;
    height: 12px;
    width: 100%;
    cursor: ns-resize;
  }

  .edge-s::after {
    inset: 4px auto auto 50%;
    height: 4px;
    width: 36px;
    translate: -50% 0;
  }

  .corner {
    right: -16px;
    bottom: -16px;
    width: 16px;
    height: 16px;
    cursor: nwse-resize;
  }

  .corner::after {
    inset: 4px;
  }

  .size-tip {
    position: absolute;
    right: 0;
    bottom: 0;
    translate: 0 calc(100% + 18px);
    padding: 0.1rem 0.5rem;
    border-radius: var(--ui-radius);
    background: var(--ink);
    color: var(--paper);
    font-weight: 700;
    font-size: 0.8125rem;
    pointer-events: none;
  }

  .toolbar {
    margin-bottom: var(--app-gap);
  }

  .toolbar :global([data-slot="field"]) {
    flex: 0 1 16rem;
  }

  .editor-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 21rem;
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
