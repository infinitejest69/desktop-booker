// Stand-in for the Rust backend in browser tests. Injected with page.addInitScript
// before the app loads; it mimics the Tauri commands in src-tauri/src/commands.rs
// closely enough for UI tests. The real rules are tested in Rust (crates/*/tests).
(() => {
  const db = {
    settings: {
      hotel_name: "The Grange",
      address: "",
      phone: "",
      email: "",
      currency: "GBP",
      timezone: "Europe/London",
      check_in_time: "15:00",
      check_out_time: "11:00",
      idle_lock_minutes: 15,
    },
    floors: [],
    types: [],
    spaces: [],
    rules: [],
    boards: [],
    guests: [],
    bookings: [],
    counter: 0,
    users: [],
    audit: [],
  };
  // ?fake=setup starts with no users; ?fake=locked starts signed out.
  const mode = new URLSearchParams(location.search).get("fake");
  const andy = { id: crypto.randomUUID(), name: "Andy", role: "admin", active: true };
  const sam = { id: crypto.randomUUID(), name: "Sam", role: "staff", active: true };
  const pins = new Map();
  if (mode !== "setup") {
    db.users = [andy, sam];
    pins.set(andy.id, "1234");
    pins.set(sam.id, "5678");
  }
  const attempts = {};
  let current = mode === "setup" || mode === "locked" ? null : andy;
  const log = (summary, entity = "booking") =>
    db.audit.unshift({
      id: crypto.randomUUID(),
      at: new Date().toISOString(),
      user_id: current?.id ?? null,
      user_name: current?.name ?? "System",
      entity,
      entity_id: "",
      action: "updated",
      summary,
      before: null,
      after: null,
    });
  const id = () => crypto.randomUUID();
  const err = (message) => {
    throw { kind: "validation", message };
  };
  const ov = (a, b) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
  const iso = (d) => d.toISOString().slice(0, 10);
  const today = () => {
    const n = new Date();
    return iso(new Date(Date.UTC(n.getFullYear(), n.getMonth(), n.getDate())));
  };
  const addDays = (s, n) => {
    const d = new Date(`${s}T00:00:00Z`);
    d.setUTCDate(d.getUTCDate() + n);
    return iso(d);
  };
  const nightsOf = (stay) => {
    const out = [];
    for (let d = stay.start; d < stay.end; d = addDays(d, 1)) out.push(d);
    return out;
  };
  const holds = (status) => status !== "cancelled" && status !== "no_show";
  const noCaps = { theatre: null, boardroom: null, banquet: null, cabaret: null, reception: null };

  function quote(spaceId, stay, adults, children, boardId) {
    const space = db.spaces.find((s) => s.id === spaceId);
    const type = db.types.find((t) => t.id === space?.room_type_id);
    if (!type) throw { kind: "not_found", message: "Room not found" };
    const board = db.boards.find((b) => b.id === boardId);
    const extra =
      Math.max(adults - type.default_occupancy, 0) * type.extra_adult_rate + children * type.child_rate;
    const boardPrice = board ? board.adult_price * adults + board.child_price * children : 0;
    const nights = nightsOf(stay).map((night) => ({
      night,
      room: type.base_rate,
      supplements: extra,
      board: boardPrice,
      total: type.base_rate + extra + boardPrice,
      rules: [],
    }));
    return { nights, total: nights.reduce((s, n) => s + n.total, 0) };
  }

  function occupied(spaceId, stay, ignore) {
    return db.bookings.some(
      (b) =>
        b.booking.id !== ignore &&
        holds(b.booking.status) &&
        b.booking.rooms.some(
          (r) => r.space_id === spaceId && r.stay.start < stay.end && stay.start < r.stay.end,
        ),
    );
  }

  function detailOf(b) {
    const guest = db.guests.find((g) => g.id === b.booking.guest_id);
    const pricing = b.booking.rooms.map((r) => {
      const nights = holds(b.booking.status)
        ? quote(r.space_id, r.stay, r.adults, r.children, r.board_option_id).nights
        : [];
      const calculated = nights.reduce((s, n) => s + n.total, 0);
      return { booking_room_id: r.id, nights, calculated, total: r.price_override ?? calculated };
    });
    return { booking: b.booking, guest, pricing, total: pricing.reduce((s, p) => s + p.total, 0) };
  }

  function saveBooking(booking, guest) {
    if (!guest.first_name.trim() && !guest.last_name.trim()) err("The guest needs a first or last name");
    for (const r of booking.rooms) {
      if (holds(booking.status) && occupied(r.space_id, r.stay, booking.id)) {
        const name = db.spaces.find((s) => s.id === r.space_id)?.name;
        err(`Room ${name} is already booked`);
      }
    }
    db.guests = db.guests.filter((g) => g.id !== guest.id).concat(guest);
    const existing = db.bookings.find((b) => b.booking.id === booking.id);
    const reference =
      existing?.booking.reference ??
      `${new Date().getFullYear()}-${String(++db.counter).padStart(4, "0")}`;
    const saved = { booking: { ...booking, reference, guest_id: guest.id } };
    db.bookings = db.bookings.filter((b) => b.booking.id !== booking.id).concat(saved);
    return detailOf(saved);
  }

  function seed() {
    if (db.floors.length) return false;
    db.types = [
      ["Double", "DBL", 9500],
      ["Twin", "TWN", 8500],
    ].map(([name, code, rate]) => ({
      id: id(),
      name,
      code,
      default_occupancy: 2,
      max_occupancy: 3,
      description: "",
      base_rate: rate,
      extra_adult_rate: 2500,
      child_rate: 1500,
    }));
    const f = { id: id(), name: "Ground floor", level: 0, grid_cols: 24, grid_rows: 10 };
    db.floors.push(f);
    for (let i = 0; i < 4; i++)
      db.spaces.push({
        id: id(),
        floor_id: f.id,
        kind: "room",
        name: `G0${i + 1}`,
        room_type_id: db.types[i % 2].id,
        suite_capacities: noCaps,
        rect: { x: i * 3, y: 0, w: 3, h: 4 },
        out_of_service: i === 3,
        notes: "",
      });
    db.spaces.push({
      id: id(),
      floor_id: f.id,
      kind: "suite",
      name: "Garden Suite",
      room_type_id: null,
      suite_capacities: { ...noCaps, theatre: 120, banquet: 80 },
      rect: { x: 0, y: 6, w: 10, h: 4 },
      out_of_service: false,
      notes: "",
    });
    db.boards = [
      { id: id(), code: "RO", name: "Room only", adult_price: 0, child_price: 0 },
      { id: id(), code: "BB", name: "Bed & breakfast", adult_price: 1200, child_price: 700 },
    ];
    const t = today();
    const guest = (first, last) => ({
      id: id(),
      first_name: first,
      last_name: last,
      email: `${first.toLowerCase()}@example.com`,
      phone: "",
      address: "",
      notes: "",
    });
    const book = (g, space, start, nights) =>
      saveBooking(
        {
          id: id(),
          reference: "",
          guest_id: g.id,
          status: "confirmed",
          source: "phone",
          notes: "",
          rooms: [
            {
              id: id(),
              space_id: space.id,
              stay: { start, end: addDays(start, nights) },
              adults: 2,
              children: 0,
              board_option_id: db.boards[1].id,
              price_override: null,
            },
          ],
        },
        g,
      );
    book(guest("Margaret", "Hughes"), db.spaces[1], t, 2);
    book(guest("Tom", "Price"), db.spaces[2], addDays(t, -1), 3);
    return true;
  }

  const PUBLIC = new Set(["app_version", "session_status", "create_first_admin", "login", "logout"]);
  const ADMIN_ONLY = new Set([
    "save_settings", "save_floor", "delete_floor", "save_room_type", "delete_room_type",
    "save_space", "delete_space", "save_rate_rule", "delete_rate_rule", "save_board_option",
    "delete_board_option", "list_users", "save_user", "list_audit", "seed_demo_data",
  ]);

  const guestName = (b) => {
    const g = db.guests.find((x) => x.id === b.booking.guest_id);
    return { g, name: `${g.first_name} ${g.last_name}`.trim() };
  };

  const handlers = {
    app_version: () => "0.1.0",
    get_settings: () => db.settings,
    save_settings: ({ settings }) => {
      db.settings = settings;
    },
    list_floors: () => [...db.floors].sort((a, b) => a.level - b.level),
    save_floor: ({ floor }) => {
      db.floors = db.floors.filter((f) => f.id !== floor.id).concat(floor);
    },
    delete_floor: ({ id }) => {
      if (db.spaces.some((s) => s.floor_id === id)) err("Floor still has rooms");
      db.floors = db.floors.filter((f) => f.id !== id);
    },
    list_room_types: () => db.types,
    save_room_type: ({ roomType }) => {
      db.types = db.types
        .filter((t) => t.id !== roomType.id)
        .concat({ ...roomType, code: roomType.code.toUpperCase() });
    },
    delete_room_type: ({ id }) => {
      db.types = db.types.filter((t) => t.id !== id);
    },
    list_spaces: ({ floorId }) => db.spaces.filter((s) => !floorId || s.floor_id === floorId),
    save_space: ({ space }) => {
      const clash = db.spaces.find(
        (o) => o.id !== space.id && o.floor_id === space.floor_id && ov(o.rect, space.rect),
      );
      if (clash) err(`"${space.name}" overlaps "${clash.name}"`);
      db.spaces = db.spaces.filter((s) => s.id !== space.id).concat(space);
    },
    delete_space: ({ id }) => {
      db.spaces = db.spaces.filter((s) => s.id !== id);
    },
    list_rate_rules: () => db.rules,
    save_rate_rule: ({ rule }) => {
      if (rule.end < rule.start) err("A rate rule's end date must be on or after its start date");
      db.rules = db.rules.filter((r) => r.id !== rule.id).concat(rule);
    },
    delete_rate_rule: ({ id }) => {
      db.rules = db.rules.filter((r) => r.id !== id);
    },
    list_board_options: () => db.boards,
    save_board_option: ({ board }) => {
      db.boards = db.boards
        .filter((b) => b.id !== board.id)
        .concat({ ...board, code: board.code.toUpperCase() });
    },
    delete_board_option: ({ id }) => {
      db.boards = db.boards.filter((b) => b.id !== id);
    },
    search_guests: ({ query }) =>
      db.guests.filter((g) =>
        `${g.first_name} ${g.last_name} ${g.email}`.toLowerCase().includes(query.toLowerCase()),
      ),
    quote_room: ({ spaceId, stay, adults, children, boardOptionId }) =>
      quote(spaceId, stay, adults, children, boardOptionId),
    free_rooms: ({ stay, ignoreBooking }) =>
      db.spaces
        .filter((s) => s.kind === "room" && !s.out_of_service && !occupied(s.id, stay, ignoreBooking))
        .map((s) => s.id),
    chart: ({ from, to }) =>
      db.bookings
        .filter((b) => holds(b.booking.status))
        .flatMap((b) =>
          b.booking.rooms
            .filter((r) => r.stay.start < to && r.stay.end > from)
            .map((r) => ({
              booking_id: b.booking.id,
              booking_room_id: r.id,
              reference: b.booking.reference,
              guest_name: guestName(b).name,
              status: b.booking.status,
              space_id: r.space_id,
              stay: r.stay,
              adults: r.adults,
              children: r.children,
            })),
        ),
    get_booking: ({ id }) => {
      const b = db.bookings.find((x) => x.booking.id === id);
      if (!b) throw { kind: "not_found", message: "Booking not found" };
      return detailOf(b);
    },
    save_booking: ({ booking, guest }) => {
      const saved = saveBooking(booking, guest);
      log(`Made booking ${saved.booking.reference} for ${guest.first_name} ${guest.last_name}`.trim());
      return saved;
    },
    set_booking_status: ({ id, status }) => {
      const b = db.bookings.find((x) => x.booking.id === id);
      return saveBooking({ ...b.booking, status }, guestName(b).g);
    },
    search_bookings: ({ query }) =>
      db.bookings
        .filter((b) => {
          const { g } = guestName(b);
          return `${b.booking.reference} ${g.first_name} ${g.last_name} ${g.email}`
            .toLowerCase()
            .includes(query.toLowerCase());
        })
        .map((b) => ({
          id: b.booking.id,
          reference: b.booking.reference,
          guest_name: guestName(b).name,
          status: b.booking.status,
          arrival: b.booking.rooms[0].stay.start,
          departure: b.booking.rooms[0].stay.end,
          rooms: b.booking.rooms.map((r) => db.spaces.find((s) => s.id === r.space_id)?.name ?? "?"),
        })),
    seed_demo_data: () => seed(),

    session_status: () => ({
      needs_setup: db.users.length === 0,
      user: current,
      users: db.users.filter((u) => u.active),
      hotel_name: db.settings.hotel_name,
      idle_lock_minutes: db.settings.idle_lock_minutes,
    }),
    create_first_admin: ({ name, pin }) => {
      const u = { id: crypto.randomUUID(), name, role: "admin", active: true };
      db.users.push(u);
      pins.set(u.id, pin);
      current = u;
      return u;
    },
    login: ({ userId, pin }) => {
      const tries = (attempts[userId] ??= { failures: 0, until: 0 });
      if (Date.now() < tries.until) {
        throw { kind: "validation", message: "Too many wrong PINs.", retry_after: Math.ceil((tries.until - Date.now()) / 1000) };
      }
      if (pins.get(userId) !== pin) {
        tries.failures += 1;
        if (tries.failures >= 5) {
          tries.failures = 0;
          tries.until = Date.now() + 30_000;
          throw { kind: "validation", message: "Too many wrong PINs.", retry_after: 30 };
        }
        err(`Wrong PIN. ${5 - tries.failures} tries left.`);
      }
      current = db.users.find((u) => u.id === userId);
      return current;
    },
    logout: () => {
      current = null;
    },
    list_users: () => db.users,
    save_user: ({ user, pin }) => {
      const existed = db.users.some((u) => u.id === user.id);
      if (!existed && !pin) err("A PIN is 4 to 8 digits");
      db.users = db.users.filter((u) => u.id !== user.id).concat(user);
      if (pin) pins.set(user.id, pin);
      log(existed ? `Changed details for ${user.name}` : `Added ${user.role === "admin" ? "admin" : "staff member"} ${user.name}`, "user");
    },
    list_audit: ({ userId, entity }) =>
      db.audit.filter((e) => (!userId || e.user_id === userId) && (!entity || e.entity === entity)),
  };

  window.__fakeDb = db;
  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd, args) => {
      if (!PUBLIC.has(cmd) && !current) throw { kind: "locked", message: "Please sign in to continue." };
      if (ADMIN_ONLY.has(cmd) && current?.role !== "admin")
        throw { kind: "forbidden", message: "Only an admin can change the hotel's setup." };
      const h = handlers[cmd];
      if (!h) throw { kind: "internal", message: `no fake for ${cmd}` };
      return structuredClone(h(args || {}));
    },
    transformCallback: () => 0,
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  };
})();
