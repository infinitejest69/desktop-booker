import type { BookingStatus } from "../bindings/BookingStatus";

export const statusLabel: Record<BookingStatus, string> = {
  provisional: "Provisional",
  confirmed: "Confirmed",
  checked_in: "Checked in",
  checked_out: "Checked out",
  cancelled: "Cancelled",
  no_show: "No-show",
};

export interface StatusAction {
  to: BookingStatus;
  label: string;
  confirm?: string;
  variant?: "solid" | "destructive";
}

// Mirrors BookingStatus::can_change_to in booker-core (the backend re-checks).
export function actionsFor(status: BookingStatus): StatusAction[] {
  switch (status) {
    case "provisional":
      return [
        { to: "confirmed", label: "Confirm", variant: "solid" },
        { to: "checked_in", label: "Check in" },
        { to: "cancelled", label: "Cancel booking", variant: "destructive", confirm: "The room becomes free for other bookings." },
      ];
    case "confirmed":
      return [
        { to: "checked_in", label: "Check in", variant: "solid" },
        { to: "provisional", label: "Mark provisional" },
        { to: "no_show", label: "No-show", confirm: "The room becomes free for other bookings." },
        { to: "cancelled", label: "Cancel booking", variant: "destructive", confirm: "The room becomes free for other bookings." },
      ];
    case "checked_in":
      return [
        { to: "checked_out", label: "Check out", variant: "solid" },
        { to: "confirmed", label: "Undo check-in" },
      ];
    case "checked_out":
      return [{ to: "checked_in", label: "Undo check-out" }];
    case "cancelled":
    case "no_show":
      return [{ to: "confirmed", label: "Reinstate", variant: "solid" }];
  }
}
