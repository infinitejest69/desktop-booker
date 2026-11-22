<script lang="ts">
  // Housekeeping now lives on the Rooms page (the Cards view); keep old
  // links working.
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";

  onMount(() => void goto("/floor-plan?view=cards", { replace: true }));
</script>

<p class="muted" style="padding: 2rem">Opening housekeeping in Rooms…</p>
