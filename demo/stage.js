// The "stage" for demo videos (Phase 11): a visible pointer that follows the
// mouse, a ripple on each click, captions along the bottom and full-screen
// title cards. Injected into every page with page.addInitScript, so it is
// recorded together with the app. `window.__stage` is driven from
// demo/director.ts.
(() => {
  const css = `
    #stage-pointer { position: fixed; left: 0; top: 0; z-index: 2147483647; pointer-events: none;
      width: 30px; height: 30px; filter: drop-shadow(0 2px 3px rgb(0 0 0 / 0.35)); }
    .stage-ripple { position: fixed; z-index: 2147483646; pointer-events: none; width: 44px; height: 44px;
      margin: -22px 0 0 -22px; border-radius: 50%; border: 3px solid #b99bd8; background: rgb(94 63 122 / 0.25);
      animation: stage-ripple 0.55s ease-out forwards; }
    @keyframes stage-ripple { from { transform: scale(0.3); opacity: 1 } to { transform: scale(1.5); opacity: 0 } }
    #stage-caption { position: fixed; left: 50%; bottom: 56px; transform: translateX(-50%); z-index: 2147483645;
      pointer-events: none; max-width: 72%; padding: 16px 30px; border-radius: 14px; background: rgb(28 20 38 / 0.9);
      color: #fff; font: 600 28px/1.35 "Atkinson Hyperlegible Next Variable", system-ui, sans-serif; text-align: center;
      opacity: 0; transition: opacity 0.3s; }
    #stage-caption.on { opacity: 1; }
    #stage-card { position: fixed; inset: 0; z-index: 2147483644; display: flex; flex-direction: column;
      align-items: center; justify-content: center; gap: 22px; background: #5e3f7a; color: #fff; text-align: center;
      opacity: 0; transition: opacity 0.5s; pointer-events: none; }
    #stage-card.on { opacity: 1; }
    #stage-card h1 { margin: 0; color: #fff; font: 400 84px/1.1 "Young Serif", Georgia, serif; }
    #stage-card p { margin: 0; color: #fff; font: 500 32px/1.4 "Atkinson Hyperlegible Next Variable", system-ui, sans-serif; opacity: 0.9; }
    #stage-card small { margin-top: 30px; color: #fff; font: 500 22px "Atkinson Hyperlegible Next Variable", system-ui, sans-serif; opacity: 0.75; }`;
  const pointer = `<svg viewBox="0 0 24 24" width="30" height="30"><path d="M4 2l15 11-6.5 1.2L16 21l-3 1.3-3.3-6.7L5 20z"
    fill="#fff" stroke="#1c1426" stroke-width="1.4" stroke-linejoin="round"/></svg>`;

  // A card or caption carries across full page loads within one scene.
  const remember = (key, value) => {
    try {
      if (value === null) sessionStorage.removeItem(key);
      else sessionStorage.setItem(key, JSON.stringify(value));
    } catch {}
  };
  const recall = (key) => {
    try {
      return JSON.parse(sessionStorage.getItem(key) ?? "null");
    } catch {
      return null;
    }
  };
  let at = recall("stage-pointer") ?? { x: 960, y: 540 };

  function build() {
    if (document.getElementById("stage-pointer")) return;
    const style = document.createElement("style");
    style.textContent = css;
    document.head.append(style);
    const p = document.createElement("div");
    p.id = "stage-pointer";
    p.innerHTML = pointer;
    const caption = document.createElement("div");
    caption.id = "stage-caption";
    const card = document.createElement("div");
    card.id = "stage-card";
    document.body.append(card, caption, p);
    place(at.x, at.y);
    const c = recall("stage-card");
    if (c) showCard(c, false);
    const t = recall("stage-caption");
    if (t) showCaption(t);
  }

  function place(x, y) {
    at = { x, y };
    const p = document.getElementById("stage-pointer");
    if (p) p.style.transform = `translate(${x - 6}px, ${y - 3}px)`;
  }

  function showCaption(text) {
    const el = document.getElementById("stage-caption");
    remember("stage-caption", text || null);
    if (!el) return;
    if (text) el.textContent = text;
    el.classList.toggle("on", !!text);
  }

  function showCard(c, fade = true) {
    const el = document.getElementById("stage-card");
    remember("stage-card", c);
    if (!el) return;
    if (c) {
      el.innerHTML = `<h1></h1><p></p>${c.footer ? "<small></small>" : ""}`;
      el.querySelector("h1").textContent = c.title;
      el.querySelector("p").textContent = c.subtitle ?? "";
      if (c.footer) el.querySelector("small").textContent = c.footer;
    }
    if (!fade) el.style.transition = "none";
    el.classList.toggle("on", !!c);
    const p = document.getElementById("stage-pointer");
    if (p) p.style.visibility = c ? "hidden" : "";
    if (!fade) requestAnimationFrame(() => (el.style.transition = ""));
  }

  document.addEventListener("mousemove", (e) => {
    place(e.clientX, e.clientY);
    remember("stage-pointer", at);
  }, true);
  document.addEventListener("mousedown", (e) => {
    const r = document.createElement("div");
    r.className = "stage-ripple";
    r.style.left = `${e.clientX}px`;
    r.style.top = `${e.clientY}px`;
    document.body.append(r);
    setTimeout(() => r.remove(), 700);
  }, true);

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", build);
  else build();

  window.__stage = {
    caption: showCaption,
    card: (title, subtitle, footer) => showCard({ title, subtitle, footer }),
    uncard: () => showCard(null),
    pointer: () => at,
  };
})();
