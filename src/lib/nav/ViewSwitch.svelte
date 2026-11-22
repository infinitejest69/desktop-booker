<script lang="ts">
  import { Toggle, ToggleGroup } from "@ui-sdk/svelte/toggle";
  import { goto } from "$app/navigation";

  // Switches between pages that are views of the same thing, such as the booking
  // calendar and the bookings list. Each view is its own page; this moves between them.
  type View = { label: string; href: string };
  let { views, current, label = "View" }: { views: View[]; current: string; label?: string } = $props();
</script>

<ToggleGroup
  aria-label={label}
  value={[current]}
  deselectable={false}
  onValueChange={(value) => {
    const next = views.find((v) => v.label === value[0]);
    if (next && next.label !== current) void goto(next.href);
  }}
>
  {#each views as view (view.href)}
    <Toggle value={view.label}>{view.label}</Toggle>
  {/each}
</ToggleGroup>
