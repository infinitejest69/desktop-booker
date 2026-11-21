//! Folios and invoices: charges, tax, payments and the balance.
//!
//! A *folio* is the running account for a booking: charges (rooms, board,
//! suites, bar, restaurant…) and payments (deposits, part payments,
//! refunds). Issuing an invoice freezes the folio's charges under a
//! sequential number.
//!
//! Tax is worked out **per line** and rounded to the penny, then summed, so
//! every line on the printed invoice adds up exactly. Prices can include
//! tax (usual for UK hotels: the guest sees £120 and £20 of it is VAT) or
//! have it added on top.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::ids::{ChargeId, PaymentId, TaxRateId};
use crate::money::Money;
use crate::validation::{ValidationError, ValidationResult};

/// Basis points in 100%: a rate of 2000 is 20%.
pub const BASIS_POINTS: u32 = 10_000;

/// A named tax rate, e.g. "Standard VAT" at 20% (2000 basis points).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TaxRate {
    pub id: TaxRateId,
    pub name: String,
    /// Hundredths of a percent: 2000 = 20%, 500 = 5%, 0 = zero-rated.
    pub basis_points: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum ChargeCategory {
    Accommodation,
    Board,
    Suite,
    Food,
    Drink,
    Other,
}

/// One line on a folio. The tax rate is copied onto the line when it's
/// added, so changing a rate later never changes old charges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Charge {
    pub id: ChargeId,
    pub date: NaiveDate,
    pub category: ChargeCategory,
    pub description: String,
    pub quantity: u16,
    /// Per item, including or excluding tax as the hotel's setting says.
    /// Negative for a discount or correction.
    pub unit_price: Money,
    pub tax_basis_points: u32,
}

/// A charge's money split three ways.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct LineAmounts {
    pub net: Money,
    pub tax: Money,
    pub gross: Money,
}

/// `value × numerator ÷ denominator`, rounded to the nearest penny with
/// halves rounded away from zero (so 2.5p → 3p and −2.5p → −3p).
/// `i128` gives head-room so large amounts can't overflow mid-calculation.
fn div_round(value: i64, numerator: u32, denominator: u32) -> i64 {
    let n = i128::from(value) * i128::from(numerator);
    let d = i128::from(denominator);
    let half = d / 2;
    let rounded = if n >= 0 {
        (n + half) / d
    } else {
        (n - half) / d
    };
    i64::try_from(rounded).unwrap_or(if n >= 0 { i64::MAX } else { i64::MIN })
}

/// Net, tax and gross for one charge, with tax rounded per line.
pub fn line_amounts(charge: &Charge, prices_include_tax: bool) -> LineAmounts {
    let amount = charge.unit_price * i64::from(charge.quantity);
    let rate = charge.tax_basis_points;
    if prices_include_tax {
        // gross = net × (1 + r), so tax = gross × r ÷ (1 + r).
        let tax = Money(div_round(amount.0, rate, BASIS_POINTS + rate));
        LineAmounts {
            net: amount - tax,
            tax,
            gross: amount,
        }
    } else {
        let tax = Money(div_round(amount.0, rate, BASIS_POINTS));
        LineAmounts {
            net: amount,
            tax,
            gross: amount + tax,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum PaymentMethod {
    Cash,
    Card,
    BankTransfer,
    Cheque,
    Voucher,
    Other,
}

impl PaymentMethod {
    pub const ALL: [PaymentMethod; 6] = [
        Self::Cash,
        Self::Card,
        Self::BankTransfer,
        Self::Cheque,
        Self::Voucher,
        Self::Other,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cash => "cash",
            Self::Card => "card",
            Self::BankTransfer => "bank_transfer",
            Self::Cheque => "cheque",
            Self::Voucher => "voucher",
            Self::Other => "other",
        }
    }
}

impl std::str::FromStr for PaymentMethod {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|m| m.as_str() == s)
            .ok_or_else(|| format!("unknown payment method {s:?}"))
    }
}

/// Money received (or, when negative, refunded). Recorded only: the app
/// never takes card payments itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Payment {
    pub id: PaymentId,
    pub date: NaiveDate,
    /// Positive for money in, negative for a refund.
    pub amount: Money,
    pub method: PaymentMethod,
    /// A deposit taken to secure the booking.
    pub is_deposit: bool,
    /// Card slip, transfer reference, cheque number…
    pub reference: String,
}

/// Tax for one rate, for the summary at the foot of an invoice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TaxSummary {
    pub basis_points: u32,
    pub net: Money,
    pub tax: Money,
}

/// What a folio adds up to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct FolioTotals {
    /// One entry per charge, in the same order.
    pub lines: Vec<LineAmounts>,
    pub net: Money,
    pub tax: Money,
    pub gross: Money,
    /// One entry per tax rate used, lowest rate first.
    pub by_rate: Vec<TaxSummary>,
    pub deposits: Money,
    pub paid: Money,
    /// Still owed by the guest; negative means the hotel owes a refund.
    pub balance: Money,
}

pub fn folio_totals(
    charges: &[Charge],
    payments: &[Payment],
    prices_include_tax: bool,
) -> FolioTotals {
    let lines: Vec<LineAmounts> = charges
        .iter()
        .map(|c| line_amounts(c, prices_include_tax))
        .collect();
    let mut by_rate: Vec<TaxSummary> = Vec::new();
    for (charge, line) in charges.iter().zip(&lines) {
        match by_rate
            .iter_mut()
            .find(|r| r.basis_points == charge.tax_basis_points)
        {
            Some(r) => {
                r.net += line.net;
                r.tax += line.tax;
            }
            None => by_rate.push(TaxSummary {
                basis_points: charge.tax_basis_points,
                net: line.net,
                tax: line.tax,
            }),
        }
    }
    by_rate.sort_by_key(|r| r.basis_points);
    let gross: Money = lines.iter().map(|l| l.gross).sum();
    let paid: Money = payments.iter().map(|p| p.amount).sum();
    FolioTotals {
        net: lines.iter().map(|l| l.net).sum(),
        tax: lines.iter().map(|l| l.tax).sum(),
        gross,
        deposits: payments
            .iter()
            .filter(|p| p.is_deposit)
            .map(|p| p.amount)
            .sum(),
        paid,
        balance: gross - paid,
        by_rate,
        lines,
    }
}

pub fn validate_tax_rate(rate: &TaxRate) -> ValidationResult {
    if rate.name.trim().is_empty() {
        return Err(ValidationError::Empty {
            field: "Tax rate name",
        });
    }
    if rate.basis_points > BASIS_POINTS {
        return Err(ValidationError::InvalidTaxRate);
    }
    Ok(())
}

pub fn validate_charge(charge: &Charge) -> ValidationResult {
    if charge.description.trim().is_empty() {
        return Err(ValidationError::Empty {
            field: "Description",
        });
    }
    if charge.quantity == 0 {
        return Err(ValidationError::Empty { field: "Quantity" });
    }
    if charge.tax_basis_points > BASIS_POINTS {
        return Err(ValidationError::InvalidTaxRate);
    }
    Ok(())
}

/// A payment must be for some money, and a refund can't give back more
/// than has been paid (`paid_so_far` excludes this payment).
pub fn validate_payment(payment: &Payment, paid_so_far: Money) -> ValidationResult {
    if payment.amount == Money::ZERO {
        return Err(ValidationError::Empty { field: "Amount" });
    }
    if payment.amount.0 < 0 && paid_so_far + payment.amount < Money::ZERO {
        return Err(ValidationError::RefundTooLarge);
    }
    Ok(())
}

/// e.g. "INV-000042". Numbers come from a counter in the store, taken in
/// the same transaction as the invoice, so there are no gaps.
pub fn format_invoice_number(prefix: &str, number: u32) -> String {
    format!("{}{number:06}", prefix.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn charge(description: &str, quantity: u16, pence: i64, rate: u32) -> Charge {
        Charge {
            id: ChargeId::new(),
            date: d("2026-06-20"),
            category: ChargeCategory::Other,
            description: description.into(),
            quantity,
            unit_price: Money(pence),
            tax_basis_points: rate,
        }
    }

    fn pay(pence: i64, deposit: bool) -> Payment {
        Payment {
            id: PaymentId::new(),
            date: d("2026-06-01"),
            amount: Money(pence),
            method: PaymentMethod::Card,
            is_deposit: deposit,
            reference: String::new(),
        }
    }

    #[test]
    fn tax_inside_a_price_is_split_out_to_the_penny() {
        // £120 including 20% VAT is £100 + £20.
        let l = line_amounts(&charge("Double room", 1, 12_000, 2000), true);
        assert_eq!(
            (l.net, l.tax, l.gross),
            (Money(10_000), Money(2_000), Money(12_000))
        );
        // £9.99 including 20%: tax 166.5p rounds to 167p.
        let l = line_amounts(&charge("Wine", 1, 999, 2000), true);
        assert_eq!((l.net, l.tax), (Money(832), Money(167)));
    }

    #[test]
    fn tax_on_top_is_added_and_rounded_per_line() {
        // 3 × £4.15 = £12.45; 20% = 249p.
        let l = line_amounts(&charge("Sandwiches", 3, 415, 2000), false);
        assert_eq!(
            (l.net, l.tax, l.gross),
            (Money(1_245), Money(249), Money(1_494))
        );
        // 5% on £0.50 = 2.5p rounds up to 3p.
        let l = line_amounts(&charge("Mints", 1, 50, 500), false);
        assert_eq!(l.tax, Money(3));
        // A discount rounds the same way, downwards.
        let l = line_amounts(&charge("Discount", 1, -50, 500), false);
        assert_eq!(l.tax, Money(-3));
    }

    #[test]
    fn a_folio_with_mixed_rates_matches_a_hand_calculation() {
        // Hand calculation (prices include tax):
        //   2 nights × £95.00 at 20%    = £190.00 → net £158.33, VAT £31.67
        //   Breakfast 2 × £12.00 at 20% = £24.00  → net £20.00,  VAT £4.00
        //   Newspaper £2.50 at 0%       = £2.50   → net £2.50,   VAT £0.00
        //   Total £216.50, VAT £35.67; deposit £50 and £100 paid → £66.50 due.
        let charges = [
            charge("Room 101", 2, 9_500, 2000),
            charge("Breakfast", 2, 1_200, 2000),
            charge("Newspaper", 1, 250, 0),
        ];
        let t = folio_totals(&charges, &[pay(5_000, true), pay(10_000, false)], true);
        assert_eq!(t.gross, Money(21_650));
        assert_eq!(t.tax, Money(3_567));
        assert_eq!(t.net, Money(21_650 - 3_567));
        assert_eq!(
            t.by_rate,
            vec![
                TaxSummary {
                    basis_points: 0,
                    net: Money(250),
                    tax: Money(0)
                },
                TaxSummary {
                    basis_points: 2000,
                    net: Money(17_833),
                    tax: Money(3_567)
                },
            ]
        );
        assert_eq!(t.deposits, Money(5_000));
        assert_eq!(t.paid, Money(15_000));
        assert_eq!(t.balance, Money(6_650));
    }

    #[test]
    fn overpaying_leaves_a_negative_balance_to_refund() {
        let room = [charge("Room", 1, 10_000, 2000)];
        let t = folio_totals(&room, &[pay(12_000, true)], true);
        assert_eq!(t.balance, Money(-2_000));
        let refund = Payment {
            amount: Money(-2_000),
            ..pay(0, false)
        };
        assert_eq!(validate_payment(&refund, t.paid), Ok(()));
        let t = folio_totals(&room, &[pay(12_000, true), refund], true);
        assert_eq!(t.balance, Money::ZERO);
    }

    #[test]
    fn payments_and_charges_are_checked() {
        assert_eq!(
            validate_payment(&pay(0, false), Money::ZERO),
            Err(ValidationError::Empty { field: "Amount" })
        );
        assert_eq!(
            validate_payment(&pay(-1_000, false), Money(500)),
            Err(ValidationError::RefundTooLarge)
        );
        assert_eq!(
            validate_charge(&charge("Bar", 0, 500, 2000)),
            Err(ValidationError::Empty { field: "Quantity" })
        );
        assert_eq!(
            validate_charge(&charge("Bar", 1, 500, 10_001)),
            Err(ValidationError::InvalidTaxRate)
        );
    }

    #[test]
    fn invoice_numbers_are_zero_padded() {
        assert_eq!(format_invoice_number("INV-", 42), "INV-000042");
        assert_eq!(format_invoice_number("", 7), "000007");
    }
}
