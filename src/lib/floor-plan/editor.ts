import {
  createChange,
  createResize,
  css,
  definePrimitive,
  live,
  type ChangeDetails,
  type ResizeBox,
  type ResizeDraft,
  type ResizeHandle,
  type ResizeReason,
} from "@ui-sdk/core";

export type Boxes = Record<string, ResizeBox>;
export type SelectReason = "item-press" | "focus";

export type FloorPlanEditorOptions = {
  boxes?: Boxes;
  defaultBoxes?: Boxes;
  onBoxesChange?: (boxes: Boxes, details: ChangeDetails<ResizeReason>) => void;
  selected?: string | null;
  defaultSelected?: string | null;
  onSelectedChange?: (key: string | null, details: ChangeDetails<SelectReason>) => void;
  width?: number;
  height?: number;
  step?: number;
  labels?: Record<string, string>;
};

export const floorPlanEditor = definePrimitive({
  name: "floor-plan-editor",
  options: {
    boxes: "value",
    defaultBoxes: "value",
    onBoxesChange: "callback",
    selected: "string",
    defaultSelected: "string",
    onSelectedChange: "callback",
    width: "number",
    height: "number",
    step: "number",
    labels: "value",
  },
  controlled: ["boxes", "selected"],
  live: ["draft"],
  state: (options: FloorPlanEditorOptions) => ({
    boxes: options.boxes ?? options.defaultBoxes ?? {},
    selected: options.selected ?? options.defaultSelected ?? null,
    draft: null as ResizeDraft | null,
    width: options.width ?? 0,
    height: options.height ?? 0,
    step: options.step ?? 1,
    labels: options.labels ?? {},
  }),
  sync: (options) => ({
    ...(options.boxes !== undefined && { boxes: options.boxes }),
    ...(options.selected !== undefined && { selected: options.selected }),
    ...(options.width !== undefined && { width: options.width }),
    ...(options.height !== undefined && { height: options.height }),
    ...(options.step !== undefined && { step: options.step }),
    ...(options.labels !== undefined && { labels: options.labels }),
  }),
  api: (context) => {
    const change = createChange(context);
    const resize = createResize(context, {
      boxes: (state) => state.boxes,
      draft: "draft",
      set: (value, details) => change("boxes", value, details),
      min: () => {
        const { step } = context.store.get();
        return { width: step, height: step };
      },
      bounds: () => {
        const { width, height } = context.store.get();
        return { left: 0, top: 0, right: width, bottom: height };
      },
      step: () => context.store.get().step,
      label: (key) => context.store.get().labels[key],
    });
    return {
      resize,
      select(key: string | null, event?: Event, reason: SelectReason = "item-press") {
        if (context.store.get().selected !== key) {
          change("selected", key, { reason, event });
        }
      },
    };
  },
  agent: { actions: ["select"] },
  slots: (slot) => ({
    root: slot({
      attrs: (state) => ({
        role: "group",
        style: css({
          position: "relative",
          width: `${state.width}px`,
          height: `${state.height}px`,
          "--cell": `${state.step}px`,
        }),
      }),
    }),
    box: slot<{ value: string }>({
      props: { value: "string" },
      attrs: (state, { props, api }) => ({
        "data-selected": state.selected === props.value ? "" : undefined,
        style: api.resize.boxStyle(state, props.value),
      }),
      live: live((state, { props, api }) => ({
        "data-dragging": api.resize.dragging(state, props.value) ? "" : undefined,
        style: api.resize.style(state, props.value),
      })),
    }),
    move: slot<{ value: string }>({
      props: { value: { kind: "string", from: "box", required: true } },
      attrs: (state, { props }) => ({
        type: "button",
        "aria-pressed": String(state.selected === props.value),
        style: "touch-action: none",
      }),
      on: {
        pointerdown: (event, { el, api, props }) => {
          api.select(props.value, event);
          api.resize.pointerdown(event, props.value, "move", el!);
        },
        keydown: (event, { api, props }) => {
          api.resize.keydown(event, props.value, "move");
        },
        focus: (event, { api, props }) => api.select(props.value, event, "focus"),
      },
    }),
    handle: slot<{ value: string; handle: ResizeHandle }>({
      props: { value: { kind: "string", from: "box", required: true }, handle: "string" },
      attrs: (_state, { props, api }) => ({
        type: "button",
        ...api.resize.handleAttrs(props.handle),
      }),
      on: {
        pointerdown: (event, { el, api, props }) =>
          api.resize.pointerdown(event, props.value, props.handle, el!),
        keydown: (event, { api, props }) => {
          api.resize.keydown(event, props.value, props.handle);
        },
      },
    }),
  }),
});
