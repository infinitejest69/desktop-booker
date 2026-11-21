import { expect, openSetup, test } from "./fixtures";

test("first run creates the admin and signs them in", async ({ page }) => {
  await page.goto("/?fake=setup");
  await expect(page.getByRole("heading", { name: "Welcome" })).toBeVisible();
  await page.getByRole("textbox", { name: "Your name" }).fill("Gwen");
  await page.getByLabel("Choose a PIN").fill("2468");
  await page.getByLabel("Type the PIN again").fill("2460");
  await page.getByRole("button", { name: "Create admin and sign in" }).click();
  await expect(page.getByText("The two PINs don't match.")).toBeVisible();

  await page.getByLabel("Type the PIN again").fill("2468");
  await page.getByRole("button", { name: "Create admin and sign in" }).click();
  await expect(page.getByText("Signed in as Gwen (admin)")).toBeVisible();
});

test.describe("lock screen", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/?fake=locked");
    await expect(page.getByRole("heading", { name: "Who's at the desk?" })).toBeVisible();
  });

  test("a wrong PIN is refused and the right one signs in", async ({ page }) => {
    await page.getByRole("radio", { name: "Sam" }).check();
    await page.getByLabel("PIN").fill("0000");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page.getByText("Wrong PIN. 4 tries left.")).toBeVisible();

    await page.getByLabel("PIN").fill("5678");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page.getByText("Signed in as Sam")).toBeVisible();
  });

  test("staff don't see setup or hand-set prices, and the backend refuses them", async ({ page }) => {
    await page.getByRole("radio", { name: "Sam" }).check();
    await page.getByLabel("PIN").fill("5678");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page.getByText("Signed in as Sam")).toBeVisible();
    await expect(page.getByRole("button", { name: "My hotel" })).toHaveCount(0);

    await page.getByRole("link", { name: "Bookings", exact: true }).click();
    await page.getByRole("button", { name: "New booking" }).click();
    await expect(page.getByRole("heading", { name: "New booking" })).toBeVisible();
    await expect(page.getByText("Set the price by hand")).toHaveCount(0);

    // Calling the command directly, as a determined user could, is refused too.
    const refused = await page.evaluate(async () => {
      const tauri = (window as unknown as { __TAURI_INTERNALS__: { invoke: (c: string, a: object) => Promise<unknown> } })
        .__TAURI_INTERNALS__;
      try {
        await tauri.invoke("save_rate_rule", { rule: {} });
        return "allowed";
      } catch (e) {
        return (e as { kind: string }).kind;
      }
    });
    expect(refused).toBe("forbidden");
  });

  test("five wrong PINs pause sign-in with a live countdown", async ({ page }) => {
    await page.clock.install();
    await page.reload();
    await page.getByRole("radio", { name: "Sam" }).check();
    const pin = page.getByLabel("PIN");
    for (let i = 0; i < 5; i++) {
      await pin.fill("0000");
      await page.getByRole("button", { name: "Sign in" }).click();
    }
    const wait = page.getByRole("status").filter({ hasText: "Too many wrong PINs" });
    await expect(wait).toContainText("try again in 30 seconds");
    await expect(pin).toBeDisabled();
    await expect(page.getByRole("button", { name: "Sign in" })).toBeDisabled();

    await page.clock.runFor(10_000);
    await expect(wait).toContainText("try again in 20 seconds");

    // Someone else can still sign in meanwhile.
    await page.getByRole("radio", { name: "Andy" }).check();
    await expect(pin).toBeEnabled();
    await page.getByRole("radio", { name: "Sam" }).check();
    await expect(pin).toBeDisabled();

    await page.clock.runFor(21_000);
    await expect(wait).toHaveCount(0);
    await expect(pin).toBeEnabled();
    await expect(pin).toBeFocused();
    await pin.fill("5678");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page.getByText("Signed in as Sam")).toBeVisible();
  });

  test("switch user goes back to the lock screen", async ({ page }) => {
    await page.getByRole("radio", { name: "Andy" }).check();
    await page.getByLabel("PIN").fill("1234");
    await page.getByRole("button", { name: "Sign in" }).click();
    await page.getByRole("button", { name: "Switch user" }).click();
    await expect(page.getByRole("heading", { name: "Who's at the desk?" })).toBeVisible();
  });
});

test("an admin adds a user and sees it in the audit log", async ({ demoHotel: page }) => {
  await openSetup(page, "Users");
  await page.getByRole("textbox", { name: "Name" }).fill("Rhys");
  await page.getByLabel("PIN").fill("1357");
  await page.getByRole("button", { name: "Add user" }).click();
  await expect(page.getByText("Added Rhys")).toBeVisible();
  await expect(page.getByRole("cell", { name: "Rhys" })).toBeVisible();

  await openSetup(page, "Audit log");
  await expect(page.getByRole("cell", { name: "Added staff member Rhys" })).toBeVisible();
  await expect(page.getByRole("row", { name: /Added staff member Rhys/ })).toContainText("Andy");
});
