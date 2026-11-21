<script lang="ts">
  import type { Floor } from "../bindings/Floor";
  import type { Space } from "../bindings/Space";

  interface Props {
    floor: Floor;
    spaces: Space[];
    typeCodes: Map<string, string>;
    cell?: number;
  }

  let { floor, spaces, typeCodes, cell = 28 }: Props = $props();

  function status(space: Space): string {
    return space.out_of_service ? "Out of service" : "Available";
  }
</script>

<div class="scroll">
  <ul
    class="plan"
    aria-label={`${floor.name} rooms`}
    style:width={`${floor.grid_cols * cell}px`}
    style:height={`${floor.grid_rows * cell}px`}
    style:--cell={`${cell}px`}
  >
    {#each spaces as space (space.id)}
      <li
        class="tile"
        data-kind={space.kind}
        data-status={space.out_of_service ? "out-of-service" : "available"}
        style:left={`${space.rect.x * cell}px`}
        style:top={`${space.rect.y * cell}px`}
        style:width={`${space.rect.w * cell}px`}
        style:height={`${space.rect.h * cell}px`}
      >
        <span class="name">{space.name}</span>
        <span class="meta">
          {space.kind === "suite" ? "Suite" : (typeCodes.get(space.room_type_id ?? "") ?? "")}
        </span>
        <span class="visually-hidden">, {status(space)}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  .scroll {
    overflow: auto;
    padding: calc(var(--ui-spacing) * 3);
    border: 1px solid var(--ui-border);
    border-radius: var(--ui-radius);
  }

  .plan {
    position: relative;
    list-style: none;
    margin: 0;
    padding: 0;
    background-image:
      linear-gradient(to right, var(--ui-border) 1px, transparent 1px),
      linear-gradient(to bottom, var(--ui-border) 1px, transparent 1px);
    background-size: var(--cell) var(--cell);
    outline: 1px solid var(--ui-border);
  }

  .tile {
    position: absolute;
    box-sizing: border-box;
    border: 2px solid var(--ui-background);
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    overflow: hidden;
  }

  .tile[data-status="available"] {
    background: light-dark(oklch(0.9 0.06 155), oklch(0.36 0.06 155));
  }

  .tile[data-kind="suite"][data-status="available"] {
    background: light-dark(oklch(0.92 0.06 80), oklch(0.38 0.06 80));
  }

  .tile[data-status="out-of-service"] {
    background: light-dark(oklch(0.88 0 0), oklch(0.32 0 0));
    background-image: repeating-linear-gradient(
      45deg,
      transparent 0 6px,
      light-dark(oklch(0 0 0 / 0.1), oklch(1 0 0 / 0.12)) 6px 12px
    );
  }

  .name {
    font-weight: 600;
  }

  .meta {
    font-size: 0.75rem;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
