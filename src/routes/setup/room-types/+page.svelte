<script lang="ts">
  import { onMount } from "svelte";
  import { confirm } from "@ui-sdk/svelte/alert-dialog";
  import { Button } from "@ui-sdk/svelte/button";
  import { toast } from "@ui-sdk/svelte/toast";
  import MoneyField from "#lib/components/MoneyField.svelte";
  import NumberField from "#lib/components/NumberField.svelte";
  import { formatMoney } from "#lib/format.ts";
  import TextField from "#lib/components/TextField.svelte";
  import { api, errorMessage, newId } from "#lib/api.ts";
  import type { RoomType } from "#lib/bindings/RoomType.ts";

  let roomTypes = $state<RoomType[]>([]);
  let draft = $state<RoomType>(blank());
  const isNew = $derived(!roomTypes.some((t) => t.id === draft.id));

  function blank(): RoomType {
    return {
      id: newId(),
      name: "",
      code: "",
      default_occupancy: 2,
      max_occupancy: 2,
      description: "",
      base_rate: 9000,
      extra_adult_rate: 0,
      child_rate: 0,
    };
  }

  async function load() {
    try {
      roomTypes = await api.listRoomTypes();
    } catch (error) {
      toast.error("Couldn't load room types", { description: errorMessage(error) });
    }
  }

  onMount(load);

  async function save(event: SubmitEvent) {
    event.preventDefault();
    try {
      await api.saveRoomType($state.snapshot(draft));
      toast.success(isNew ? `Added ${draft.name}` : `Saved ${draft.name}`);
      await load();
      draft = blank();
    } catch (error) {
      toast.error("Couldn't save room type", { description: errorMessage(error) });
    }
  }

  async function remove(roomType: RoomType) {
    const yes = await confirm({
      title: `Delete ${roomType.name}?`,
      description: "Room types still used by rooms can't be deleted.",
      confirmLabel: "Delete room type",
      cancelLabel: "Cancel",
    });
    if (!yes) return;
    try {
      await api.deleteRoomType(roomType.id);
      toast.success(`Deleted ${roomType.name}`);
      await load();
      if (draft.id === roomType.id) draft = blank();
    } catch (error) {
      toast.error("Couldn't delete room type", { description: errorMessage(error) });
    }
  }
</script>

<div class="page">
  <h1>Room types</h1>
  <p class="page-intro">Categories of bedroom, like Double or Family. Rates are set per type later.</p>

  <div class="split">
    <section aria-labelledby="types-heading">
      <h2 id="types-heading" class="muted" style="margin-bottom: 0.5rem">All room types</h2>
      {#if roomTypes.length}
        <table class="list">
          <thead>
            <tr><th>Name</th><th>Code</th><th>Guests</th><th>Per night</th><th><span class="muted">Actions</span></th></tr>
          </thead>
          <tbody>
            {#each roomTypes as rt (rt.id)}
              <tr aria-current={rt.id === draft.id ? "true" : undefined}>
                <td>{rt.name}</td>
                <td>{rt.code}</td>
                <td>{rt.default_occupancy}–{rt.max_occupancy}</td>
                <td>{formatMoney(rt.base_rate)}</td>
                <td class="actions">
                  <Button onclick={() => (draft = { ...rt })}>Edit</Button>
                  <Button data-variant="destructive" onclick={() => remove(rt)}>Delete</Button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="muted">No room types yet.</p>
      {/if}
    </section>

    <form class="stack panel" onsubmit={save} aria-labelledby="type-form-heading">
      <h2 id="type-form-heading">{isNew ? "Add a room type" : `Edit ${draft.name}`}</h2>
      <div class="row">
        <TextField label="Name" bind:value={draft.name} placeholder="Double" required />
        <TextField
          label="Code"
          bind:value={draft.code}
          placeholder="DBL"
          maxlength={6}
          description="Shown on the booking calendar"
          required
        />
      </div>
      <div class="row">
        <NumberField label="Usual guests" bind:value={draft.default_occupancy} min={1} max={20} />
        <NumberField label="Maximum guests" bind:value={draft.max_occupancy} min={1} max={20} />
      </div>
      <MoneyField
        label="Room rate per night"
        bind:value={draft.base_rate}
        description="Before seasonal or weekend rules"
      />
      <div class="row">
        <MoneyField
          label="Extra adult per night"
          bind:value={draft.extra_adult_rate}
          description="Each adult above usual guests"
        />
        <MoneyField label="Child per night" bind:value={draft.child_rate} />
      </div>
      <TextField label="Description" bind:value={draft.description} />
      <div class="row">
        <Button type="submit" data-variant="solid">{isNew ? "Add room type" : "Save room type"}</Button>
        {#if !isNew}
          <Button onclick={() => (draft = blank())}>Cancel</Button>
        {/if}
      </div>
    </form>
  </div>
</div>
