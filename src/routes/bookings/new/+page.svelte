<script lang="ts">
  import { Button } from "@ui-sdk/svelte/button";
  import { toast } from "@ui-sdk/svelte/toast";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import BookingEditor from "#lib/booking/BookingEditor.svelte";
  import Breadcrumbs from "#lib/nav/Breadcrumbs.svelte";
  import type { BookingDetail } from "#lib/bindings/BookingDetail.ts";

  const chartFrom = page.url.searchParams.get("from");

  function saved(detail: BookingDetail) {
    const id = detail.booking.id;
    if (chartFrom) {
      void goto(`/chart?from=${chartFrom}&highlight=${id}`, { replace: true });
      toast.success(`Booked ${detail.booking.reference}`, {
        action: { label: "Open booking", onAction: () => goto(`/chart?from=${chartFrom}&booking=${id}`) },
      });
    } else {
      toast.success(`Booked ${detail.booking.reference}`);
      void goto(`/bookings/${id}`, { replace: true });
    }
  }
</script>

<div class="page">
  <Breadcrumbs
    items={chartFrom
      ? [{ label: "Booking calendar", href: `/chart?from=${chartFrom}` }, { label: "New booking" }]
      : [{ label: "Bookings", href: "/bookings" }, { label: "New booking" }]}
  />
  <div class="heading">
    <h1>New booking</h1>
    <Button onclick={() => history.back()}>Cancel</Button>
  </div>
  <p class="page-intro">Prices are worked out from the room type, rate rules and board.</p>
  <BookingEditor
    initialRoom={page.url.searchParams.get("room")}
    initialStart={page.url.searchParams.get("start")}
    onSaved={saved}
  />
</div>

<style>
  .heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    max-width: 52rem;
  }
</style>
