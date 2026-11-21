<script lang="ts">
  import { Toggle, ToggleGroup } from "@ui-sdk/svelte/toggle";
  import { goto } from "$app/navigation";

  // The two views of bookings: the calendar (rooms × nights) and the searchable list.
  // Each is its own page, so this switch just moves between them.
  let { current }: { current: "calendar" | "list" } = $props();

  const pages = { calendar: "/chart", list: "/bookings" } as const;
</script>

<ToggleGroup
  aria-label="View"
  value={[current]}
  deselectable={false}
  onValueChange={(value) => {
    const next = value[0] as keyof typeof pages | undefined;
    if (next && next !== current) void goto(pages[next]);
  }}
>
  <Toggle value="calendar">Calendar</Toggle>
  <Toggle value="list">List</Toggle>
</ToggleGroup>
