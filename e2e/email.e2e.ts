import { expect, openSetup, test } from "./fixtures";

async function setUpGmail(page: import("@playwright/test").Page) {
  await openSetup(page, "Email");
  const account = page.getByRole("form", { name: "Email account" });
  await account.getByRole("button", { name: "Use Gmail" }).click();
  await expect(account.getByRole("textbox", { name: "Mail server" })).toHaveValue("smtp.gmail.com");
  await account.getByRole("textbox", { name: "User name" }).fill("grange.hotel@example.com");
  await account.getByLabel("Password").fill("abcd efgh ijkl mnop");
  await account.getByRole("textbox", { name: "From address" }).fill("grange.hotel@example.com");
  await account.getByRole("textbox", { name: "From name" }).fill("The Grange");
  await account.getByRole("button", { name: "Save email account" }).click();
  await expect(page.getByText("Saved the email settings")).toBeVisible();
  await expect(account).toContainText("A password is saved in this computer's keychain.");
}

test("Gmail is set up and a test email is sent and logged", async ({ demoHotel: page }) => {
  await setUpGmail(page);
  const test = page.getByRole("form", { name: "Send a test email" });
  await test.getByRole("textbox", { name: "Send a test to" }).fill("andy@example.com");
  await test.getByRole("button", { name: "Send test email" }).click();
  await expect(page.getByText("Sent a test email to andy@example.com")).toBeVisible();
  await expect(page.getByRole("row", { name: /andy@example.com/ })).toContainText("Sent");
});

test("a booking confirmation needs the account set up, then is sent", async ({ demoHotel: page }) => {
  const openMargaret = async () => {
    await page.getByRole("navigation", { name: "Main" }).getByRole("link", { name: "Dashboard" }).click();
    await page.getByRole("region", { name: /Arrivals/ }).getByRole("link", { name: "Margaret Hughes" }).click();
  };
  await openMargaret();
  const email = page.getByRole("region", { name: "Email the guest" });
  await expect(email).toContainText("Sends to margaret@example.com");
  await email.getByRole("button", { name: "Send confirmation" }).click();
  await expect(page.getByText("No password is saved for this email account.").first()).toBeVisible();
  await expect(email).toContainText("not sent");

  await setUpGmail(page);
  await openMargaret();
  await page.getByRole("region", { name: "Email the guest" }).getByRole("button", { name: "Send confirmation" }).click();
  await expect(page.getByText("Emailed margaret@example.com")).toBeVisible();
  await expect(page.getByRole("region", { name: "Email the guest" })).toContainText("sent by Andy");
});

test("reports can be saved as files", async ({ demoHotel: page }) => {
  await page.getByRole("navigation", { name: "Main" }).getByRole("link", { name: "Reports" }).click();
  await page.getByRole("button", { name: "Save as CSV…" }).first().click();
  await expect(page.getByText("/home/demo/occupancy.csv")).toBeVisible();
});
