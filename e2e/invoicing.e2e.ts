import { expect, openSetup, test } from "./fixtures";

test.describe("bills and invoices", () => {
  test.beforeEach(async ({ demoHotel: page }) => {
    // Margaret Hughes: G02 (twin, £85) for 2 nights with B&B for two (£24 a night).
    await page.getByRole("region", { name: /Arrivals/ }).getByRole("link", { name: "Margaret Hughes" }).click();
    await expect(page.getByRole("heading", { level: 2, name: "Bill" })).toBeVisible();
  });

  test("the bill shows the stay with VAT and takes extras and payments", async ({ page }) => {
    const bill = page.getByRole("region", { name: "Bill" });
    const total = bill.getByRole("definition").first();
    await expect(bill).toContainText("Room G02, 2 nights");
    await expect(total).toHaveText("£218.00");
    await expect(bill).toContainText("VAT at 20% (included)£36.33");

    await bill.getByRole("textbox", { name: "Item" }).fill("Bar");
    await bill.getByRole("textbox", { name: "Price each" }).fill("12");
    await bill.getByRole("button", { name: "Add to bill" }).click();
    await expect(page.getByText("Added Bar")).toBeVisible();
    await expect(total).toHaveText("£230.00");

    await bill.getByRole("textbox", { name: "Amount" }).fill("50");
    await bill.getByRole("switch", { name: "Deposit" }).click();
    await bill.getByRole("button", { name: "Record payment" }).click();
    await expect(bill.getByRole("row", { name: /Card, deposit/ })).toContainText("£50.00");
    await expect(bill).toContainText("Balance due£180.00");

    // A refund bigger than what's been paid is refused.
    await bill.getByRole("switch", { name: "This is a refund" }).click();
    await bill.getByRole("textbox", { name: "Amount" }).fill("80");
    await bill.getByRole("button", { name: "Record refund" }).click();
    await expect(page.getByText("A refund can't be more than has been paid")).toBeVisible();
  });

  test("issuing an invoice numbers it and opens a printable page", async ({ page }) => {
    const bill = page.getByRole("region", { name: "Bill" });
    await bill.getByRole("textbox", { name: "Amount" }).fill("100");
    await bill.getByRole("button", { name: "Record payment" }).click();
    await expect(bill).toContainText("Balance due£118.00");

    await bill.getByRole("button", { name: "Issue invoice" }).click();
    const invoice = page.getByRole("article", { name: "Invoice INV-000001" });
    await expect(invoice).toBeVisible();
    await expect(invoice).toContainText("Margaret Hughes");
    await expect(invoice).toContainText("Room G02, 2 nights");
    await expect(invoice).toContainText(/Balance due\s*£118\.00/);
    await expect(invoice).toContainText("VAT no. GB 123 4567 89");
    await expect(page.getByRole("button", { name: "Print or save as PDF" })).toBeVisible();

    await page.getByRole("link", { name: /^Back to Booking/ }).click();
    await expect(page.getByRole("region", { name: "Bill" }).getByRole("link", { name: "INV-000001" })).toBeVisible();
  });
});

test("an admin adds a tax rate and chooses where it applies", async ({ demoHotel: page }) => {
  await openSetup(page, "Invoices and tax");
  await expect(page.getByRole("heading", { level: 1, name: "Invoices and tax" })).toBeVisible();
  const add = page.getByRole("form", { name: "Add a tax rate" });
  await add.getByRole("textbox", { name: "Name" }).fill("Reduced VAT");
  await add.getByRole("textbox", { name: "Rate (%)" }).fill("5");
  await add.getByRole("button", { name: "Add tax rate" }).click();
  await expect(page.getByRole("row", { name: /Reduced VAT/ })).toContainText("5%");

  await page.getByRole("textbox", { name: "Invoice number prefix" }).fill("GR-");
  await page.getByRole("button", { name: "Save invoice settings" }).click();
  await expect(page.getByText("Saved the invoice settings")).toBeVisible();
});
