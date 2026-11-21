<script lang="ts">
  import { onMount } from "svelte";
  import { confirm } from "@ui-sdk/svelte/alert-dialog";
  import { Button } from "@ui-sdk/svelte/button";
  import { toast } from "@ui-sdk/svelte/toast";
  import NumberField from "#lib/components/NumberField.svelte";
  import TextField from "#lib/components/TextField.svelte";
  import { api, errorMessage, newId } from "#lib/api.ts";
  import type { Floor } from "#lib/bindings/Floor.ts";

  let floors = $state<Floor[]>([]);
  let draft = $state<Floor>(blank([]));
  const isNew = $derived(!floors.some((f) => f.id === draft.id));

  function blank(existing: Floor[]): Floor {
    const nextLevel = existing.length ? Math.max(...existing.map((f) => f.level)) + 1 : 0;
    return { id: newId(), name: "", level: nextLevel, grid_cols: 24, grid_rows: 10 };
  }

  async function load() {
    try {
      floors = await api.listFloors();
    } catch (error) {
      toast.error("Couldn't load floors", { description: errorMessage(error) });
    }
  }

  onMount(async () => {
    await load();
    draft = blank(floors);
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();
    try {
      await api.saveFloor($state.snapshot(draft));
      toast.success(isNew ? `Added ${draft.name}` : `Saved ${draft.name}`);
      await load();
      draft = blank(floors);
    } catch (error) {
      toast.error("Couldn't save floor", { description: errorMessage(error) });
    }
  }

  async function remove(floor: Floor) {
    const yes = await confirm({
      title: `Delete ${floor.name}?`,
      description: "Only empty floors can be deleted.",
      confirmLabel: "Delete floor",
      cancelLabel: "Cancel",
    });
    if (!yes) return;
    try {
      await api.deleteFloor(floor.id);
      toast.success(`Deleted ${floor.name}`);
      await load();
      if (draft.id === floor.id) draft = blank(floors);
    } catch (error) {
      toast.error("Couldn't delete floor", { description: errorMessage(error) });
    }
  }
</script>

<div class="page">
  <h1>Floors</h1>
  <p class="page-intro">
    Each floor is a grid you lay rooms out on. Level 0 is the ground floor.
  </p>

  <div class="split">
    <section aria-labelledby="floors-heading">
      <h2 id="floors-heading" class="muted" style="margin-bottom: 0.5rem">All floors</h2>
      {#if floors.length}
        <table class="list">
          <thead>
            <tr><th>Name</th><th>Level</th><th>Grid</th><th><span class="muted">Actions</span></th></tr>
          </thead>
          <tbody>
            {#each floors as floor (floor.id)}
              <tr aria-current={floor.id === draft.id ? "true" : undefined}>
                <td>{floor.name}</td>
                <td>{floor.level}</td>
                <td>{floor.grid_cols} × {floor.grid_rows}</td>
                <td class="actions">
                  <Button onclick={() => (draft = { ...floor })}>Edit</Button>
                  <Button data-variant="destructive" onclick={() => remove(floor)}>Delete</Button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="muted">No floors yet. Add the first one.</p>
      {/if}
    </section>

    <form class="stack panel" onsubmit={save} aria-labelledby="floor-form-heading">
      <h2 id="floor-form-heading">{isNew ? "Add a floor" : `Edit ${draft.name}`}</h2>
      <TextField label="Name" bind:value={draft.name} placeholder="Ground floor" required />
      <NumberField label="Level" bind:value={draft.level} min={-5} max={50} />
      <div class="row">
        <NumberField label="Grid width" bind:value={draft.grid_cols} min={1} max={200} />
        <NumberField label="Grid height" bind:value={draft.grid_rows} min={1} max={200} />
      </div>
      <div class="row">
        <Button type="submit" data-variant="solid">{isNew ? "Add floor" : "Save floor"}</Button>
        {#if !isNew}
          <Button onclick={() => (draft = blank(floors))}>Cancel</Button>
        {/if}
      </div>
    </form>
  </div>
</div>
