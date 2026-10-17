// On discogs.com and Bandcamp: a button next to the page's title (floating in a corner when
// there is none) with Play in ‹App› (added to the crate on screen and played), Send to crate (with a New crate… field)
// and the switch; a label's page has only Send label, a Bandcamp fan's page only Import
// collection & wishlist. Only the address, the title's position
// and the document title (for New crate…'s suggested name, and a Bandcamp label's name) are
// read from the page; everything goes to the player through the service worker. On Juno,
// Beatport and Traxsource, the records or tracks the page shows are read too (juno.js,
// beatport.js, traxsource.js), and only when the menu opens.

(() => {
  const STYLE = `
    :host { all: initial; }
    * { box-sizing: border-box; font: 12px/1.3 "Trebuchet MS", Tahoma, sans-serif; }
    .wrap { position: relative; display: inline-block; vertical-align: middle; margin: 6px 0; }
    :host(.beside) .wrap { margin: 0 0 0 10px; }
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

  /** The page's title: Discogs' main heading, or Bandcamp's album, track or label name. */
  function heading() {
    // Juno: after the record's Play / Add to playlist buttons; its first h1 is the breadcrumb,
    // whose bar would clip the menu.
    if (WR.isJuno(WR.pageKind(location.href))) {
      // Visible ones only: the page holds hidden copies (other layouts, per-track menus).
      const buttons = [...document.querySelectorAll("button, a, [role=button]")].filter(
        (b) => b.getClientRects().length > 0 && b.offsetWidth > 0,
      );
      const text = (b) => b.textContent.replace(/\s+/g, " ").trim();
      const at = buttons.findIndex((b) => /^play$/i.test(text(b)));
      if (at >= 0) {
        // Its Add to playlist is the next visible button.
        const next = buttons[at + 1];
        return next && /^add to playlist$/i.test(text(next)) ? next : buttons[at];
      }
      return document.querySelector("h1");
    }
    if (WR.isBandcamp(WR.pageKind(location.href))) {
      return (
        document.querySelector("#name-section .trackTitle") ||
        document.querySelector("#band-name-location .title") ||
        document.querySelector("#name-section h2")
      );
    }
    return document.querySelector("h1");
  }

  /** After the page's title, else floating in the bottom-right corner. */
  function place() {
    const h1 = heading();
    const wrap = root.querySelector(".wrap");
    if (h1) {
      if (host.previousElementSibling !== h1 || !host.isConnected) h1.after(host);
      host.classList.toggle("beside", h1.tagName !== "H1" && WR.isJuno(WR.pageKind(location.href)));
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
      title:
        kind === "Seller"
          ? `Add this seller to ${appName}'s Top Sellers`
          : kind === "Label" || kind === "BandcampLabel"
            ? `Follow this label in ${appName}: it goes under LABELS`
            : kind === "BandcampFan"
              ? `Import this fan's Bandcamp collection & wishlist into ${appName}`
              : WR.storeOf(kind)
                ? `Send the tracks on this ${WR.storeOf(kind)} page to ${appName}`
                : `Send this ${kind.replace("Bandcamp", "").toLowerCase()} to ${appName}`,
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
    // A seller's page only adds the seller (or refreshes it): no tracks, no crate to pick.
    if (WR.pageKind(location.href) === "Seller") {
      menu.append(item(`Add seller to ${appName}`, () => sendPage("enqueue")));
      wrap.append(menu);
      return;
    }
    // A fan's page imports their collection and wishlist (only the address is sent).
    if (WR.pageKind(location.href) === "BandcampFan") {
      menu.append(
        item(`${appName}: Import collection & wishlist`, () => sendPage("enqueue")),
      );
      wrap.append(menu);
      return;
    }
    // A label's page follows the label under LABELS (or refreshes it): nothing else to pick.
    if (["Label", "BandcampLabel"].includes(WR.pageKind(location.href))) {
      menu.append(item(`${appName}: Send label`, () => sendPage("enqueue")));
      wrap.append(menu);
      return;
    }
    // A store's page is read now, and what was read is what gets sent.
    let juno = null;
    const store = WR.storeOf(WR.pageKind(location.href));
    if (store) {
      // Each store's page loads only its own reader (WRJuno, WRBeatport, WRTraxsource).
      const reader = self[`WR${store}`];
      const read = reader.read(document, location.href, WR.pageKind(location.href));
      if (!read) {
        // Beatport's page data describes the address it was loaded at.
        const why = store === "Beatport" ? ": reload it, then try again" : "";
        menu.append(el("div", { className: "note", textContent: `Couldn't read this ${store} page${why}` }));
        wrap.append(menu);
        return;
      }
      juno = { store, ...read };
      const n = (read.records || read.tracks).length;
      const unit = read.records ? "record" : "track";
      menu.append(el("div", { className: "note", textContent: n === 1 ? `1 ${unit}` : `${n} ${unit}s` }));
      if (read.single && n > 1) {
        const hint = "1 track per record: set Show tracks to All for more";
        menu.append(el("div", { className: "note", textContent: hint }));
      }
      menu.append(el("div", { className: "sep" }));
    }
    const sendThis = (mode, crate) => send(mode, crate, juno);
    menu.append(
      item(`Play in ${appName}`, () => sendThis("play")),
      el("div", { className: "sep" }),
      el("div", { className: "note", textContent: "Send to crate ▸" }),
    );
    const crates = el("div", {}, el("div", { className: "note sub", textContent: "…" }));
    menu.append(crates);
    const s = await chrome.storage.local.get({ skipPassed: true });
    const toggle = (key, text) => {
      const box = el("input", { type: "checkbox", checked: s[key] });
      box.addEventListener("change", () => chrome.storage.local.set({ [key]: box.checked }));
      return el("label", { className: "item" }, box, text);
    };
    menu.append(
      el("div", { className: "sep" }),
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
      crates.append(item(c.name + mark, () => sendThis("crate", c.name), "item sub"));
    }
    const newCrate = item(
      "New crate…",
      () => newCrate.replaceWith(newCrateField(sendThis, juno)),
      "item sub",
    );
    crates.append(newCrate);
  }

  /**
   * New crate…: a field in the menu holding the suggested name as real, editable text (the
   * cursor at its end). Enter or Create sends; Esc closes the menu.
   */
  function newCrateField(sendThis, juno) {
    const kind = WR.pageKind(location.href);
    // Juno: "Artist - Title" for a product; any store's other page: its heading.
    const r = juno?.records?.[0];
    const value = !WR.storeOf(kind)
      ? WR.suggestName(document.title, kind, location.href)
      : kind === "JunoProduct" && r
        ? WR.fit(`${r.artist} - ${r.title}`)
        : WR.fit((document.querySelector("h1")?.textContent || "").replace(/\s+/g, " ").trim());
    const input = el("input", {
      type: "text",
      className: "name",
      value,
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
      sendThis("crate", name);
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

  const sendPage = (mode, crate) => send(mode, crate, null);

  async function send(mode, crate, juno) {
    closeMenus();
    if (juno) {
      const { store, page, records, tracks } = juno;
      const r = await ask({ type: "sendStore", store, page, items: records || tracks, mode, crate });
      if (!r.ok) return toast(WR.problem(r.error, appName), false);
      const refused = r.json.refused ? ` (${r.json.refused} refused)` : "";
      return toast(`Sent to ${appName}: ${r.json.page} → ${r.json.crate}${refused}`);
    }
    // A Bandcamp label is named after its page's title ("Music | Analogical Force").
    const title = WR.pageKind(location.href) === "BandcampLabel" ? document.title : undefined;
    const r = await ask({ type: "send", url: location.href, mode, crate, title });
    // Only the app bundle sends `front`: opening it brings the player forward on macOS.
    if (r.ok && r.json.front === "diggr://show") location.href = r.json.front;
    if (r.ok && r.json.message) toast(`${appName}: ${r.json.message}`);
    else if (r.ok) toast(`Sent to ${appName}: ${r.json.page} → ${r.json.crate}`);
    else toast(WR.problem(r.error, appName), false);
  }

  // Discogs changes pages without reloading: follow the address, and put the button back if
  // the page redraws its heading.
  // A Discogs page listing records for sale (a seller's shop, the marketplace): a small
  // ▶ after each record's title plays it in the app (sent as its marketplace item's address).
  const ITEM = /\/(?:[a-z]{2}\/)?(?:shop|sell)\/item\/(\d+)/;
  const PLAY_STYLE = `
    :host { all: initial; }
    button { font: 11px/1 "Trebuchet MS", Tahoma, sans-serif; margin-left: 6px; padding: 2px 6px;
      background: #1b1b24; color: #2fe45a; border: 1px solid #4a4a5e; border-radius: 3px;
      cursor: pointer; vertical-align: middle; }
    button:hover { border-color: #2fe45a; }
  `;

  function itemButtons() {
    if (!/(^|\.)discogs\.com$/.test(location.hostname) || ITEM.test(location.pathname)) return;
    const seen = new Set();
    for (const a of document.querySelectorAll('a[href*="/item/"]')) {
      const m = (a.getAttribute("href") || "").match(ITEM);
      // The title link only: one button per record, none on its picture.
      if (!m || seen.has(m[1]) || !a.textContent.trim()) continue;
      seen.add(m[1]);
      if (a.nextElementSibling?.classList.contains("wr-item-play")) continue;
      const span = el("span", { className: "wr-item-play" });
      const shadow = span.attachShadow({ mode: "closed" });
      const b = el("button", { textContent: "▶", title: `Play in ${appName}` });
      b.addEventListener("click", async (e) => {
        e.preventDefault();
        e.stopPropagation();
        const url = new URL(`/sell/item/${m[1]}`, location.origin).href;
        const r = await ask({ type: "send", url, mode: "play" });
        if (r.ok) toast(`Playing in ${appName}: ${a.textContent.trim().slice(0, 80)}`);
        else toast(WR.problem(r.error, appName), false);
      });
      shadow.append(el("style", { textContent: PLAY_STYLE }), b);
      a.after(span);
    }
  }

  function tick() {
    itemButtons();
    if (location.href !== lastHref) {
      lastHref = location.href;
      closeMenus();
      render();
    } else if (host && root.querySelector(".wrap")) {
      if (!host.isConnected || heading()?.nextElementSibling !== host) place();
    }
  }
  tick();
  setInterval(tick, 500);
})();
