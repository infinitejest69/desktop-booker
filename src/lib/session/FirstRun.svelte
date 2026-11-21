<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldDescription, FieldError, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import { api, errorMessage } from "../api";
  import type { User } from "../bindings/User";

  interface Props {
    onDone: (user: User) => void;
  }

  let { onDone }: Props = $props();
  let name = $state("");
  let pin = $state("");
  let again = $state("");
  let error = $state("");
  let busy = $state(false);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    error = "";
    if (pin !== again) {
      error = "The two PINs don't match.";
      return;
    }
    busy = true;
    try {
      onDone(await api.createFirstAdmin(name, pin));
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="gate">
  <form class="panel stack card" onsubmit={submit}>
    <h1>Welcome</h1>
    <p class="muted">
      First, set up the person who looks after the app. They can add the rest of the team later.
    </p>
    <Field>
      <FieldLabel>Your name</FieldLabel>
      <Input bind:value={name} required autocomplete="name" />
    </Field>
    <Field invalid={error !== ""}>
      <FieldLabel>Choose a PIN</FieldLabel>
      <Input type="password" inputmode="numeric" bind:value={pin} required autocomplete="new-password" />
      <FieldDescription>4 to 8 digits. You'll use it to sign in.</FieldDescription>
    </Field>
    <Field invalid={error !== ""}>
      <FieldLabel>Type the PIN again</FieldLabel>
      <Input type="password" inputmode="numeric" bind:value={again} required autocomplete="new-password" />
      <FieldError>{error}</FieldError>
    </Field>
    <div>
      <Button type="submit" data-variant="solid" disabled={busy}>Create admin and sign in</Button>
    </div>
  </form>
</div>

<style>
  .gate {
    min-height: 100vh;
    display: grid;
    place-items: center;
    background: var(--ground);
    padding: calc(var(--ui-spacing) * 6);
  }

  .card {
    width: min(26rem, 100%);
  }

  .card h1 {
    margin: 0;
  }

  .card p {
    margin: 0;
  }
</style>
