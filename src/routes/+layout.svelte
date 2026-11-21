<script lang="ts">
  import "@fontsource-variable/atkinson-hyperlegible-next";
  import "@fontsource/young-serif";
  import "@ui-sdk/core/web.css";
  import "../app.css";
  import { afterNavigate } from "$app/navigation";
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import { api } from "#lib/api.ts";
  import { Button } from "@ui-sdk/svelte/button";
  import {
    Collapsible,
    CollapsiblePanel,
    CollapsibleTrigger,
  } from "@ui-sdk/svelte/collapsible";
  import { OverlayHost } from "@ui-sdk/svelte/overlays";
  import type { Snippet } from "svelte";

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();

  // How far into this session's history we are, so Back can be disabled at the start.
  let depth = $state(0);
  let ahead = $state(0);

  afterNavigate((nav) => {
    if (nav.type === "enter") {
      depth = 0;
      ahead = 0;
    } else if (nav.type === "popstate") {
      const delta = nav.delta ?? -1;
      depth = Math.max(depth + delta, 0);
      ahead = Math.max(ahead - delta, 0);
    } else if (!nav.willUnload) {
      depth += 1;
      ahead = 0;
    }
  });

  function back() {
    if (depth > 0) history.back();
  }

  function forward() {
    if (ahead > 0) history.forward();
  }

  onMount(() => {
    // Mouse side buttons (3 = back, 4 = forward) and Alt+Left / Alt+Right.
    const onMouse = (e: MouseEvent) => {
      if (e.button === 3) back();
      if (e.button === 4) forward();
    };
    const onKey = (e: KeyboardEvent) => {
      if (!e.altKey || e.ctrlKey || e.metaKey) return;
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        back();
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        forward();
      }
    };
    window.addEventListener("mouseup", onMouse);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mouseup", onMouse);
      window.removeEventListener("keydown", onKey);
    };
  });

  function isCurrent(href: string): boolean {
    const path = page.url.pathname;
    return href === "/" ? path === "/" : path === href || path.startsWith(`${href}/`);
  }

  const main = [
    { href: "/", label: "Dashboard" },
    { href: "/chart", label: "Tariff chart" },
    { href: "/bookings", label: "Bookings" },
    { href: "/floor-plan", label: "Floor plan" },
  ];
  const hotel = [
    { href: "/setup/property", label: "Hotel details" },
    { href: "/setup/floors", label: "Floors" },
    { href: "/setup/room-types", label: "Room types" },
    { href: "/setup/rates", label: "Rates" },
    { href: "/setup/layout", label: "Layout editor" },
  ];

  let hotelOpen = $state(false);
  let hotelName = $state("");

  // Refresh the sign after any navigation, so a renamed hotel shows straight away.
  $effect(() => {
    void page.url.pathname;
    api
      .getSettings()
      .then((s) => (hotelName = s.hotel_name))
      .catch(() => {});
  });
  $effect(() => {
    if (page.url.pathname.startsWith("/setup/")) hotelOpen = true;
  });
</script>

<div class="shell">
  <nav aria-label="Main">
    <a class="sign" href="/">{hotelName || "Your hotel"}</a>
    <ul>
      {#each main as item (item.href)}
        <li>
          <a href={item.href} aria-current={isCurrent(item.href) ? "page" : undefined}>{item.label}</a>
        </li>
      {/each}
    </ul>
    <Collapsible bind:open={hotelOpen} class="group">
      <CollapsibleTrigger class="group-trigger">
        My hotel
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg>
      </CollapsibleTrigger>
      <CollapsiblePanel>
        <ul class="sub">
          {#each hotel as item (item.href)}
            <li>
              <a href={item.href} aria-current={isCurrent(item.href) ? "page" : undefined}>{item.label}</a>
            </li>
          {/each}
        </ul>
      </CollapsiblePanel>
    </Collapsible>
  </nav>
  <div class="content">
    <div class="topbar" role="toolbar" aria-label="History">
      <Button aria-label="Back" aria-keyshortcuts="Alt+ArrowLeft" disabled={depth === 0} onclick={back}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 18l-6-6 6-6" /></svg>
      </Button>
      <Button aria-label="Forward" aria-keyshortcuts="Alt+ArrowRight" disabled={ahead === 0} onclick={forward}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 18l6-6-6-6" /></svg>
      </Button>
    </div>
    <main>
      {@render children()}
    </main>
  </div>
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

  .sign {
    display: block;
    margin: 0 0 calc(var(--ui-spacing) * 6);
    padding: calc(var(--ui-spacing) * 1) calc(var(--ui-spacing) * 3);
    font-family: var(--font-sign);
    font-size: 1.3rem;
    line-height: 1.2;
    color: var(--ink);
    text-decoration: none;
    overflow-wrap: anywhere;
  }

  .sign:hover {
    background: transparent;
    color: var(--heather);
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

  nav :global(.group) {
    margin-top: calc(var(--ui-spacing) * 5);
    padding-top: calc(var(--ui-spacing) * 3);
    border-top: 1px solid var(--ui-border);
  }

  nav :global(.group-trigger) {
    display: flex;
    width: 100%;
    align-items: center;
    justify-content: space-between;
    padding: calc(var(--ui-spacing) * 2) calc(var(--ui-spacing) * 3);
    border: 0;
    border-radius: var(--ui-radius);
    background: transparent;
    color: var(--ui-muted-foreground);
    font: inherit;
    font-weight: 700;
    font-size: 0.9375rem;
    cursor: pointer;
  }

  nav :global(.group-trigger:hover) {
    background: var(--ui-muted);
  }

  nav :global(.group-trigger:focus-visible) {
    outline: 2px solid var(--ui-ring-border);
    outline-offset: 1px;
  }

  nav :global(.group-trigger svg) {
    width: 1rem;
    height: 1rem;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    transition: transform 150ms;
  }

  nav :global(.group-trigger[data-open] svg) {
    transform: rotate(180deg);
  }

  .sub {
    margin-top: calc(var(--ui-spacing) * 1);
  }

  .sub a:not([aria-current]) {
    color: var(--ui-foreground);
  }

  a:focus-visible {
    outline: 2px solid var(--ui-ring-border);
    outline-offset: 1px;
  }

  a[aria-current="page"] {
    background: var(--heather);
    color: var(--on-heather);
    font-weight: 700;
  }

  .content {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .topbar {
    position: sticky;
    top: 0;
    z-index: 5;
    display: flex;
    gap: calc(var(--ui-spacing) * 2);
    padding: calc(var(--ui-spacing) * 2) calc(var(--ui-spacing) * 8);
    background: var(--ground);
    border-bottom: 1px solid var(--rule);
  }

  .topbar svg {
    width: 1.1rem;
    height: 1.1rem;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  main {
    min-width: 0;
  }
</style>
