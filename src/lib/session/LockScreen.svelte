<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldError, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import { Radio, RadioGroup, RadioIndicator } from "@ui-sdk/svelte/radio";
  import { onDestroy, tick } from "svelte";
  import { api, errorMessage, isCommandError } from "../api";
  import type { SessionStatus } from "../bindings/SessionStatus";
  import type { User } from "../bindings/User";

  interface Props {
    status: SessionStatus;
    onSignedIn: (user: User) => void;
  }

  let { status, onSignedIn }: Props = $props();
  let picked = $state<string | null>(null);
  const chosen = $derived(picked ?? (status.users.length === 1 ? status.users[0].id : null));
  let pin = $state("");
  let error = $state("");
  let busy = $state(false);

  // Per person: when their wrong-PIN lockout ends (ms since epoch).
  let lockedUntil = $state<Record<string, number>>({});
  let now = $state(Date.now());
  const ticker = setInterval(() => (now = Date.now()), 1000);
  onDestroy(() => clearInterval(ticker));

  const secondsLeft = $derived(
    chosen && lockedUntil[chosen] ? Math.max(0, Math.ceil((lockedUntil[chosen] - now) / 1000)) : 0,
  );
  const lockedOut = $derived(secondsLeft > 0);

  // When the wait is over, put the cursor back in the PIN box.
  let wasLocked = false;
  $effect(() => {
    if (lockedOut) {
      wasLocked = true;
    } else if (wasLocked) {
      wasLocked = false;
      error = "";
      void tick().then(() => document.getElementById("pin")?.focus());
    }
  });

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!chosen) {
      error = "Choose who you are first.";
      return;
    }
    if (lockedOut) return;
    busy = true;
    error = "";
    try {
      onSignedIn(await api.login(chosen, pin));
    } catch (e) {
      pin = "";
      if (isCommandError(e) && e.retry_after && chosen) {
        lockedUntil = { ...lockedUntil, [chosen]: Date.now() + e.retry_after * 1000 };
        now = Date.now();
        error = "";
      } else {
        error = errorMessage(e);
      }
    } finally {
      busy = false;
    }
  }
</script>

<div class="gate">
  <form class="panel stack card" onsubmit={submit}>
    <p class="sign">{status.hotel_name}</p>
    <h1>Who's at the desk?</h1>
    <Field>
      <FieldLabel>Sign in as</FieldLabel>
      <RadioGroup value={chosen} onValueChange={(v) => (picked = v)} class="people">
        {#each status.users as u (u.id)}
          <label data-slot="label" class="person">
            <Radio value={u.id}><RadioIndicator /></Radio>
            {u.name}
          </label>
        {/each}
      </RadioGroup>
    </Field>
    <Field invalid={error !== "" || lockedOut} disabled={lockedOut}>
      <FieldLabel>PIN</FieldLabel>
      <Input
        id="pin"
        type="password"
        inputmode="numeric"
        autocomplete="off"
        bind:value={pin}
        disabled={lockedOut}
        required
      />
      <FieldError>{error}</FieldError>
    </Field>
    {#if lockedOut}
      <p class="wait" role="status">
        Too many wrong PINs. You can try again in
        <strong><span aria-live="off">{secondsLeft}</span> second{secondsLeft === 1 ? "" : "s"}</strong>.
      </p>
    {/if}
    <div>
      <Button type="submit" data-variant="solid" disabled={busy || lockedOut}>Sign in</Button>
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

  .sign {
    margin: 0;
    font-family: var(--font-sign);
    font-size: 1.1rem;
    color: var(--ink-muted);
  }

  .card :global(.people) {
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 2);
  }

  .wait {
    margin: 0;
    padding: calc(var(--ui-spacing) * 3);
    border-radius: var(--ui-radius);
    background: var(--brass-tint);
    border: 1px solid var(--brass);
  }

  .person {
    display: flex;
    align-items: center;
    gap: calc(var(--ui-spacing) * 2);
    font-size: 1rem;
  }
</style>
