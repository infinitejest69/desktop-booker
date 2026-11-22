# Phase 10 — Rooms, screens and demo data

**Goal:** a batch of features and design changes Andy listed on 2026-11-21. Recorded here so they aren't lost. Started 2026-11-21 with the easier screen changes (Andy: "tackle some of the easier wins in phase 10"). Room features and parking followed (2026-11-22). The review alert and richer demo data are still to do.

## Tasks
### Rooms
- [x] **Room features:** wifi, pets allowed, accessible, smoking (non-smoking unless ticked). Set per room in Floors and layout; icons on the room's tile, card and calendar row; a *Room must have* filter when booking.
- [x] **Parking:** a booking option priced like board: a price per car per night (Rates), *Cars to park* on each room of a booking, its own line on the bill.
- [x] **Floor plan renamed to Rooms:** every floor shown at once in one scrolling list (no floor tabs). *Icons come with room features above.*

### Bookings, suites and events
- [x] **Search on the booking calendar:** the calendar gets the list view's search bar; when a search is typed, the results show in the list view.
- [x] **Function suites and Events combined** like Bookings: one menu item with two views.

### Dashboard
- [x] **One clean page with minimal scrolling:** *The hotel today* at the top, then arrivals and departures. Keep the events widget. Remove the *In house* widget, the *Coming weeks* calendar and *Coming up*.
- [ ] **Review alert:** a small one-line widget that says when a new review has been left on Google or Tripadvisor.

### Demo data
- [ ] **Richer demo hotel:** many more guests, bookings across past and future months (all statuses and sources), events and suite hire, invoices and payments, housekeeping history. Enough to make reports, the calendar and the dashboard look realistic.

## Open questions (answer before starting)
- **Review alert:** the app is offline-first, and both sites need an online account to read reviews. Google needs the Business Profile API (OAuth sign-in and Google's approval). Tripadvisor's Content API needs a partner key and limits what can be shown. A simpler option is to check the review email notifications in the hotel's Gmail, which the app already uses for sending. Needs an ADR.
- **Dashboard:** should staff and admins see the same dashboard? (Phase 9 could let an admin choose a custom dashboard for the home page.)

## Notes / decisions
**Parking, built (2026-11-22):** Andy chose option 1 (a booking option priced like board) over a plain "parking included" note.
- Core: `PropertySettings.parking_price` (per car per night, zero until set, can't be negative); `BookingRoom.cars` (at most `MAX_CARS` = 10); `NightPrice.parking`; `quote_stay` takes the cars and price, so parking is part of each night's total. On the bill (`stay_charges`) it's its own line, "Parking for room 101, 2 nights from 19 Jun", taxed at the *Other* VAT rate (no separate parking rate to set up). A hand-set ("agreed") price includes everything, parking too, as it already did board.
- Store: migration `0010_parking.sql` adds `property_settings.parking_price`, `booking_rooms.cars` and `room_nights.parking_price`. Like board, the parking price is fixed on each night when the room is priced; a booking is priced again only when its room, dates, guests, board or cars change. Revenue: parking counts under *Extras*.
- Screens: *Parking* on the Rates page (saved with the hotel settings); *Cars to park* next to Adults and Children on each room of a booking; a Parking column in the price breakdown; "parking for 2 cars" on the booking summary.

**Room features, built (2026-11-22):** Andy's list: pets allowed, wifi, disabled (accessible) room, non-smoking / smoking, and parking (see the open task).
- Core: `RoomFeature` (`Wifi`, `PetsAllowed`, `Accessible`, `Smoking`) and `Space.features`; a room without `Smoking` is non-smoking, so there's no separate non-smoking flag to keep in step. Suites can't have features (`SuiteHasFeatures`). Stored as text by `features_to_text` / `features_from_text` (sorted, no repeats).
- Store: migration `0009_room_features.sql` adds `spaces.features` (comma-separated, default empty), a column rather than a table because the features belong to the room row and change with it.
- Demo hotel: wifi everywhere, G01–G02 accessible, G03–G05 pets allowed, 216 the smoking room.
- Screens: checkboxes in Floors and layout (new rooms start with wifi); `src/lib/rooms/FeatureIcons.svelte` draws the four as line icons (no icon library), each named for screen readers and in a tooltip; on Rooms tiles and cards, the Rooms side panel (in words, "Wifi, pets allowed, non-smoking"), and the booking calendar's room column (widened to 8.5rem). Booking form: *Room must have* (wifi, pets allowed, accessible, non-smoking, smoking) per room line; rooms that don't match are greyed out as "(doesn't match)". The filter isn't saved with the booking.

**Easier wins, built (2026-11-21):**
- *View switch:* `BookingViews` became the general `src/lib/nav/ViewSwitch.svelte`; the views of each menu item are listed in `src/lib/nav/views.ts`.
- *Suites and events:* one menu item, **Suites and events**, opening the suite diary first (like Bookings opens the calendar); a *Suites / Events* switch on both pages (labelled "Suites or events", so it doesn't clash with the diary's Day / Week switch). Routes unchanged (`/suites`, `/events`).
- *Calendar search:* a *Search bookings* box on the calendar; Enter opens the List view with the results (`/bookings?q=…`). The list keeps its search in the address, so Back from a booking returns to the results.
- *Dashboard:* order is now the glance sentence, *The hotel today*, Arrivals and Departures side by side, then Events. *In house*, *Coming weeks* and *Coming up* are gone (in-house guests are on Rooms and the calendar). With the demo hotel it fits a 1400 × 900 window without scrolling.
- *Rooms:* menu item and heading renamed (route stays `/floor-plan`); floors are stacked sections, each with its own heading and *Mark floor clean / ready*; the colour key shows once at the top.

- Created 2026-11-21 at Andy's request: "add a todo list of features and changes but not tackle them just yet".
