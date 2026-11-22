// Stands in for Tauri's IPC in the browser for demo runs (screenshots and
// videos, Phase 11). Each command goes to the demo bridge
// (src-tauri/src/bin/demo_bridge.rs), which runs the app's real commands
// against a real database. Injected with page.addInitScript before the app loads.
(() => {
  const bridge = "http://127.0.0.1:1430/invoke/";
  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd, args) => {
      const response = await fetch(bridge + cmd, { method: "POST", body: JSON.stringify(args ?? {}) });
      const text = await response.text();
      const value = text ? JSON.parse(text) : null;
      if (!response.ok) throw value;
      return value;
    },
    transformCallback: () => 0,
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  };
})();
