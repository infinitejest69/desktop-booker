<script lang="ts">
  interface Crumb {
    label: string;
    href?: string;
  }

  interface Props {
    items: Crumb[];
  }

  let { items }: Props = $props();
</script>

<nav aria-label="Breadcrumb" class="crumbs">
  <ol>
    {#each items as item, i (item.label)}
      <li>
        {#if item.href && i < items.length - 1}
          <a href={item.href}>{item.label}</a>
        {:else}
          <span aria-current="page">{item.label}</span>
        {/if}
      </li>
    {/each}
  </ol>
</nav>

<style>
  .crumbs ol {
    display: flex;
    flex-wrap: wrap;
    gap: calc(var(--ui-spacing) * 2);
    list-style: none;
    margin: 0 0 calc(var(--ui-spacing) * 2);
    padding: 0;
    color: var(--ui-muted-foreground);
  }

  .crumbs li + li::before {
    content: "›";
    margin-inline-end: calc(var(--ui-spacing) * 2);
  }

  .crumbs a {
    color: inherit;
  }

  .crumbs [aria-current] {
    color: var(--ui-foreground);
  }
</style>
