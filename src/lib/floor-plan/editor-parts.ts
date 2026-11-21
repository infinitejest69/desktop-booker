import { createParts } from "@ui-sdk/svelte/primitive";
import { floorPlanEditor } from "./editor";

const {
  Root: FloorPlanEditor,
  Box: FloorPlanBox,
  Move: FloorPlanMove,
  Handle: FloorPlanHandle,
} = createParts(floorPlanEditor, {
  root: "div",
  box: "div",
  move: "button",
  handle: "button",
});

export { FloorPlanBox, FloorPlanEditor, FloorPlanHandle, FloorPlanMove };
