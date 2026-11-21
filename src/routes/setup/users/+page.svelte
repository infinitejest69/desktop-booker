<script lang="ts">
  import { onMount } from "svelte";
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
  import TextField from "#lib/components/TextField.svelte";
  import { api, errorMessage, newId } from "#lib/api.ts";
  import type { Role } from "#lib/bindings/Role.ts";
  import type { User } from "#lib/bindings/User.ts";
  import { useSession } from "#lib/session/session.svelte.ts";

  const session = useSession();
  const roles = [
    { value: "staff", label: "Staff: bookings and check-in/out" },
    { value: "admin", label: "Admin: everything, including setup and prices" },
  ];

  let users = $state<User[]>([]);
  let draft = $state<User>(blank());
  let pin = $state("");
  const isNew = $derived(!users.some((u) => u.id === draft.id));

  function blank(): User {
    return { id: newId(), name: "", role: "staff", active: true };
  }

  async function load() {
    try {
      users = await api.listUsers();
    } catch (error) {
      toast.error("Couldn't load users", { description: errorMessage(error) });
    }
  }

  onMount(load);

  function edit(u: User) {
    draft = { ...u };
    pin = "";
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    try {
      await api.saveUser($state.snapshot(draft), pin || null);
      toast.success(isNew ? `Added ${draft.name}` : `Saved ${draft.name}`);
      await load();
      await session.refresh();
      edit(blank());
    } catch (error) {
      toast.error("Couldn't save", { description: errorMessage(error) });
    }
  }
</script>

<div class="page">
  <h1>Users</h1>
  <p class="page-intro">
    Everyone who uses the desk signs in with their own PIN, so the audit log shows who did what.
  </p>

  <div class="split">
    <section aria-label="All users">
      <table class="list">
        <thead><tr><th>Name</th><th>Role</th><th>Status</th><th><span class="muted">Actions</span></th></tr></thead>
        <tbody>
          {#each users as u (u.id)}
            <tr aria-current={u.id === draft.id ? "true" : undefined}>
              <td>{u.name}{u.id === session.user?.id ? " (you)" : ""}</td>
              <td>{u.role === "admin" ? "Admin" : "Staff"}</td>
              <td>{u.active ? "Active" : "Can't sign in"}</td>
              <td class="actions"><Button onclick={() => edit(u)}>Edit</Button></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>

    <form class="stack panel" onsubmit={save} aria-labelledby="user-form-heading">
      <h2 id="user-form-heading">{isNew ? "Add a user" : `Edit ${draft.name}`}</h2>
      <TextField label="Name" bind:value={draft.name} required />
      <Field>
        <FieldLabel>Role</FieldLabel>
        <Select items={roles} value={draft.role} onValueChange={(v) => v && (draft.role = v as Role)}>
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            {#each roles as r (r.value)}
              <SelectItem value={r.value}>{r.label}<SelectItemIndicator /></SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </Field>
      <Field>
        <FieldLabel>{isNew ? "PIN" : "New PIN"}</FieldLabel>
        <Input type="password" inputmode="numeric" autocomplete="new-password" bind:value={pin} required={isNew} />
        <FieldDescription>
          {isNew ? "4 to 8 digits." : "Leave empty to keep their current PIN."}
        </FieldDescription>
      </Field>
      {#if !isNew}
        <label data-slot="label" class="switch-row">
          Can sign in
          <Switch bind:checked={draft.active}><SwitchThumb /></Switch>
        </label>
      {/if}
      <div class="row">
        <Button type="submit" data-variant="solid">{isNew ? "Add user" : "Save user"}</Button>
        {#if !isNew}<Button onclick={() => edit(blank())}>Cancel</Button>{/if}
      </div>
    </form>
  </div>
</div>

<style>
  .switch-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
</style>
