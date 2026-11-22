<script lang="ts">
  import type { RoomFeature } from "../bindings/RoomFeature";
  import { featureLabel, roomFeatures } from "./features";

  // A room's features as small line icons, in a fixed order. Each icon has its
  // name as a tooltip and for screen readers.
  interface Props {
    features: readonly RoomFeature[];
    /** Icon size in pixels. */
    size?: number;
  }

  let { features, size = 16 }: Props = $props();

  const shown = $derived(roomFeatures.filter((f) => features.includes(f)));
</script>

{#if shown.length}
  <span class="features" role="list" aria-label="Room features">
    {#each shown as feature (feature)}
      <span class="feature" role="listitem" title={featureLabel[feature]} data-feature={feature}>
        <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true">
          {#if feature === "wifi"}
            <path d="M2.5 9a14 14 0 0 1 19 0" />
            <path d="M5.5 12.5a9.5 9.5 0 0 1 13 0" />
            <path d="M8.75 16a5 5 0 0 1 6.5 0" />
            <circle cx="12" cy="19.5" r="1" class="fill" />
          {:else if feature === "pets_allowed"}
            <circle cx="6" cy="10" r="1.8" class="fill" />
            <circle cx="9.5" cy="5.5" r="1.8" class="fill" />
            <circle cx="14.5" cy="5.5" r="1.8" class="fill" />
            <circle cx="18" cy="10" r="1.8" class="fill" />
            <path d="M7.5 17.5c0-3 2-5.5 4.5-5.5s4.5 2.5 4.5 5.5c0 1.8-1.8 2.5-4.5 2.5s-4.5-.7-4.5-2.5z" class="fill" />
          {:else if feature === "accessible"}
            <circle cx="12" cy="3.5" r="1.8" class="fill" />
            <path d="M12 7v6.5h5l2.5 5.5" />
            <path d="M12 10h4" />
            <path d="M8.5 10.8a5.5 5.5 0 1 0 7.2 7.4" />
          {:else if feature === "smoking"}
            <path d="M2.5 15.5h13v3.5h-13z" />
            <path d="M18.5 15.5v3.5" />
            <path d="M21.5 15.5v3.5" />
            <path d="M18.5 12c0-2 2.5-2 2.5-4.5S19 5 19 3" />
          {/if}
        </svg>
        <span class="visually-hidden">{featureLabel[feature]}</span>
      </span>
    {/each}
  </span>
{/if}

<style>
  .features {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.3em;
    line-height: 1;
  }

  .feature {
    display: inline-flex;
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  svg .fill {
    fill: currentColor;
    stroke: none;
  }

  /* The name is for screen readers only; sighted users get the icon and its tooltip. */
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
    border: 0;
  }
</style>
