<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let version = $state("…");

  onMount(async () => {
    try {
      version = await invoke<string>("app_version");
    } catch {
      version = "unavailable (not running inside Tauri)";
    }
  });
</script>

<main>
  <h1>DesktopBooker</h1>
  <p>Hotel bookings for rooms, function suites and events.</p>
  <p class="muted">Core version: {version}</p>
</main>

<style>
  :root {
    font-family: system-ui, sans-serif;
    color: #1d1d1f;
    background: #f6f6f4;
  }
  main {
    max-width: 40rem;
    margin: 4rem auto;
    padding-inline: 1rem;
  }
  .muted {
    color: #6b6b6b;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      color: #ececec;
      background: #1c1c1e;
    }
    .muted {
      color: #9a9a9a;
    }
  }
</style>
