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
    suiteRates: {},
    suiteBookings: [],
    suitePrices: {},
    events: [],
    taxRates: [],
    invoiceSettings: {
      prices_include_tax: true, prefix: "INV-", footer: "VAT no. GB 123 4567 89",
      accommodation_rate: null, board_rate: null, suite_rate: null, food_rate: null, drink_rate: null, other_rate: null,
    },
    charges: [],
    payments: [],
    invoices: [],
    invoiceCounter: 0,
    housekeeping: {},
    backupSettings: { folder: "", keep_daily: 14, keep_monthly: 12 },
    backups: [],
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

  const boardNameOf = (id) => db.boards.find((b) => b.id === id)?.name ?? null;

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
    const book = (g, space, start, nights, boardId) =>
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
              board_option_id: boardId,
              price_override: null,
            },
          ],
        },
        g,
      );
    book(guest("Margaret", "Hughes"), db.spaces[1], t, 2, db.boards[1].id);
    book(guest("Tom", "Price"), db.spaces[2], addDays(t, -1), 3, null);
    seedEvents();
    const standard = { id: id(), name: "Standard VAT", basis_points: 2000 };
    db.taxRates = [standard, { id: id(), name: "Zero rated", basis_points: 0 }];
    for (const k of ["accommodation", "board", "suite", "food", "drink", "other"]) db.invoiceSettings[`${k}_rate`] = standard.id;
    return true;
  }

  // ---- function suites and events (mirrors crates/store/src/events.rs) ----

  const suites = () => db.spaces.filter((s) => s.kind === "suite");
  const shiftMinutes = (dt, m) => {
    const d = new Date(`${dt}Z`);
    d.setUTCMinutes(d.getUTCMinutes() + m);
    return d.toISOString().slice(0, 19);
  };
  const minutesBetween = (a, b) => Math.round((Date.parse(`${b}Z`) - Date.parse(`${a}Z`)) / 60000);

  function suitePrice(b) {
    const r = db.suiteRates[b.suite_id] ?? {};
    const p = b.pricing;
    if (p.kind === "hourly") {
      if (r.hourly == null) err("This suite has no hourly price; choose another way of pricing it");
      return r.hourly * Math.max(Math.ceil(minutesBetween(b.start, b.end) / 60), r.min_hours);
    }
    if (p.kind === "half_day") return r.half_day ?? err("This suite has no half-day price; choose another way of pricing it");
    if (p.kind === "full_day") return r.full_day ?? err("This suite has no full-day price; choose another way of pricing it");
    if (p.kind === "per_head") return p.price * b.attendees;
    return p.price;
  }

  function suiteEntry(b) {
    const g = db.guests.find((x) => x.id === b.guest_id);
    return {
      booking: b,
      price: db.suitePrices[b.id],
      suite_name: db.spaces.find((s) => s.id === b.suite_id)?.name ?? "?",
      guest_name: g ? `${g.first_name} ${g.last_name}`.trim() : "?",
      event_name: db.events.find((e) => e.id === b.event_id)?.name ?? null,
    };
  }

  function saveSuiteBooking(b, guest) {
    const suite = db.spaces.find((s) => s.id === b.suite_id);
    if (!suite || suite.kind !== "suite") err("Choose a function suite");
    if (!b.title.trim()) err("What it's for must not be empty");
    if (b.end <= b.start) err("The end time must be after the start time");
    if (b.status !== "cancelled") {
      const cap = suite.suite_capacities[b.layout];
      if (!cap) err(`This suite isn't set up for a ${b.layout} layout`);
      if (b.attendees > cap) err(`This layout seats at most ${cap}`);
      const pad = (x) => [shiftMinutes(x.start, -x.setup_minutes), shiftMinutes(x.end, x.cleardown_minutes)];
      const [a0, a1] = pad(b);
      const clash = db.suiteBookings.find((o) => {
        if (o.id === b.id || o.suite_id !== b.suite_id || o.status === "cancelled") return false;
        const [b0, b1] = pad(o);
        return a0 < b1 && b0 < a1;
      });
      if (clash) err(`${suite.name} is in use then ("${clash.title}", including setup and clear-down)`);
    }
    const old = db.suiteBookings.find((o) => o.id === b.id);
    const same =
      old && old.start === b.start && old.end === b.end && old.attendees === b.attendees &&
      JSON.stringify(old.pricing) === JSON.stringify(b.pricing);
    db.suitePrices[b.id] = same ? db.suitePrices[b.id] : suitePrice(b);
    db.guests = db.guests.filter((g) => g.id !== guest.id).concat(guest);
    db.suiteBookings = db.suiteBookings.filter((o) => o.id !== b.id).concat({ ...b, guest_id: guest.id });
    return suiteEntry(db.suiteBookings.find((o) => o.id === b.id));
  }

  const linked = (eventId) => db.bookings.filter((b) => b.booking.event_id === eventId);

  function eventDetail(ev) {
    const organiser = db.guests.find((g) => g.id === ev.organiser_id);
    const suitesFor = db.suiteBookings
      .filter((b) => b.event_id === ev.id)
      .sort((a, b) => a.start.localeCompare(b.start))
      .map(suiteEntry);
    const rooms = linked(ev.id).map((b) => {
      const d = detailOf(b);
      return {
        booking_id: b.booking.id,
        reference: b.booking.reference,
        guest_name: guestName(b).name,
        status: b.booking.status,
        rooms: b.booking.rooms.map((r) => db.spaces.find((s) => s.id === r.space_id)?.name ?? "?"),
        stay: b.booking.rooms[0].stay,
        total: d.total,
      };
    });
    const suites_value = suitesFor.filter((s) => s.booking.status !== "cancelled").reduce((n, s) => n + s.price, 0);
    const rooms_value = rooms.filter((r) => holds(r.status)).reduce((n, r) => n + r.total, 0);
    return {
      event: ev,
      organiser,
      suites: suitesFor,
      rooms,
      held_rooms: rooms.filter((r) => r.status === "provisional").length,
      catering: {
        sittings: ev.sittings.length,
        adults: ev.sittings.reduce((n, x) => n + x.adults, 0),
        children: ev.sittings.reduce((n, x) => n + x.children, 0),
      },
      suites_value,
      rooms_value,
      total: suites_value + rooms_value,
    };
  }

  /** Holds, reprices or releases block rooms; returns how many were released. */
  function syncBlock(ev) {
    const block = ev.room_block;
    const live = linked(ev.id).filter((b) => holds(b.booking.status));
    const held = live.filter((b) => b.booking.status === "provisional");
    const release = (list) => list.forEach((b) => (b.booking.status = "cancelled"));
    if (!block || today() >= block.release_date) {
      release(held);
      return held.length;
    }
    const sameStay = (b) => b.booking.rooms[0].stay.start === block.stay.start && b.booking.rooms[0].stay.end === block.stay.end;
    const moved = held.filter((b) => !sameStay(b));
    const keep = held.filter(sameStay);
    const counted = live.length - held.length + keep.length;
    const surplus = keep.splice(Math.max(keep.length - Math.max(counted - block.rooms, 0), 0));
    release([...moved, ...surplus]);
    const price = block.rate * nightsOf(block.stay).length;
    keep.forEach((b) => b.booking.rooms.forEach((r) => (r.price_override = price)));
    const add = Math.max(block.rooms - (counted - surplus.length), 0);
    const free = db.spaces.filter(
      (s) => s.kind === "room" && !s.out_of_service && (!block.room_type_id || s.room_type_id === block.room_type_id) && !occupied(s.id, block.stay, null),
    );
    if (free.length < add) err(`Only ${free.length} suitable room(s) are free for those nights; the block needs ${add} more`);
    const organiser = db.guests.find((g) => g.id === ev.organiser_id);
    for (const space of free.slice(0, add)) {
      const saved = saveBooking(
        {
          id: id(),
          reference: "",
          guest_id: organiser.id,
          status: "provisional",
          source: "other",
          notes: `Held for ${ev.name}`,
          rooms: [{ id: id(), space_id: space.id, stay: block.stay, adults: 2, children: 0, board_option_id: null, price_override: price }],
        },
        organiser,
      );
      db.bookings.find((b) => b.booking.id === saved.booking.id).booking.event_id = ev.id;
    }
    return moved.length + surplus.length;
  }

  function saveEvent(event, organiser) {
    if (!event.name.trim()) err("Event name must not be empty");
    if (!organiser.first_name.trim() && !organiser.last_name.trim()) err("The guest needs a first or last name");
    const block = event.room_block;
    if (block && block.release_date > block.stay.start) err("The release date must be on or before the guests' arrival");
    const before = db.events.find((e) => e.id === event.id);
    if (current?.role !== "admin" && block && block.rate !== before?.room_block?.rate)
      throw { kind: "forbidden", message: "Only an admin can set a room block's group rate." };
    const snapshot = structuredClone({ events: db.events, bookings: db.bookings, guests: db.guests });
    try {
      db.guests = db.guests.filter((g) => g.id !== organiser.id).concat(organiser);
      const saved = { ...event, organiser_id: organiser.id };
      db.events = db.events.filter((e) => e.id !== event.id).concat(saved);
      syncBlock(saved);
      log(`${before ? "Changed" : "Added"} event ${event.name}`, "event");
      return eventDetail(saved);
    } catch (e) {
      Object.assign(db, snapshot);
      throw e;
    }
  }

  function seedEvents() {
    const t = today();
    const garden = suites()[0];
    if (!garden || db.events.length) return false;
    db.suiteRates[garden.id] = { hourly: 10000, min_hours: 3, half_day: 40000, full_day: 70000, setup_minutes: 60, cleardown_minutes: 30 };
    const contact = (first, last) => ({ id: id(), first_name: first, last_name: last, email: `${first.toLowerCase()}@example.com`, phone: "", address: "", notes: "" });
    const ffion = contact("Ffion", "Hughes");
    const wedding = addDays(t, 20);
    const ev = saveEvent(
      {
        id: id(),
        name: "Hughes–Price wedding",
        kind: "wedding",
        organiser_id: ffion.id,
        start_date: wedding,
        end_date: wedding,
        notes: "Band brings own PA.",
        running_order: "13:00 Ceremony\n15:00 Wedding breakfast",
        sittings: [{ name: "Wedding breakfast", date: wedding, time: "15:00:00", adults: 60, children: 4, dietary: "5 vegetarian\n1 nut allergy" }],
        room_block: { room_type_id: db.types[0].id, rooms: 1, stay: { start: wedding, end: addDays(wedding, 1) }, rate: 8500, release_date: addDays(t, 6) },
      },
      ffion,
    ).event;
    const hire = (guest, title, start, end, extra) =>
      saveSuiteBooking(
        { id: id(), suite_id: garden.id, event_id: null, guest_id: guest.id, title, start, end, layout: "theatre", attendees: 40, setup_minutes: 60, cleardown_minutes: 30, pricing: { kind: "hourly" }, status: "confirmed", notes: "", ...extra },
        guest,
      );
    hire(contact("Owen", "Davies"), "Board meeting", `${t}T10:00:00`, `${t}T12:00:00`, {});
    hire(ffion, "Wedding breakfast", `${wedding}T15:00:00`, `${wedding}T23:00:00`, { event_id: ev.id, layout: "banquet", attendees: 64, pricing: { kind: "per_head", price: 5500 } });
    return true;
  }

  const key = (owner) => JSON.stringify({ kind: owner.kind, id: owner.id });
  const vat = (gross, bp) => Math.round((gross * bp) / (10000 + bp));
  const rateFor = (category) => {
    const rid = db.invoiceSettings[`${category}_rate`];
    return db.taxRates.find((r) => r.id === rid)?.basis_points ?? 0;
  };

  function folioOf(owner) {
    let title = "";
    let billTo = "";
    let automatic = [];
    if (owner.kind === "booking") {
      const b = db.bookings.find((x) => x.booking.id === owner.id);
      const d = detailOf(b);
      title = `Booking ${b.booking.reference}`;
      billTo = guestName(b).name;
      automatic = b.booking.rooms.flatMap((r, i) => {
        const nights = d.pricing[i].nights.length;
        if (!nights) return [];
        const name = db.spaces.find((s) => s.id === r.space_id)?.name;
        return [{ id: `stay-${r.id}`, date: r.stay.start, category: "accommodation", description: `Room ${name}, ${nights} night${nights === 1 ? "" : "s"}`, quantity: 1, unit_price: d.pricing[i].total, tax_basis_points: rateFor("accommodation") }];
      });
    } else if (owner.kind === "event") {
      const ev = db.events.find((e) => e.id === owner.id);
      title = ev.name;
      billTo = db.guests.find((g) => g.id === ev.organiser_id)?.last_name ?? "";
      automatic = db.suiteBookings
        .filter((sb) => sb.event_id === ev.id && sb.status !== "cancelled")
        .map((sb) => ({ id: `suite-${sb.id}`, date: sb.start.slice(0, 10), category: "suite", description: sb.title, quantity: 1, unit_price: db.suitePrices[sb.id], tax_basis_points: rateFor("suite") }));
    }
    const extras = db.charges.filter((c) => c.owner === key(owner)).map((c) => c.charge);
    const lines = [...automatic.map((c) => [c, true]), ...extras.map((c) => [c, false])].map(([charge, automatic]) => {
      const gross = charge.unit_price * charge.quantity;
      const tax = vat(gross, charge.tax_basis_points);
      return { charge, automatic, amounts: { net: gross - tax, tax, gross } };
    });
    const payments = db.payments.filter((p) => p.owner === key(owner)).map((p) => p.payment);
    const byRate = {};
    for (const l of lines) {
      const r = (byRate[l.charge.tax_basis_points] ??= { basis_points: l.charge.tax_basis_points, net: 0, tax: 0 });
      r.net += l.amounts.net;
      r.tax += l.amounts.tax;
    }
    const sum = (xs) => xs.reduce((n, x) => n + x, 0);
    const gross = sum(lines.map((l) => l.amounts.gross));
    const paid = sum(payments.map((p) => p.amount));
    return {
      owner,
      title,
      bill_to: billTo,
      lines,
      payments,
      prices_include_tax: true,
      invoices: db.invoices.filter((i) => key(i.owner) === key(owner)).map((i) => ({ id: i.id, number: i.number, issued_on: i.issued_on, gross: i.totals.gross })),
      totals: {
        lines: lines.map((l) => l.amounts),
        net: sum(lines.map((l) => l.amounts.net)),
        tax: sum(lines.map((l) => l.amounts.tax)),
        gross,
        by_rate: Object.values(byRate).sort((a, b) => a.basis_points - b.basis_points),
        deposits: sum(payments.filter((p) => p.is_deposit).map((p) => p.amount)),
        paid,
        balance: gross - paid,
      },
    };
  }

  function backupStatus() {
    const last = db.backups[0]?.taken_at ?? null;
    return {
      settings: db.backupSettings,
      folder: db.backupSettings.folder || "/home/demo/.local/share/com.desktopbooker.app/backups",
      folder_ok: true,
      last,
      overdue: last === null,
      backups: db.backups,
    };
  }

  const PUBLIC = new Set(["app_version", "session_status", "create_first_admin", "login", "logout"]);
  const ADMIN_ONLY = new Set([
    "save_settings", "save_floor", "delete_floor", "save_room_type", "delete_room_type",
    "save_space", "delete_space", "save_rate_rule", "delete_rate_rule", "save_board_option",
    "delete_board_option", "list_users", "save_user", "list_audit", "seed_demo_data",
    "save_suite_rates", "save_tax_rate", "save_invoice_settings", "erase_guest", "merge_guests", "revenue_report", "room_kpis", "save_backup_settings", "backup_now", "restore_backup",
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
              board: boardNameOf(r.board_option_id),
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
      const saved = saveBooking({ ...b.booking, status }, guestName(b).g);
      // Like the store: a check-out leaves the rooms dirty.
      if (status === "checked_out") b.booking.rooms.forEach((r) => (db.housekeeping[r.space_id] = { status: "dirty", on: today() }));
      return saved;
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
          boards: b.booking.rooms.map((r) => boardNameOf(r.board_option_id)),
        })),
    seed_demo_data: () => seed() || seedEvents(),

    get_suite_rates: ({ suite }) => {
      if (!suites().some((s) => s.id === suite)) throw { kind: "not_found", message: "Suite not found" };
      return db.suiteRates[suite] ?? { hourly: null, min_hours: 0, half_day: null, full_day: null, setup_minutes: 60, cleardown_minutes: 30 };
    },
    save_suite_rates: ({ suite, rates }) => {
      db.suiteRates[suite] = rates;
      log(`Changed the rates for ${db.spaces.find((s) => s.id === suite)?.name}`, "space");
    },
    list_suite_bookings: ({ from, to }) =>
      db.suiteBookings
        .filter((b) => b.status !== "cancelled" && b.start < to && b.end > from)
        .sort((a, b) => a.start.localeCompare(b.start))
        .map(suiteEntry),
    get_suite_booking: ({ id: bookingId }) => {
      const b = db.suiteBookings.find((x) => x.id === bookingId);
      if (!b) throw { kind: "not_found", message: "Suite booking not found" };
      return suiteEntry(b);
    },
    save_suite_booking: ({ booking, guest }) => {
      const before = db.suiteBookings.find((b) => b.id === booking.id);
      if (booking.pricing.kind === "manual" && current?.role !== "admin" && JSON.stringify(before?.pricing) !== JSON.stringify(booking.pricing))
        throw { kind: "forbidden", message: "Only an admin can set a price by hand." };
      return saveSuiteBooking(booking, guest);
    },
    set_suite_booking_status: ({ id: bookingId, status }) => {
      const b = db.suiteBookings.find((x) => x.id === bookingId);
      const guest = db.guests.find((g) => g.id === b.guest_id);
      return saveSuiteBooking({ ...b, status }, guest);
    },
    list_events: ({ from }) =>
      db.events
        .filter((e) => e.end_date >= from)
        .sort((a, b) => a.start_date.localeCompare(b.start_date))
        .map((e) => {
          const g = db.guests.find((x) => x.id === e.organiser_id);
          return {
            id: e.id,
            name: e.name,
            kind: e.kind,
            organiser_name: `${g.first_name} ${g.last_name}`.trim(),
            start_date: e.start_date,
            end_date: e.end_date,
            suite_bookings: db.suiteBookings.filter((b) => b.event_id === e.id && b.status !== "cancelled").length,
            held_rooms: linked(e.id).filter((b) => b.booking.status === "provisional").length,
            release_date: e.room_block?.release_date ?? null,
          };
        }),
    get_event: ({ id: eventId }) => {
      const ev = db.events.find((e) => e.id === eventId);
      if (!ev) throw { kind: "not_found", message: "Event not found" };
      return eventDetail(ev);
    },
    save_event: ({ event, organiser }) => saveEvent(event, organiser),

    // Backups (mirrors src-tauri's backup commands): none yet, so overdue.
    backup_status: () => backupStatus(),
    save_backup_settings: ({ settings }) => {
      if (settings.folder.trim() && !settings.folder.startsWith("/media/")) err("That folder can't be found. Plug in the drive or choose another folder.");
      db.backupSettings = settings;
      return backupStatus();
    },
    backup_now: () => {
      const now = new Date();
      const stamp = now.toISOString().slice(0, 19).replace(/[-:]/g, "").replace("T", "-");
      db.backups.unshift({ name: `booker-${stamp}.db`, taken_at: now.toISOString().slice(0, 19), size_bytes: 245000 });
      return backupStatus();
    },
    restore_backup: () => {},

    // Housekeeping, reports and guests (mirrors crates/store/src/operations.rs).
    // A room slept in last night is dirty unless marked today (housekeeping_today in core).
    list_housekeeping: () => {
      const lastNight = addDays(today(), -1);
      return db.spaces
        .filter((s) => s.kind === "room")
        .map((s) => {
          const saved = db.housekeeping[s.id];
          const slept = db.bookings.some(
            (b) => holds(b.booking.status) && b.booking.rooms.some((r) => r.space_id === s.id && r.stay.start <= lastNight && lastNight < r.stay.end),
          );
          const status = slept && (!saved || saved.on < today()) ? "dirty" : (saved?.status ?? "inspected");
          return { space_id: s.id, status, note: "" };
        });
    },
    set_housekeeping: ({ space, status }) => {
      db.housekeeping[space] = { status, on: today() };
    },
    set_floor_housekeeping: ({ floor, status }) => {
      const rooms = db.spaces.filter((s) => s.floor_id === floor && s.kind === "room" && !s.out_of_service);
      rooms.forEach((s) => (db.housekeeping[s.id] = { status, on: today() }));
      return rooms.length;
    },
    occupancy_report: ({ period }) => {
      const rooms = db.spaces.filter((s) => s.kind === "room" && !s.out_of_service).length;
      return nightsOf(period).map((date) => {
        const occupied = db.bookings
          .filter((b) => holds(b.booking.status))
          .flatMap((b) => b.booking.rooms)
          .filter((r) => r.stay.start <= date && date < r.stay.end).length;
        return { date, occupied, rooms, per_mille: rooms ? Math.round((occupied * 1000) / rooms) : 0 };
      });
    },
    revenue_report: () => [],
    // Every room-night at £95 in the fake hotel.
    room_kpis: ({ period }) => {
      const days = handlers.occupancy_report({ period });
      const sold = days.reduce((n, d) => n + d.occupied, 0);
      const available = days.reduce((n, d) => n + d.rooms, 0);
      const revenue = sold * 9500;
      return { rooms_sold: sold, rooms_available: available, room_revenue: revenue, adr: sold ? 9500 : 0, revpar: available ? Math.round(revenue / available) : 0 };
    },
    report_csv: ({ report }) => (report === "occupancy" ? "Night,Rooms sold,Rooms,Occupancy %\r\n" : "Month,Rooms\r\n"),
    guest_profile: ({ id: guestId }) => {
      const guest = db.guests.find((g) => g.id === guestId);
      const stays = db.bookings
        .filter((b) => b.booking.guest_id === guestId)
        .map((b) => ({
          id: b.booking.id,
          reference: b.booking.reference,
          guest_name: guestName(b).name,
          status: b.booking.status,
          arrival: b.booking.rooms[0].stay.start,
          departure: b.booking.rooms[0].stay.end,
          rooms: b.booking.rooms.map((r) => db.spaces.find((s) => s.id === r.space_id)?.name ?? "?"),
          boards: b.booking.rooms.map(() => null),
        }));
      const total_spend = db.bookings
        .filter((b) => b.booking.guest_id === guestId && holds(b.booking.status))
        .reduce((n, b) => n + detailOf(b).total, 0);
      return { guest, stays, total_spend };
    },
    merge_guests: ({ keep, duplicate }) => {
      const k = db.guests.find((g) => g.id === keep);
      const d = db.guests.find((g) => g.id === duplicate);
      for (const f of ["first_name", "last_name", "email", "phone", "address"]) if (!k[f].trim()) k[f] = d[f];
      db.bookings.forEach((b) => b.booking.guest_id === duplicate && (b.booking.guest_id = keep));
      db.suiteBookings.forEach((b) => b.guest_id === duplicate && (b.guest_id = keep));
      db.events.forEach((e) => e.organiser_id === duplicate && (e.organiser_id = keep));
      db.guests = db.guests.filter((g) => g.id !== duplicate);
    },
    erase_guest: ({ id: guestId }) => {
      db.guests = db.guests.map((g) =>
        g.id === guestId ? { id: g.id, first_name: "", last_name: "Erased guest", email: "", phone: "", address: "", notes: "" } : g,
      );
    },

    // Invoicing (mirrors crates/store/src/invoicing.rs; VAT 20% included in prices).
    list_tax_rates: () => db.taxRates,
    get_invoice_settings: () => db.invoiceSettings,
    save_invoice_settings: ({ settings }) => {
      db.invoiceSettings = settings;
    },
    save_tax_rate: ({ rate }) => {
      db.taxRates = db.taxRates.filter((r) => r.id !== rate.id).concat(rate);
    },
    get_folio: ({ owner }) => folioOf(owner),
    add_charge: ({ owner, charge }) => {
      if (!charge.description.trim()) err("Description must not be empty");
      db.charges.push({ owner: key(owner), charge });
      return folioOf(owner);
    },
    remove_charge: ({ id: chargeId }) => {
      const found = db.charges.find((c) => c.charge.id === chargeId);
      db.charges = db.charges.filter((c) => c !== found);
      return folioOf(JSON.parse(found.owner));
    },
    add_payment: ({ owner, payment }) => {
      const paid = folioOf(owner).totals.paid;
      if (payment.amount === 0) err("Amount must not be empty");
      if (payment.amount < 0 && paid + payment.amount < 0) err("A refund can't be more than has been paid");
      db.payments.push({ owner: key(owner), payment });
      return folioOf(owner);
    },
    issue_invoice: ({ owner }) => {
      const f = folioOf(owner);
      if (!f.lines.length) err("The folio must not be empty");
      const inv = {
        id: id(),
        number: `${db.invoiceSettings.prefix}${String(++db.invoiceCounter).padStart(6, "0")}`,
        owner,
        issued_on: today(),
        bill_to: f.bill_to,
        bill_address: "",
        title: f.title,
        lines: f.lines,
        payments: f.payments,
        totals: f.totals,
        prices_include_tax: true,
        footer: db.invoiceSettings.footer,
      };
      db.invoices.push(inv);
      return inv;
    },
    get_invoice: ({ id: invoiceId }) => {
      const inv = db.invoices.find((i) => i.id === invoiceId);
      if (!inv) throw { kind: "not_found", message: "Invoice not found" };
      return inv;
    },

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
