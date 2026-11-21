import type { ChargeCategory } from "../bindings/ChargeCategory";
import type { PaymentMethod } from "../bindings/PaymentMethod";

export const categoryLabel: Record<ChargeCategory, string> = {
  accommodation: "Accommodation",
  board: "Meals with the room",
  suite: "Suite hire",
  food: "Food",
  drink: "Drinks",
  other: "Other",
};

export const methodLabel: Record<PaymentMethod, string> = {
  cash: "Cash",
  card: "Card",
  bank_transfer: "Bank transfer",
  cheque: "Cheque",
  voucher: "Voucher",
  other: "Other",
};

/** 2000 basis points → "20%", 550 → "5.5%". */
export function percent(basisPoints: number): string {
  return `${Number((basisPoints / 100).toFixed(2))}%`;
}
