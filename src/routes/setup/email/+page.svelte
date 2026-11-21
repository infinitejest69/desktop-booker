<script lang="ts">
  import { onMount } from "svelte";
  import { Button } from "@ui-sdk/svelte/button";
  import { Field, FieldDescription, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";
  import { Textarea } from "@ui-sdk/svelte/textarea";
  import { toast } from "@ui-sdk/svelte/toast";
  import NumberField from "#lib/components/NumberField.svelte";
  import TextField from "#lib/components/TextField.svelte";
  import { api, errorMessage, type EmailSetup } from "#lib/api.ts";
  import type { EmailKind } from "#lib/bindings/EmailKind.ts";
  import type { EmailLogEntry } from "#lib/bindings/EmailLogEntry.ts";
  import type { EmailSettings } from "#lib/bindings/EmailSettings.ts";
  import type { EmailTemplate } from "#lib/bindings/EmailTemplate.ts";

  const kindLabel: Record<EmailKind, string> = {
    confirmation: "Booking confirmation",
    amendment: "Booking changed",
    cancellation: "Booking cancelled",
  };
  const placeholders = [
    "guest_name",
    "first_name",
    "reference",
    "arrival",
    "departure",
    "nights",
    "rooms",
    "total",
    "hotel_name",
    "hotel_phone",
  ];

  let setup = $state<EmailSetup | null>(null);
  let settings = $state<EmailSettings | null>(null);
  let templates = $state<EmailTemplate[]>([]);
  let password = $state("");
  let testTo = $state("");
  let sending = $state(false);
  let log = $state<EmailLogEntry[]>([]);

  async function load() {
    try {
      setup = await api.getEmailSetup();
      settings = $state.snapshot(setup.settings);
      templates = $state.snapshot(setup.templates);
      log = await api.listEmailLog();
    } catch (error) {
      toast.error("Couldn't load the email settings", { description: errorMessage(error) });
    }
  }
  onMount(load);

  function useGmail() {
    if (!settings) return;
    settings.host = "smtp.gmail.com";
    settings.port = 587;
    if (settings.username && !settings.from_address) settings.from_address = settings.username;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!settings) return;
    try {
      await api.saveEmailSettings($state.snapshot(settings), password || null);
      toast.success("Saved the email settings", {
        description: password ? "The password is in this computer's keychain." : undefined,
      });
      password = "";
      await load();
    } catch (error) {
      toast.error("Couldn't save", { description: errorMessage(error) });
    }
  }

  async function sendTest(event: SubmitEvent) {
    event.preventDefault();
    sending = true;
    try {
      await api.sendTestEmail(testTo);
      toast.success(`Sent a test email to ${testTo}`);
    } catch (error) {
      toast.error("The test email wasn't sent", { description: errorMessage(error) });
    } finally {
      sending = false;
      log = await api.listEmailLog().catch(() => log);
    }
  }

  async function saveTemplate(t: EmailTemplate) {
    try {
      await api.saveEmailTemplate($state.snapshot(t));
      toast.success(`Saved the ${kindLabel[t.kind].toLowerCase()} email`);
    } catch (error) {
      toast.error("Couldn't save the email", { description: errorMessage(error) });
    }
  }

  const when = (at: string) =>
    new Date(at).toLocaleString("en-GB", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
</script>

<div class="page">
  <h1>Email</h1>
  <p class="page-intro">Booking emails go out through the hotel's own email account, so replies come back to you.</p>

  {#if setup && settings}
    <form class="panel stack" onsubmit={save} aria-label="Email account">
      <div class="head">
        <h2>Email account</h2>
        <Button onclick={useGmail}>Use Gmail</Button>
      </div>
      <div class="row align-top">
        <TextField label="Mail server" bind:value={settings.host} required placeholder="smtp.gmail.com" />
        <NumberField label="Port" bind:value={settings.port} min={1} max={65535} description="587 for Gmail" />
      </div>
      <TextField label="User name" bind:value={settings.username} required placeholder="yourhotel@gmail.com" />
      <Field>
        <FieldLabel>Password</FieldLabel>
        <Input
          type="password"
          bind:value={password}
          autocomplete="new-password"
          placeholder={setup.password_saved ? "Saved (type to replace)" : ""}
        />
        <FieldDescription>
          {setup.password_saved ? "A password is saved in this computer's keychain." : "No password saved yet."}
          For Gmail, turn on 2-step verification and create an app password at myaccount.google.com/apppasswords;
          your normal password won't work.
        </FieldDescription>
      </Field>
      <div class="row align-top">
        <TextField
          label="From address"
          type="email"
          bind:value={settings.from_address}
          required
          description="Guests see this and reply to it"
        />
        <TextField label="From name" bind:value={settings.from_name} placeholder="The Grange" />
      </div>
      <div><Button type="submit" data-variant="solid">Save email account</Button></div>
    </form>

    <form class="panel stack block" onsubmit={sendTest} aria-label="Send a test email">
      <h2>Send a test</h2>
      <div class="row align-end">
        <TextField label="Send a test to" type="email" bind:value={testTo} required placeholder="you@example.com" />
        <Button type="submit" disabled={sending}>{sending ? "Sending…" : "Send test email"}</Button>
      </div>
    </form>

    <section class="panel stack block" aria-labelledby="templates-heading">
      <h2 id="templates-heading">What the emails say</h2>
      <p class="muted" style="margin: 0">
        These are filled in for each booking:
        {#each placeholders as p, i (p)}{i ? ", " : ""}<code>{`{${p}}`}</code>{/each}.
      </p>
      {#each templates as t (t.kind)}
        <fieldset class="stack template">
          <legend>{kindLabel[t.kind]}</legend>
          <TextField label="Subject" bind:value={t.subject} required />
          <Field>
            <FieldLabel>Message</FieldLabel>
            <Textarea bind:value={t.body} rows={9} />
          </Field>
          <div><Button onclick={() => saveTemplate(t)}>Save {kindLabel[t.kind].toLowerCase()}</Button></div>
        </fieldset>
      {/each}
    </section>

    <section class="panel block" aria-labelledby="log-heading">
      <h2 id="log-heading">Recent emails</h2>
      {#if log.length}
        <table class="list">
          <thead><tr><th>When</th><th>To</th><th>Subject</th><th>Result</th></tr></thead>
          <tbody>
            {#each log as e, i (i)}
              <tr>
                <td>{when(e.at)}</td>
                <td>{e.to}</td>
                <td>{e.subject}</td>
                <td>{e.sent ? "Sent" : `Failed: ${e.error}`}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="muted">No emails sent yet.</p>
      {/if}
    </section>
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</div>

<style>
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 3);
  }

  .head h2 {
    margin: 0;
  }

  .block {
    margin-top: calc(var(--ui-spacing) * 4);
  }

  .align-end {
    align-items: flex-end;
  }

  .template {
    margin: 0;
    padding: calc(var(--ui-spacing) * 3);
    border: 1px solid var(--ui-border);
    border-radius: 8px;
  }

  legend {
    font-weight: 700;
  }

  code {
    font-size: 0.85em;
  }
</style>
