# Phase 4 — Function suites & events

**Goal:** book function suites by time and group everything for a wedding or dinner under one event.
**Rust concepts:** more complex domain modelling, enums with data, time ranges with `chrono::NaiveDateTime`.

## Tasks
- [ ] `core`: time-slot availability with setup/teardown buffers
- [ ] `core`: hire types: hourly / half-day / day, with pricing
- [ ] `store`: migrations for `suite_bookings`, `events`, event links
- [ ] Suite booking: layout, attendees (≤ capacity for that layout), notes, catering requirements
- [ ] Event container: name, type (wedding/conference/dinner…), contact, date(s)
- [ ] Link suite bookings + room block (held rooms with release date) to an event
- [ ] Packages (e.g. wedding package: suite + per-head catering + room block rate)
- [ ] Catering numbers / dietary summary per event
- [ ] UI: suite calendar (day/week timeline per suite)
- [ ] UI: event page bringing everything together
- [ ] Floor-plan view shows suite occupancy for a chosen time
- [ ] Tests: buffer overlaps, capacity per layout, room-block release

## Acceptance
- Two suite bookings that collide only through buffers are rejected.
- An event page shows suites, rooms, numbers and value in one place.

## Notes / decisions
