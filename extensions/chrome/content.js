// On discogs.com: a button next to the page's title (floating in a corner when there is none)
// with Play in ‹App›, Enqueue in ‹App›, Send to crate (with a New crate… field) and the two
// switches. Only the address,
// the title's position and the document title (for New crate…'s suggested name) are read from
// the page; everything goes to the player through the service worker.

(() => {
  const STYLE = `
    :host { all: initial; }
    * { box-sizing: border-box; font: 12px/1.3 "Trebuchet MS", Tahoma, sans-serif; }
    .wrap { position: relative; display: inline-block; vertical-align: middle; margin: 6px 0; }
    .wrap.floating { position: fixed; right: 16px; bottom: 16px; z-index: 2147483646; margin: 0; }
    button.main {
      background: #1b1b24; color: #2fe45a; border: 1px solid #4a4a5e; border-radius: 3px;
      padding: 4px 10px; cursor: pointer; letter-spacing: .02em;
      box-shadow: inset 0 1px 0 #34344a;
    }
    button.main:hover { border-color: #2fe45a; }
    .menu {
      position: absolute; left: 0; top: calc(100% + 4px); min-width: 220px; z-index: 2147483647;
      background: #1b1b24; color: #d6f5dc; border: 1px solid #4a4a5e; border-radius: 3px;
      padding: 4px 0; box-shadow: 0 6px 18px rgba(0,0,0,.45);
    }
    .floating .menu { top: auto; bottom: calc(100% + 4px); left: auto; right: 0; }
    .item { display: block; width: 100%; text-align: left; padding: 5px 12px; background: none;
      border: 0; color: inherit; cursor: pointer; }
    .item:hover, .item:focus { background: #2a3a2e; color: #2fe45a; outline: none; }
    .sep { height: 1px; background: #33334a; margin: 4px 0; }
    .sub { padding-left: 24px; }
    .note { padding: 5px 12px; color: #8a8aa0; }
    label.item { display: flex; gap: 8px; align-items: center; }
    .toast {
      position: fixed; left: 50%; bottom: 24px; transform: translateX(-50%);
      z-index: 2147483647; max-width: 90vw; background: #1b1b24; color: #2fe45a;
      border: 1px solid #4a4a5e; border-radius: 3px; padding: 8px 14px;
      box-shadow: 0 6px 18px rgba(0,0,0,.45);
    }
    .toast.bad { color: #ff8a80; }
    .owned { display: block; margin-top: 3px; color: #2fe45a; font-size: 11px; }
    .owned.dim { color: #8a8aa0; }
    .field { display: flex; gap: 6px; padding: 4px 12px 2px 24px; }
    input.name {
      flex: 1; min-width: 0; background: #0f0f16; color: #d6f5dc; border: 1px solid #4a4a5e;
      border-radius: 3px; padding: 4px 6px; outline: none;
    }
    input.name:focus { border-color: #2fe45a; }
    button.create {
      background: #2a3a2e; color: #2fe45a; border: 1px solid #4a4a5e; border-radius: 3px;
      padding: 3px 8px; cursor: pointer;
    }
  `;

  let host = null; // the shadow host, while a button is shown
  let root = null;
  let appName = WR.PLACEHOLDER;
  let lastHref = null;

  const ask = (msg) =>
    new Promise((resolve) => {
      try {
        chrome.runtime.sendMessage(msg, (r) =>
          resolve(r || { ok: false, error: "not-running" }),
        );
      } catch {
        // The extension was reloaded: this old script can't reach it any more.
        resolve({ ok: false, error: "reload the page" });
      }
    });

  chrome.storage.local.get({ appName: WR.PLACEHOLDER }).then((s) => {
    appName = s.appName;
    render();
  });
  chrome.storage.onChanged.addListener((c) => {
    if (c.appName) {
      appName = c.appName.newValue || WR.PLACEHOLDER;
      render();
    }
  });

  function el(tag, props = {}, ...children) {
    const e = document.createElement(tag);
    Object.assign(e, props);
    for (const c of children) e.append(c);
    return e;
  }

  function toast(text, ok = true) {
    const t = el("div", { className: ok ? "toast" : "toast bad", textContent: text });
    ensureHost();
    root.append(t);
    setTimeout(() => t.remove(), 3000);
  }

  function ensureHost() {
    if (host && host.isConnected) return;
    host = el("span", { id: "wr-bridge-host" });
    root = host.attachShadow({ mode: "closed" });
    root.append(el("style", { textContent: STYLE }));
    place();
  }

  /** After the page's main heading, else floating in the bottom-right corner. */
  function place() {
    const h1 = document.querySelector("h1");
    const wrap = root.querySelector(".wrap");
    if (h1) {
      if (host.previousElementSibling !== h1 || !host.isConnected) h1.after(host);
      wrap?.classList.remove("floating");
    } else {
      if (host.parentElement !== document.body) document.body.append(host);
      wrap?.classList.add("floating");
    }
  }

  function removeButton() {
    root?.querySelector(".wrap")?.remove();
  }

  function render() {
    const kind = WR.pageKind(location.href);
    if (!kind) return removeButton();
    ensureHost();
    removeButton();
    const main = el("button", {
      className: "main",
      textContent: `▶ ${appName} ▾`,
      title: `Send this ${kind.toLowerCase()} to ${appName}`,
    });
    const wrap = el("div", { className: "wrap" }, main);
    root.append(wrap);
    place();
    if (kind === "Release" || kind === "Master" || kind === "Listing") {
      showOwned(wrap, location.href, true);
    }
    main.addEventListener("click", (e) => {
      e.stopPropagation();
      const open = wrap.querySelector(".menu");
      if (open) open.remove();
      else openMenu(wrap);
    });
  }

  /**
   * "✓ In your collection" under the button when the player says the page's record is owned
   * (only the address is sent). A marketplace item the player doesn't know yet is "checking":
   * it is asked once more, 3 s later.
   */
  async function showOwned(wrap, href, retry) {
    const r = await ask({ type: "owned", url: href });
    if (location.href !== href || !wrap.isConnected) return;
    const owned = r.ok ? r.json.owned : null;
    if (owned === "checking" && retry) {
      setTimeout(() => showOwned(wrap, href, false), 3000);
      return;
    }
    let text = null;
    let dim = false;
    if (owned === "no-token") {
      text = `Add a Discogs token in ${appName} (Options ▸ Discogs…) to see records you own`;
      dim = true;
    }
    if (owned === "this") text = "✓ In your collection";
    if (owned === "another") {
      const what = [r.json.catno, r.json.year].filter(Boolean).join(", ");
      text = `✓ Another pressing in your collection${what ? ` (${what})` : ""}`;
    }
    wrap.querySelector(".owned")?.remove();
    if (text) wrap.append(el("div", { className: dim ? "owned dim" : "owned", textContent: text }));
  }

  function closeMenus() {
    root?.querySelectorAll(".menu").forEach((m) => m.remove());
  }
  document.addEventListener("click", closeMenus);
  document.addEventListener("keydown", (e) => e.key === "Escape" && closeMenus());

  async function openMenu(wrap) {
    const menu = el("div", { className: "menu", role: "menu" });
    menu.addEventListener("click", (e) => e.stopPropagation());
    const item = (text, onClick, cls = "item") => {
      const b = el("button", { className: cls, textContent: text, role: "menuitem" });
      b.addEventListener("click", onClick);
      return b;
    };
    menu.append(
      item(`Play in ${appName}`, () => sendPage("play")),
      item(`Enqueue in ${appName}`, () => sendPage("enqueue")),
      el("div", { className: "sep" }),
      el("div", { className: "note", textContent: "Send to crate ▸" }),
    );
    const crates = el("div", {}, el("div", { className: "note sub", textContent: "…" }));
    menu.append(crates);
    const s = await chrome.storage.local.get({ vinylOnly: true, skipPassed: true });
    const toggle = (key, text) => {
      const box = el("input", { type: "checkbox", checked: s[key] });
      box.addEventListener("change", () => chrome.storage.local.set({ [key]: box.checked }));
      return el("label", { className: "item" }, box, text);
    };
    menu.append(
      el("div", { className: "sep" }),
      toggle("vinylOnly", "Vinyl only"),
      toggle("skipPassed", "Skip what I've passed"),
    );
    wrap.append(menu);

    // The crate list is fetched each time the menu opens.
    const r = await ask({ type: "crates" });
    crates.replaceChildren();
    if (!r.ok) {
      crates.append(el("div", { className: "note sub", textContent: WR.problem(r.error, appName) }));
      if (r.error === "not-paired") {
        crates.append(item("Pair…", () => ask({ type: "openOptions" }), "item sub"));
      }
      return;
    }
    for (const c of r.json.crates) {
      const mark = c.playing ? " ▶" : c.shown ? " ●" : "";
      crates.append(item(c.name + mark, () => sendPage("crate", c.name), "item sub"));
    }
    const newCrate = item("New crate…", () => newCrate.replaceWith(newCrateField()), "item sub");
    crates.append(newCrate);
  }

  /**
   * New crate…: a field in the menu holding the suggested name as real, editable text (the
   * cursor at its end). Enter or Create sends; Esc closes the menu.
   */
  function newCrateField() {
    const kind = WR.pageKind(location.href);
    const input = el("input", {
      type: "text",
      className: "name",
      value: WR.suggestName(document.title, kind, location.href),
      maxLength: 200,
      spellcheck: false,
    });
    const hint = el("div", { className: "note sub" });
    const create = el("button", { className: "create", textContent: "Create" });
    const submit = () => {
      const name = input.value.trim();
      const len = [...name].length;
      if (len === 0 || len > WR.MAX_NAME) {
        hint.textContent = `A crate name needs 1 to ${WR.MAX_NAME} characters (now ${len})`;
        input.focus();
        return;
      }
      sendPage("crate", name);
    };
    input.addEventListener("keydown", (e) => {
      e.stopPropagation(); // Discogs' own shortcuts stay out of the typing
      if (e.key === "Enter") submit();
      if (e.key === "Escape") closeMenus();
    });
    input.addEventListener("input", () => (hint.textContent = ""));
    create.addEventListener("click", submit);
    const row = el("div", { className: "field" }, input, create);
    const box = el("div", {}, row, hint);
    setTimeout(() => {
      input.focus();
      input.setSelectionRange(input.value.length, input.value.length);
    });
    return box;
  }

  async function sendPage(mode, crate) {
    closeMenus();
    const r = await ask({ type: "send", url: location.href, mode, crate });
    if (r.ok) toast(`Sent to ${appName}: ${r.json.page} → ${r.json.crate}`);
    else toast(WR.problem(r.error, appName), false);
  }

  // Discogs changes pages without reloading: follow the address, and put the button back if
  // the page redraws its heading.
  function tick() {
    if (location.href !== lastHref) {
      lastHref = location.href;
      closeMenus();
      render();
    } else if (host && root.querySelector(".wrap")) {
      if (!host.isConnected || document.querySelector("h1")?.nextElementSibling !== host) place();
    }
  }
  tick();
  setInterval(tick, 500);
})();
