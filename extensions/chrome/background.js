// The service worker: every call to the player's bridge goes through here (the key never
// reaches a web page), plus the link context menus, the toolbar badge and the store baskets.

importScripts("pages.js", "carts.js");

const DEFAULTS = {
  port: WR.DEFAULT_PORT,
  key: null,
  appName: WR.PLACEHOLDER,
  skipPassed: true,
};

function settings() {
  return chrome.storage.local.get(DEFAULTS);
}

/**
 * Calls the bridge. Resolves to { ok, status, json } or { ok: false, error }, where error is
 * "not-running", "not-paired", "unsupported" or another short reason.
 */
async function bridge(path, { method = "GET", body, auth = true } = {}) {
  const s = await settings();
  if (auth && !s.key) return { ok: false, error: "not-paired" };
  const headers = {};
  if (body !== undefined) headers["Content-Type"] = "application/json";
  if (auth) headers["X-Bridge-Key"] = s.key;
  let resp;
  try {
    resp = await fetch(`http://127.0.0.1:${s.port}/v1/${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
      cache: "no-store",
      credentials: "omit",
    });
  } catch {
    return { ok: false, error: "not-running" };
  }
  const json = await resp.json().catch(() => ({}));
  if (resp.ok) return { ok: true, status: resp.status, json };
  const error =
    {
      401: path === "pair" ? "wrong-code" : "not-paired",
      410: "no-code",
      422: json.error === "unsupported" ? "unsupported" : "invalid",
      429: "locked",
    }[resp.status] || json.error || `HTTP ${resp.status}`;
  return { ok: false, status: resp.status, error };
}

/** The player's name, remembered so labels follow it even while it's closed. */
async function hello() {
  const r = await bridge("hello", { auth: false });
  if (r.ok && typeof r.json.app === "string" && r.json.app) {
    await chrome.storage.local.set({ appName: r.json.app });
  }
  return r;
}

async function send(url, mode, crate, title) {
  const s = await settings();
  const body = { url, mode, skip_passed: s.skipPassed };
  if (mode === "crate") body.crate = crate;
  // Only a Bandcamp page's title, to name its label.
  if (title && WR.isBandcamp(WR.pageKind(url))) body.title = [...title].slice(0, 200).join("");
  return bridge("send", { method: "POST", body });
}

/**
 * The records read from a Juno or Bleep page, or the tracks from a Beatport or Traxsource one
 * (the player checks each one again).
 */
async function sendStore(store, page, items, mode, crate) {
  const s = await settings();
  const records = store === "Juno" || store === "Bleep";
  const payload = records ? { page, records: items } : { page, tracks: items };
  const body = { [store.toLowerCase()]: payload, mode, skip_passed: s.skipPassed };
  if (mode === "crate") body.crate = crate;
  return bridge("send", { method: "POST", body });
}

async function pair(code, port) {
  await chrome.storage.local.set({ port });
  const r = await bridge("pair", { method: "POST", body: { code }, auth: false });
  if (r.ok) {
    await chrome.storage.local.set({ key: r.json.key });
    await hello();
  }
  return r;
}

async function handle(msg) {
  switch (msg.type) {
    case "hello":
      return hello();
    case "crates":
      return bridge("crates");
    case "status": {
      // Running and paired are told apart: hello needs no key.
      const h = await hello();
      if (!h.ok) return h;
      return bridge("status");
    }
    case "send": {
      const r = await send(msg.url, msg.mode, msg.crate, msg.title);
      if (r.error === "not-paired") chrome.runtime.openOptionsPage();
      return r;
    }
    case "sendStore": {
      if (WR.storeOf(WR.pageKind(msg.page)) !== msg.store) return { ok: false, error: "unsupported" };
      const r = await sendStore(msg.store, msg.page, msg.items, msg.mode, msg.crate);
      if (r.error === "not-paired") chrome.runtime.openOptionsPage();
      return r;
    }
    case "owned":
      // Quietly: no options page when unpaired, the button just shows no note.
      return bridge("owned", { method: "POST", body: { url: msg.url } });
    case "pair":
      return pair(msg.code, msg.port);
    case "unpair":
      await chrome.storage.local.set({ key: null });
      return { ok: true };
    case "openOptions":
      chrome.runtime.openOptionsPage();
      return { ok: true };
    default:
      return { ok: false, error: "unknown message" };
  }
}

chrome.runtime.onMessage.addListener((msg, sender, reply) => {
  // Only this extension's own pages and content scripts talk to it.
  if (sender.id !== chrome.runtime.id) return false;
  handle(msg).then(reply);
  return true;
});

// ---- links on any site ----------------------------------------------------------------------

async function buildMenus() {
  const { appName } = await settings();
  await chrome.contextMenus.removeAll();
  for (const [id, verb] of [
    ["play", "Play"],
  ]) {
    chrome.contextMenus.create({
      id,
      title: `${verb} in ${appName}`,
      contexts: ["link"],
      targetUrlPatterns: WR.LINK_PATTERNS,
    });
  }
  // A label link only follows the label (the player decides that from the address).
  chrome.contextMenus.create({
    id: "label",
    title: `${appName}: Send label`,
    contexts: ["link"],
    targetUrlPatterns: WR.LABEL_LINK_PATTERNS,
  });
}

chrome.runtime.onInstalled.addListener((details) => {
  buildMenus();
  hello();
  // Every send keeps every format now: the old vinyl-only switch is forgotten.
  chrome.storage.local.remove("vinylOnly");
  if (details.reason === "install") chrome.runtime.openOptionsPage();
});
chrome.runtime.onStartup.addListener(buildMenus);
chrome.storage.onChanged.addListener((changes) => {
  if (changes.appName) buildMenus();
});

let badgeTimer = null;

/** ✓ or ! on the toolbar button for 3 s: the page itself isn't ours to show anything on. */
async function badge(ok) {
  await chrome.action.setBadgeBackgroundColor({ color: ok ? "#1f9d3a" : "#c62828" });
  await chrome.action.setBadgeText({ text: ok ? "✓" : "!" });
  clearTimeout(badgeTimer);
  badgeTimer = setTimeout(() => chrome.action.setBadgeText({ text: "" }), 3000);
}

chrome.contextMenus.onClicked.addListener(async (info) => {
  if (!info.linkUrl) return;
  // The label item sends as an enqueue: the player follows the label from the address.
  const mode = info.menuItemId === "label" ? "enqueue" : info.menuItemId;
  const r = await send(info.linkUrl, mode);
  badge(r.ok);
  // The popup shows what happened to the last link sent.
  const { appName } = await settings();
  const text = r.ok
    ? r.json.message ?? `Sent to ${appName}: ${r.json.page} → ${r.json.crate}`
    : WR.problem(r.error, appName);
  await chrome.storage.session.set({ lastLink: { ok: r.ok, text } });
  if (r.error === "not-paired") chrome.runtime.openOptionsPage();
});

// ---- store baskets --------------------------------------------------------------------------

// While the player runs, a wait for its next cart action is always open (the player holds it
// up to 25 s), which also keeps this worker awake; each action is done and answered at once.
let cartWaiting = false;

async function cartLoop() {
  if (cartWaiting) return;
  cartWaiting = true;
  try {
    for (;;) {
      const r = await bridge("cart/next");
      if (!r.ok) {
        // Not running or not paired (or a player without baskets): look again later.
        await new Promise((ok) => setTimeout(ok, r.status === 404 ? 60_000 : 5_000));
        continue;
      }
      const action = r.json.action;
      if (!action) continue;
      const out = await WRCarts.perform(action);
      await bridge("cart/result", { method: "POST", body: { id: action.id, ...out } });
    }
  } finally {
    cartWaiting = false;
  }
}

// Every time the worker starts (installed, browser started, woken by a page or the popup).
cartLoop();
