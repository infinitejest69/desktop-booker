<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { toast } from "@ui-sdk/svelte/toast";
  import { page } from "$app/state";
  import { api, errorMessage } from "#lib/api.ts";
  import type { Invoice } from "#lib/bindings/Invoice.ts";
  import type { PropertySettings } from "#lib/bindings/PropertySettings.ts";
  import { formatDay } from "#lib/dates.ts";
  import { formatMoney } from "#lib/format.ts";
  import { methodLabel, percent } from "#lib/invoices/labels.ts";

  let invoice = $state<Invoice | null>(null);
  let hotel = $state<PropertySettings | null>(null);

  $effect(() => {
    const id = page.params.id;
    if (!id) return;
    Promise.all([api.getInvoice(id), api.getSettings()])
      .then(([inv, s]) => {
        invoice = inv;
        hotel = s;
      })
      .catch((error) => toast.error("Couldn't load the invoice", { description: errorMessage(error) }));
  });

  const ownerHref = $derived(
    invoice
      ? invoice.owner.kind === "booking"
        ? `/bookings/${invoice.owner.id}`
        : invoice.owner.kind === "event"
          ? `/events/${invoice.owner.id}`
          : `/suites/booking?id=${invoice.owner.id}`
      : "/",
  );
  const money = (m: number) => formatMoney(m, hotel?.currency ?? "GBP");
  const long: Intl.DateTimeFormatOptions = { day: "numeric", month: "long", year: "numeric" };
</script>

<div class="page">
  {#if invoice && hotel}
    <div class="row no-print toolbar">
      <a class="link-button" href={ownerHref}>Back to {invoice.title}</a>
      <!-- The system print dialog can also save the page as a PDF (see ADR 0004). -->
      <Button data-variant="solid" onclick={() => window.print()}>Print or save as PDF</Button>
    </div>

    <article class="invoice" aria-label={`Invoice ${invoice.number}`}>
      <header class="top">
        <div>
          <p class="hotel">{hotel.hotel_name}</p>
          {#if hotel.address}<p class="pre">{hotel.address}</p>{/if}
          {#if hotel.phone}<p>{hotel.phone}</p>{/if}
          {#if hotel.email}<p>{hotel.email}</p>{/if}
        </div>
        <div class="meta">
          <h1>Invoice</h1>
          <p><strong>{invoice.number}</strong></p>
          <p>{formatDay(invoice.issued_on, long)}</p>
          <p class="muted">{invoice.title}</p>
        </div>
      </header>

      <section class="to" aria-label="Billed to">
        <p class="label">To</p>
        <p><strong>{invoice.bill_to}</strong></p>
        {#if invoice.bill_address}<p class="pre">{invoice.bill_address}</p>{/if}
      </section>

      <table class="lines">
        <thead>
          <tr><th>Date</th><th>Item</th><th class="num">Qty</th><th class="num">Price</th><th class="num">VAT</th><th class="num">Amount</th></tr>
        </thead>
        <tbody>
          {#each invoice.lines as line (line.charge.id)}
            <tr>
              <td>{formatDay(line.charge.date, { day: "numeric", month: "short" })}</td>
              <td>{line.charge.description}</td>
              <td class="num">{line.charge.quantity}</td>
              <td class="num">{money(line.charge.unit_price)}</td>
              <td class="num">{percent(line.charge.tax_basis_points)}</td>
              <td class="num">{money(line.amounts.gross)}</td>
            </tr>
          {/each}
        </tbody>
      </table>

      <div class="sums">
        <table class="vat" aria-label="VAT summary">
          <thead><tr><th>VAT rate</th><th class="num">Net</th><th class="num">VAT</th></tr></thead>
          <tbody>
            {#each invoice.totals.by_rate as r (r.basis_points)}
              <tr><td>{percent(r.basis_points)}</td><td class="num">{money(r.net)}</td><td class="num">{money(r.tax)}</td></tr>
            {/each}
          </tbody>
        </table>
        <dl class="totals">
          <div><dt>Net</dt><dd>{money(invoice.totals.net)}</dd></div>
          <div><dt>VAT</dt><dd>{money(invoice.totals.tax)}</dd></div>
          <div class="strong"><dt>Total</dt><dd>{money(invoice.totals.gross)}</dd></div>
          {#each invoice.payments as p (p.id)}
            <div class="muted">
              <dt>{p.amount < 0 ? "Refund" : p.is_deposit ? "Deposit" : "Paid"}, {methodLabel[p.method].toLowerCase()}, {formatDay(p.date, { day: "numeric", month: "short" })}</dt>
              <dd>{money(-p.amount)}</dd>
            </div>
          {/each}
          <div class="strong balance">
            <dt>{invoice.totals.balance >= 0 ? "Balance due" : "In credit"}</dt>
            <dd>{money(Math.abs(invoice.totals.balance))}</dd>
          </div>
        </dl>
      </div>

      {#if invoice.footer}
        <footer class="pre foot">{invoice.footer}</footer>
      {/if}
    </article>
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</div>

<style>
  .toolbar {
    justify-content: space-between;
    margin-bottom: calc(var(--ui-spacing) * 4);
  }

  /* Always black on white, like the paper it ends up on. */
  .invoice {
    max-width: 52rem;
    padding: calc(var(--ui-spacing) * 8);
    background: #fff;
    color: #1d2125;
    border: 1px solid var(--ui-border);
    border-radius: 8px;
  }

  .invoice p {
    margin: 0 0 2px;
  }

  .top {
    display: flex;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 6);
    margin-bottom: calc(var(--ui-spacing) * 8);
  }

  .hotel {
    font-family: var(--font-sign);
    font-size: 1.6rem;
  }

  .meta {
    text-align: right;
  }

  .meta h1 {
    margin: 0 0 calc(var(--ui-spacing) * 2);
    font-size: 1.6rem;
  }

  .to {
    margin-bottom: calc(var(--ui-spacing) * 6);
  }

  .label,
  .invoice .muted {
    color: #5b6470;
  }

  .label {
    font-size: 0.8rem;
  }

  .pre {
    white-space: pre-line;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    padding: 6px 8px;
    text-align: left;
    border-bottom: 1px solid #dde1e6;
  }

  th {
    font-size: 0.8rem;
    color: #5b6470;
    font-weight: 600;
  }

  .num {
    text-align: right;
    font-variant-numeric: tabular-nums lining-nums;
  }

  .sums {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: calc(var(--ui-spacing) * 8);
    margin-top: calc(var(--ui-spacing) * 6);
  }

  .vat {
    max-width: 18rem;
  }

  .totals {
    margin: 0;
    min-width: 18rem;
  }

  .totals div {
    display: flex;
    justify-content: space-between;
    gap: calc(var(--ui-spacing) * 4);
    padding: 3px 0;
  }

  .totals dt,
  .totals dd {
    margin: 0;
  }

  .totals dd {
    font-variant-numeric: tabular-nums lining-nums;
  }

  .strong {
    font-weight: 800;
  }

  .balance {
    border-top: 2px solid #1d2125;
    margin-top: 4px;
    padding-top: 6px;
    font-size: 1.1rem;
  }

  .foot {
    margin-top: calc(var(--ui-spacing) * 10);
    padding-top: calc(var(--ui-spacing) * 3);
    border-top: 1px solid #dde1e6;
    font-size: 0.85rem;
    color: #5b6470;
  }

  @media print {
    .invoice {
      border: 0;
      padding: 0;
      max-width: none;
    }
  }
</style>
