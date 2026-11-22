-- Phase 10: parking, charged per car per night like board.
-- The hotel's price (0 until set), the cars on each booked room, and the
-- parking part of each night's price, fixed when the room is booked.

ALTER TABLE property_settings ADD COLUMN parking_price INTEGER NOT NULL DEFAULT 0;
ALTER TABLE booking_rooms ADD COLUMN cars INTEGER NOT NULL DEFAULT 0;
ALTER TABLE room_nights ADD COLUMN parking_price INTEGER NOT NULL DEFAULT 0;
