# Phase 10 — Rooms, screens and demo data (backlog)

**Goal:** a batch of features and design changes Andy listed on 2026-11-21. Recorded here so they aren't lost. **Not started; don't begin until Andy says so.** Order and grouping can change when the phase starts.

## Tasks
### Rooms
- [ ] **Room features:** accessible toilet / bathroom, non-smoking, pet friendly and similar. Set them per room in Floors and layout. Each feature has an icon shown on the room's tile, card and booking screens. (Probably a list of features in core, a `space_features` table, and a filter when choosing a room for a booking.)
- [ ] **Floor plan renamed to Rooms:** every floor shown at once in one scrolling list (no floor tabs), with a nicer design including icons (room features, housekeeping state, guests).

### Bookings, suites and events
- [ ] **Search on the booking calendar:** the calendar gets the list view's search bar; when a search is typed, the results show in the list view.
- [ ] **Function suites and Events combined** like Bookings: one menu item with two views.

### Dashboard
- [ ] **One clean page with minimal scrolling:** *The hotel today* at the top, then arrivals and departures. Keep the events widget. Remove the *In house* widget, the *Coming weeks* calendar and *Coming up*.
- [ ] **Review alert:** a small one-line widget that says when a new review has been left on Google or Tripadvisor.

### Demo data
- [ ] **Richer demo hotel:** many more guests, bookings across past and future months (all statuses and sources), events and suite hire, invoices and payments, housekeeping history. Enough to make reports, the calendar and the dashboard look realistic.

## Open questions (answer before starting)
- **Room features:** which features exactly, and should they also affect price or search (for example "find a pet-friendly room")?
- **Suites + events combined:** which view opens first (suite calendar or events list)?
- **Review alert:** the app is offline-first, and both sites need an online account to read reviews. Google needs the Business Profile API (OAuth sign-in and Google's approval). Tripadvisor's Content API needs a partner key and limits what can be shown. A simpler option is to check the review email notifications in the hotel's Gmail, which the app already uses for sending. Needs an ADR.
- **Dashboard:** should staff and admins see the same dashboard? (Phase 9 could let an admin choose a custom dashboard for the home page.)

## Notes / decisions
- Created 2026-11-21 at Andy's request: "add a todo list of features and changes but not tackle them just yet".
