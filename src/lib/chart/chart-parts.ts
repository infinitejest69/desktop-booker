import { createParts } from "@ui-sdk/svelte/primitive";
import { tariffChart } from "./chart";

const {
  Root: ChartGrid,
  Row: ChartRow,
  Header: ChartHeader,
  RowHeader: ChartRowHeader,
  Cell: ChartCell,
} = createParts(tariffChart, {
  root: "div",
  row: "div",
  header: "div",
  rowHeader: "div",
  cell: "div",
});

export { ChartCell, ChartGrid, ChartHeader, ChartRow, ChartRowHeader };
