<script lang="ts">
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
  import { toast } from "@ui-sdk/svelte/toast";
  import { goto } from "$app/navigation";
  import MoneyField from "../components/MoneyField.svelte";
  import NumberField from "../components/NumberField.svelte";
  import TextField from "../components/TextField.svelte";
  import { api, errorMessage, newId } from "../api";
  import { formatDay, today } from "../dates";
  import { formatMoney } from "../format";
  import { useSession } from "../session/session.svelte";
  import type { ChargeCategory } from "../bindings/ChargeCategory";
  import type { Folio } from "../bindings/Folio";
  import type { FolioOwner } from "../bindings/FolioOwner";
  import type { InvoiceSettings } from "../bindings/InvoiceSettings";
  import type { PaymentMethod } from "../bindings/PaymentMethod";
  import type { TaxRate } from "../bindings/TaxRate";
  import { categoryLabel, methodLabel, percent } from "./labels";

  interface Props {
    owner: FolioOwner;
  }

  let { owner }: Props = $props();
  const session = useSession();

  let folio = $state<Folio | null>(null);
  let rates = $state<TaxRate[]>([]);
  let settings = $state<InvoiceSettings | null>(null);

  // Add an extra
  let description = $state("");
  let category = $state<ChargeCategory>("drink");
  let quantity = $state(1);
  let unitPrice = $state(0);
  // Take a payment
  let amount = $state(0);
  let method = $state<PaymentMethod>("card");
  let isDeposit = $state(false);
  let isRefund = $state(false);
  let reference = $state("");

  const extraCategories: ChargeCategory[] = ["food", "drink", "other"];
  const categoryItems = extraCategories.map((c) => ({ value: c, label: categoryLabel[c] }));
  const methodItems = (Object.keys(methodLabel) as PaymentMethod[]).map((m) => ({ value: m, label: methodLabel[m] }));
  const short: Intl.DateTimeFormatOptions = { day: "numeric", month: "short" };

  $effect(() => {
    const o = owner;
    Promise.all([api.getFolio(o), api.listTaxRates(), api.getInvoiceSettings()])
      .then(([f, r, s]) => {
        folio = f;
        rates = r;
        settings = s;
        amount = Math.max(f.totals.balance, 0);
      })
      .catch((error) => toast.error("Couldn't load the bill", { description: errorMessage(error) }));
  });

  /** The tax rate the hotel charges on a kind of charge. */
  function taxFor(c: ChargeCategory): number {
    if (!settings) return 0;
    const byCategory: Record<ChargeCategory, string | null> = {
      accommodation: settings.accommodation_rate,
      board: settings.board_rate,
      suite: settings.suite_rate,
      food: settings.food_rate,
      drink: settings.drink_rate,
      other: settings.other_rate,
    };
    return rates.find((r) => r.id === byCategory[c])?.basis_points ?? 0;
  }

  function updated(f: Folio) {
    folio = f;
    amount = Math.max(f.totals.balance, 0);
  }

  async function addExtra(event: SubmitEvent) {
    event.preventDefault();
    try {
      updated(
        await api.addCharge(owner, {
          id: newId(),
          date: today(),
          category,
          description,
          quantity,
          unit_price: unitPrice,
          tax_basis_points: taxFor(category),
        }),
      );
      toast.success(`Added ${description}`);
      description = "";
      quantity = 1;
      unitPrice = 0;
    } catch (error) {
      toast.error("Couldn't add the charge", { description: errorMessage(error) });
    }
  }

  async function remove(id: string, what: string) {
    try {
      updated(await api.removeCharge(id));
      toast.success(`Removed ${what}`);
    } catch (error) {
      toast.error("Couldn't remove the charge", { description: errorMessage(error) });
    }
  }

  async function takePayment(event: SubmitEvent) {
    event.preventDefault();
    try {
      updated(
        await api.addPayment(owner, {
          id: newId(),
          date: today(),
          amount: isRefund ? -amount : amount,
          method,
          is_deposit: isDeposit && !isRefund,
          reference,
        }),
      );
      toast.success(isRefund ? `Refunded ${formatMoney(amount)}` : `Recorded ${formatMoney(amount)}`);
      reference = "";
      isDeposit = false;
      isRefund = false;
    } catch (error) {
      toast.error("Couldn't record the payment", { description: errorMessage(error) });
    }
  }

  async function issue() {
    try {
      const invoice = await api.issueInvoice(owner);
      toast.success(`Issued invoice ${invoice.number}`);
      await goto(`/invoices/${invoice.id}`);
    } catch (error) {
      toast.error("Couldn't issue the invoice", { description: errorMessage(error) });
    }
  }
</script>

<section class="panel stack folio" aria-labelledby="folio-heading">
  <div class="head">
    <h2 id="folio-heading">Bill</h2>
    {#if folio}
      <Button data-variant="solid" onclick={issue} disabled={!folio.lines.length}>Issue invoice</Button>
    {/if}
  </div>

  {#if folio}
    {#if folio.lines.length}
      <table class="list">
        <thead>
          <tr><th>Date</th><th>Item</th><th class="num">Qty</th><th class="num">Price</th><th class="num">VAT</th><th class="num">Amount</th><th><span class="sr-only">Actions</span></th></tr>
        </thead>
        <tbody>
          {#each folio.lines as line (line.charge.id)}
            <tr>
              <td>{formatDay(line.charge.date, short)}</td>
              <td>{line.charge.description}</td>
              <td class="num">{line.charge.quantity}</td>
              <td class="num">{formatMoney(line.charge.unit_price)}</td>
              <td class="num">{percent(line.charge.tax_basis_points)}</td>
              <td class="num">{formatMoney(line.amounts.gross)}</td>
              <td class="actions">
                {#if !line.automatic && session.can("cancel_bookings")}
                  <Button onclick={() => remove(line.charge.id, line.charge.description)}>Remove</Button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <p class="muted">Nothing to charge yet.</p>
    {/if}

    <dl class="totals" aria-label="Bill totals">
      <div><dt>Total</dt><dd>{formatMoney(folio.totals.gross)}</dd></div>
      {#each folio.totals.by_rate.filter((r) => r.tax !== 0) as r (r.basis_points)}
        <div class="muted"><dt>VAT at {percent(r.basis_points)}{folio.prices_include_tax ? " (included)" : ""}</dt><dd>{formatMoney(r.tax)}</dd></div>
      {/each}
      <div><dt>Paid</dt><dd>{formatMoney(folio.totals.paid)}</dd></div>
      {#if folio.totals.balance >= 0}
        <div class="balance"><dt>Balance due</dt><dd>{formatMoney(folio.totals.balance)}</dd></div>
      {:else}
        <div class="balance credit"><dt>In credit (refund due)</dt><dd>{formatMoney(-folio.totals.balance)}</dd></div>
      {/if}
    </dl>

    {#if folio.payments.length}
      <h3>Payments</h3>
      <table class="list">
        <thead><tr><th>Date</th><th>How</th><th>Reference</th><th class="num">Amount</th></tr></thead>
        <tbody>
          {#each folio.payments as p (p.id)}
            <tr>
              <td>{formatDay(p.date, short)}</td>
              <td>{methodLabel[p.method]}{p.is_deposit ? ", deposit" : ""}{p.amount < 0 ? ", refund" : ""}</td>
              <td>{p.reference}</td>
              <td class="num">{formatMoney(p.amount)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}

    <div class="forms">
      <form class="stack" onsubmit={addExtra} aria-labelledby="extra-heading">
        <h3 id="extra-heading">Add to the bill</h3>
        <TextField label="Item" bind:value={description} required placeholder="Bar, dinner, newspaper…" />
        <div class="row align-top">
          <Field>
            <FieldLabel>Kind</FieldLabel>
            <Select items={categoryItems} value={category} onValueChange={(v) => v && (category = v as ChargeCategory)}>
              <SelectTrigger><SelectValue /></SelectTrigger>
              <SelectContent>
                {#each categoryItems as c (c.value)}
                  <SelectItem value={c.value}>{c.label}<SelectItemIndicator /></SelectItem>
                {/each}
              </SelectContent>
            </Select>
          </Field>
          <NumberField label="Quantity" bind:value={quantity} min={1} max={999} />
        </div>
        <MoneyField label="Price each" bind:value={unitPrice} min={-100000} />
        <div><Button type="submit">Add to bill</Button></div>
      </form>

      <form class="stack" onsubmit={takePayment} aria-labelledby="payment-heading">
        <h3 id="payment-heading">Record a payment</h3>
        <MoneyField label="Amount" bind:value={amount} />
        <Field>
          <FieldLabel>How it was paid</FieldLabel>
          <Select items={methodItems} value={method} onValueChange={(v) => v && (method = v as PaymentMethod)}>
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              {#each methodItems as m (m.value)}
                <SelectItem value={m.value}>{m.label}<SelectItemIndicator /></SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </Field>
        <TextField label="Reference" bind:value={reference} placeholder="Card slip, transfer reference…" />
        <div class="row">
          <label data-slot="label" class="switch-row">
            Deposit
            <Switch bind:checked={isDeposit} disabled={isRefund}><SwitchThumb /></Switch>
          </label>
          <label data-slot="label" class="switch-row">
            This is a refund
            <Switch bind:checked={isRefund}><SwitchThumb /></Switch>
          </label>
        </div>
        <div><Button type="submit">{isRefund ? "Record refund" : "Record payment"}</Button></div>
      </form>
    </div>

    {#if folio.invoices.length}
      <h3>Invoices</h3>
      <ul class="invoices">
        {#each folio.invoices as inv (inv.id)}
          <li>
            <a href={`/invoices/${inv.id}`}>{inv.number}</a>
            <span class="muted">{formatDay(inv.issued_on, { day: "numeric", month: "short", year: "numeric" })}, {formatMoney(inv.gross)}</span>
          </li>
        {/each}
      </ul>
    {/if}
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</section>

<style>
  .folio {
    margin-top: calc(var(--ui-spacing) * 4);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 3);
  }

  .head h2,
  h3 {
    margin: 0;
  }

  h3 {
    font-size: 1rem;
  }

  .num {
    text-align: right;
    font-variant-numeric: tabular-nums lining-nums;
  }

  .totals {
    margin: 0;
    margin-left: auto;
    min-width: 18rem;
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 1);
  }

  .totals div {
    display: flex;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 4);
  }

  .totals dt,
  .totals dd {
    margin: 0;
  }

  .totals dd {
    font-variant-numeric: tabular-nums lining-nums;
  }

  .balance {
    padding-top: calc(var(--ui-spacing) * 1);
    border-top: 1px solid var(--ui-border);
    font-weight: 800;
    font-size: 1.1rem;
  }

  .forms {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
    gap: calc(var(--ui-spacing) * 6);
    padding-top: calc(var(--ui-spacing) * 3);
    border-top: 1px solid var(--ui-border);
  }

  .switch-row {
    display: flex;
    align-items: center;
    gap: calc(var(--ui-spacing) * 2);
  }

  .invoices {
    margin: 0;
    padding-left: 1.2rem;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
