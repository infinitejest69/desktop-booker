// Stand-in for the Rust backend in browser tests. Injected with page.addInitScript
// before the app loads; it mimics the Tauri commands in src-tauri/src/commands.rs.
(() => {
  const db = { settings: { hotel_name: "The Grange", address: "", phone: "", email: "", currency: "GBP", timezone: "Europe/London" }, floors: [], types: [], spaces: [] };
  const id = () => crypto.randomUUID();
  const ov = (a, b) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
  const err = (message) => { throw { kind: "validation", message }; };
  function seed() {
    if (db.floors.length) return false;
    db.types = [["Double","DBL"],["Twin","TWN"]].map(([name, code]) => ({ id: id(), name, code, default_occupancy: 2, max_occupancy: 2, description: "" }));
    const f = { id: id(), name: "Ground floor", level: 0, grid_cols: 24, grid_rows: 10 };
    db.floors.push(f);
    for (let i = 0; i < 4; i++) db.spaces.push({ id: id(), floor_id: f.id, kind: "room", name: `G0${i+1}`, room_type_id: db.types[i%2].id, suite_capacities: { theatre: null, boardroom: null, banquet: null, cabaret: null, reception: null }, rect: { x: i*3, y: 0, w: 3, h: 4 }, out_of_service: i === 3, notes: "" });
    db.spaces.push({ id: id(), floor_id: f.id, kind: "suite", name: "Garden Suite", room_type_id: null, suite_capacities: { theatre: 120, boardroom: null, banquet: 80, cabaret: null, reception: null }, rect: { x: 0, y: 6, w: 10, h: 4 }, out_of_service: false, notes: "" });
    return true;
  }
  const handlers = {
    app_version: () => "0.1.0",
    get_settings: () => db.settings,
    save_settings: ({ settings }) => { db.settings = settings; },
    list_floors: () => [...db.floors].sort((a, b) => a.level - b.level),
    save_floor: ({ floor }) => { db.floors = db.floors.filter(f => f.id !== floor.id).concat(floor); },
    delete_floor: ({ id }) => { if (db.spaces.some(s => s.floor_id === id)) err("Floor still has rooms"); db.floors = db.floors.filter(f => f.id !== id); },
    list_room_types: () => db.types,
    save_room_type: ({ roomType }) => { db.types = db.types.filter(t => t.id !== roomType.id).concat({ ...roomType, code: roomType.code.toUpperCase() }); },
    delete_room_type: ({ id }) => { db.types = db.types.filter(t => t.id !== id); },
    list_spaces: ({ floorId }) => db.spaces.filter(s => !floorId || s.floor_id === floorId),
    save_space: ({ space }) => {
      const clash = db.spaces.find(o => o.id !== space.id && o.floor_id === space.floor_id && ov(o.rect, space.rect));
      if (clash) err(`"${space.name}" overlaps "${clash.name}"`);
      db.spaces = db.spaces.filter(s => s.id !== space.id).concat(space);
      window.__saves = (window.__saves || 0) + 1;
    },
    delete_space: ({ id }) => { db.spaces = db.spaces.filter(s => s.id !== id); },
    seed_demo_data: () => seed(),
  };
  window.__fakeDb = db;
  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd, args) => {
      const h = handlers[cmd];
      if (!h) throw { kind: "internal", message: `no fake for ${cmd}` };
      return structuredClone(h(args || {}));
    },
    transformCallback: () => 0,
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  };
})();
