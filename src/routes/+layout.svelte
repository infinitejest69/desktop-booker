<script lang="ts">
  import "@ui-sdk/core/web.css";
  import "../app.css";
  import { page } from "$app/state";
  import { OverlayHost } from "@ui-sdk/svelte/overlays";
  import type { Snippet } from "svelte";

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();

  const nav = [
    { href: "/", label: "Floor plan" },
    { href: "/setup/property", label: "Hotel details" },
    { href: "/setup/floors", label: "Floors" },
    { href: "/setup/room-types", label: "Room types" },
    { href: "/setup/layout", label: "Layout editor" },
  ];
</script>

<div class="shell">
  <nav aria-label="Main">
    <p class="brand">DesktopBooker</p>
    <ul>
      {#each nav as item (item.href)}
        <li>
          <a href={item.href} aria-current={page.url.pathname === item.href ? "page" : undefined}
            >{item.label}</a
          >
        </li>
      {/each}
    </ul>
  </nav>
  <main>
    {@render children()}
  </main>
</div>
<OverlayHost />

<style>
  .shell {
    display: grid;
    grid-template-columns: 13rem 1fr;
    min-height: 100vh;
  }

  nav {
    background: var(--app-sidebar);
    border-inline-end: 1px solid var(--ui-border);
    padding: calc(var(--ui-spacing) * 5) calc(var(--ui-spacing) * 3);
  }

  .brand {
    font-weight: 700;
    font-size: 1rem;
    margin: 0 calc(var(--ui-spacing) * 3) calc(var(--ui-spacing) * 5);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: calc(var(--ui-spacing) * 1);
  }

  a {
    display: block;
    padding: calc(var(--ui-spacing) * 2) calc(var(--ui-spacing) * 3);
    border-radius: var(--ui-radius);
    color: inherit;
    text-decoration: none;
  }

  a:hover {
    background: var(--ui-muted);
  }

  a:focus-visible {
    outline: 2px solid var(--ui-ring-border);
    outline-offset: 1px;
  }

  a[aria-current="page"] {
    background: var(--ui-primary);
    color: var(--ui-primary-foreground);
    font-weight: 600;
  }

  main {
    min-width: 0;
  }
</style>
