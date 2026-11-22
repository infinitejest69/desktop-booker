// Tools for recording demo videos (Phase 11, `pnpm videos`).
//
// Recording: Chromium's own screencast (high-quality JPEG frames with
// timestamps) is turned into an H.264 MP4 at 30 frames a second by ffmpeg.
// It is sharper than Playwright's built-in video, whose low bitrate blurs text.
//
// Acting: the pointer glides to each control before clicking, typing is at
// a readable pace, and captions and title cards come from demo/stage.js.
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Locator, Page } from "@playwright/test";

/** Finished videos: videos/out/<name>.mp4 */
export const videoDir = fileURLToPath(new URL("../videos/out/", import.meta.url));
export const stageScript = fileURLToPath(new URL("./stage.js", import.meta.url));

const pause = (page: Page, ms: number) => page.waitForTimeout(ms);

/** Records everything `scene` does into videos/out/<name>.mp4. */
export async function record(page: Page, name: string, scene: () => Promise<void>): Promise<void> {
  mkdirSync(videoDir, { recursive: true });
  const frames = mkdtempSync(join(tmpdir(), `demo-${name}-`));
  const shots: { file: string; at: number }[] = [];
  const cdp = await page.context().newCDPSession(page);
  cdp.on("Page.screencastFrame", (frame) => {
    const file = join(frames, `${String(shots.length).padStart(6, "0")}.jpg`);
    writeFileSync(file, Buffer.from(frame.data, "base64"));
    shots.push({ file, at: frame.metadata.timestamp ?? Date.now() / 1000 });
    cdp.send("Page.screencastFrameAck", { sessionId: frame.sessionId }).catch(() => {});
  });
  await cdp.send("Page.startScreencast", { format: "jpeg", quality: 95, maxWidth: 1920, maxHeight: 1080 });
  try {
    await scene();
    await pause(page, 600);
  } finally {
    await cdp.send("Page.stopScreencast").catch(() => {});
  }
  // Frames only come when the screen changes, so each is held until the next.
  const list = shots
    .map((s, i) => `file '${s.file}'\nduration ${Math.max(0.001, (shots[i + 1]?.at ?? s.at + 1) - s.at).toFixed(3)}`)
    .join("\n");
  writeFileSync(join(frames, "list.txt"), `${list}\nfile '${shots.at(-1)?.file}'\n`);
  execFileSync("ffmpeg", [
    "-loglevel", "error", "-y", "-f", "concat", "-safe", "0", "-i", join(frames, "list.txt"),
    "-vf", "fps=30,scale=1920:1080:flags=lanczos,format=yuv420p",
    "-c:v", "libx264", "-preset", "slow", "-crf", "18", "-movflags", "+faststart",
    join(videoDir, `${name}.mp4`),
  ]);
  rmSync(frames, { recursive: true, force: true });
}

/** A full-screen title card, held for `ms`, then faded away. */
export async function card(page: Page, title: string, subtitle: string, ms = 3200, footer?: string): Promise<void> {
  await page.evaluate(([t, s, f]) => window.__stage.card(t, s, f), [title, subtitle, footer] as const);
  await pause(page, ms);
}

export async function uncard(page: Page): Promise<void> {
  await page.evaluate(() => window.__stage.uncard());
  await pause(page, 700);
}

/** A caption along the bottom, given time to be read. */
export async function say(page: Page, text: string, ms?: number): Promise<void> {
  await page.evaluate((t) => window.__stage.caption(t), text);
  await pause(page, ms ?? Math.max(2200, text.length * 55));
}

export async function quiet(page: Page): Promise<void> {
  await page.evaluate(() => window.__stage.caption(""));
}

/** Glides the pointer to the middle of `target`, at a watchable speed. */
export async function glide(page: Page, target: Locator): Promise<void> {
  await target.scrollIntoViewIfNeeded();
  const box = await target.boundingBox();
  if (!box) return;
  const from = await page.evaluate(() => window.__stage.pointer());
  const to = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  const distance = Math.hypot(to.x - from.x, to.y - from.y);
  await page.mouse.move(to.x, to.y, { steps: Math.max(12, Math.round(distance / 14)) });
  await pause(page, 250);
}

/** Moves to `target`, clicks it, and lets the result show. */
export async function click(page: Page, target: Locator, after = 700): Promise<void> {
  await glide(page, target);
  await target.click();
  await pause(page, after);
}

/** Clicks into a field and types at a readable pace. */
export async function type(page: Page, field: Locator, text: string): Promise<void> {
  await glide(page, field);
  await field.click();
  await field.fill("");
  await field.pressSequentially(text, { delay: 55 });
  await pause(page, 300);
}

/** Opens a select (combobox) and picks an option by its name. */
export async function choose(page: Page, combobox: Locator, option: string | RegExp): Promise<void> {
  await click(page, combobox, 400);
  await click(page, page.getByRole("option", { name: option }).first(), 500);
}

/** Scrolls smoothly so `target` comes into view. */
export async function scrollTo(page: Page, target: Locator, block: ScrollLogicalPosition = "center"): Promise<void> {
  await target.evaluate((el, b) => el.scrollIntoView({ behavior: "smooth", block: b }), block);
  await pause(page, 900);
}

declare global {
  interface Window {
    __stage: {
      caption(text: string): void;
      card(title: string, subtitle: string, footer?: string): void;
      uncard(): void;
      pointer(): { x: number; y: number };
    };
  }
}
