-- Phase 10: what each bedroom offers (wifi, pets allowed, accessible, smoking),
-- as a comma-separated list such as 'wifi,pets_allowed'. Empty for suites
-- and for rooms with none. Rooms without 'smoking' are non-smoking.

ALTER TABLE spaces ADD COLUMN features TEXT NOT NULL DEFAULT '';
