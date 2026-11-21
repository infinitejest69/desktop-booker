import {
  byCell,
  createGridFocus,
  definePrimitive,
  selects,
  type GridCell,
} from "@ui-sdk/core";

export type TariffChartOptions = {
  rows?: number;
  cols?: number;
  onActivate?: (cell: GridCell, event?: Event) => void;
};

// Rooms are rows, nights are columns. The grid has one tab stop; Enter, Space
// or a click "activates" a cell, and the page decides what that means.
export const tariffChart = definePrimitive({
  name: "tariff-chart",
  options: { rows: "number", cols: "number", onActivate: "callback" },
  state: (_options: TariffChartOptions) => ({ active: null as GridCell | null }),
  api: (context) => {
    const grid = createGridFocus(context, {
      key: "active",
      rows: () => context.options().rows ?? 0,
      cols: () => context.options().cols ?? 0,
      pageSize: () => 10,
    });
    return {
      grid,
      activate(cell: GridCell, event?: Event) {
        grid.focus(cell, { reason: "item-press", event });
        context.options().onActivate?.(cell, event);
      },
    };
  },
  agent: { actions: ["activate"] },
  slots: (slot) => ({
    root: slot({
      role: "grid",
      keys: { none: "Focus stays on a cell, whose keydown handles them." },
    }),
    row: slot({ role: "row" }),
    header: slot({ role: "columnheader" }),
    rowHeader: slot({ role: "rowheader" }),
    cell: slot<{ id: string; row: number; col: number }>({
      props: { id: "string", row: "number", col: "number" },
      role: "gridcell",
      focus: "roving",
      selects: selects({ active: byCell }),
      keys: [
        "ArrowUp",
        "ArrowDown",
        "ArrowLeft",
        "ArrowRight",
        "Home",
        "End",
        "mod+Home",
        "mod+End",
        "PageUp",
        "PageDown",
        "Enter",
        " ",
      ],
      attrs: (state, { props, api }) => ({
        tabindex: api.grid.tabIndex(state, props),
        "data-active": api.grid.isActive(state, props) ? "" : undefined,
      }),
      press: ({ api, props }, event) => api.activate({ row: props.row, col: props.col }, event),
      on: {
        keydown: (event, { api, props }) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            api.activate({ row: props.row, col: props.col }, event);
            return;
          }
          api.grid.keydown(event);
        },
        pointerdown: (event, { api, props }) =>
          api.grid.focus({ row: props.row, col: props.col }, { reason: "item-press", event }),
      },
      bind: (el, { api, props }) => api.grid.bind(el, { row: props.row, col: props.col }),
    }),
  }),
});
