// Store baskets, for the player's cart pill: each action the player queues (add, remove or
// read, on Juno or Bleep) is done here with the store's own basket requests, in the user's
// browser session, and nothing else (the store-carts design, D1). Beatport and Traxsource
// baskets need an account, so the player never asks for them. An outcome is only ever what
// the store's answer, or the basket read after it, says: anything else is "failed".

/* exported WRCarts */
var WRCarts = (() => {
  const JUNO = "https://www.juno.co.uk";
  const BLEEP = "https://bleep.com";
  const JUNO_PRODUCT = /^(\d{1,9})-(\d{2})$/;
  const BLEEP_ID = /^\d{1,10}$/;
  const MAX_BASKET = 500;

  function call(url, { method = "GET", headers = {}, body } = {}) {
    return fetch(url, {
      method,
      body,
      credentials: "include",
      cache: "no-store",
      // Both stores answer their own pages' scripts only when this is set (D1).
      headers: { "X-Requested-With": "XMLHttpRequest", ...headers },
    });
  }

  /** A price as the store shows it, as text the player takes (≤ 20 characters). */
  function price(s) {
    const t = String(s ?? "")
      .replace(/&#128;|&euro;/g, "€")
      .replace(/&pound;/g, "£")
      .replace(/<[^>]*>/g, "")
      .replace(/[\u0000-\u001f\u007f-\u009f]/g, "")
      .trim();
    return [...t].slice(0, 20).join("");
  }

  const has = (basket, item) => basket?.some((b) => b.item === item);

  // ---- Juno: a product `1170462-01` is the basket's `item[1170462-1]` ----------------------

  const junoItem = (title, n) => `${title}-${String(n).padStart(2, "0")}`;

  /** The basket page's items (no prices: the page's markup isn't worth reading for them). */
  async function junoBasket() {
    const r = await call(`${JUNO}/cart/`);
    if (!r.ok) return null;
    const html = await r.text();
    const out = [];
    for (const m of html.matchAll(/name="item\[(\d{1,9})-(\d{1,2})\]"/g)) {
      const item = junoItem(m[1], m[2]);
      if (!has(out, item)) out.push({ item, price: "" });
    }
    return out.slice(0, MAX_BASKET);
  }

  async function junoAdd(product) {
    const [, title, n] = product.match(JUNO_PRODUCT);
    const before = await junoBasket();
    if (!before) return { outcome: "failed" };
    // Juno asks before adding a second copy: an item already in is left alone.
    if (has(before, product)) return { outcome: "already_in", basket: before };
    const r = await call(`${JUNO}/cart/add/${title}/${+n}/?json=1`);
    const j = await r.json().catch(() => null);
    const items = j?.quickCart?.items;
    const basket = items
      ? Object.values(items)
          .map((i) => ({ item: junoItem(i.titleId, i.productId), price: price(i.unitPrice) }))
          .filter((b) => JUNO_PRODUCT.test(b.item))
          .slice(0, MAX_BASKET)
      : await junoBasket();
    if (has(basket, product)) return { outcome: "added", basket };
    if (/out of stock|sold out|no longer available/i.test(String(j?.cart?.errors ?? ""))) return { outcome: "sold_out", basket };
    return { outcome: "failed", basket };
  }

  async function junoRemove(product) {
    const [, title, n] = product.match(JUNO_PRODUCT);
    await call(`${JUNO}/cart/remove/?item%5B${title}-${+n}%5D=1&json=1`);
    const basket = await junoBasket();
    if (!basket) return { outcome: "failed" };
    return { outcome: has(basket, product) ? "failed" : "removed", basket };
  }

  // ---- Bleep: a format's product id; removing needs the basket line's id --------------------

  /** The basket's formats from Bleep's cart answer, with their line ids (kept here). */
  function bleepItems(j) {
    const symbol = j.symbol || j.currency || "";
    return j.items
      .filter((i) => i.cartableType === "ReleaseFormat" && Number.isInteger(i.id))
      .map((i) => {
        const p = j.cartDetailPrices?.[`com.bleep.shop.cart.CartDetail : ${i.id}`];
        const amount = p?.priceWithDiscount ?? p?.price;
        const shown = typeof amount === "number" ? `${amount.toFixed(2)} ${symbol}` : "";
        return { item: String(i.cartableId), price: price(shown), line: i.id };
      })
      .filter((b) => BLEEP_ID.test(b.item))
      .slice(0, MAX_BASKET);
  }

  async function bleepBasket() {
    const r = await call(`${BLEEP}/rest/cart`, { headers: { Accept: "application/json" } });
    const j = await r.json().catch(() => null);
    return Array.isArray(j?.items) ? bleepItems(j) : null;
  }

  async function bleepAdd(id) {
    const r = await call(`${BLEEP}/rest/cart`, {
      method: "POST",
      headers: { "Content-Type": "application/json; charset=utf-8", Accept: "application/json" },
      body: JSON.stringify({ item: { id: +id, type: "ReleaseFormat", qty: 1, bundled: [] } }),
    });
    const j = await r.json().catch(() => null);
    if (j?.error && /already/i.test(String(j.message))) {
      return { outcome: "already_in", basket: await bleepBasket() };
    }
    // An error page can come back with the format added anyway (D1): the basket decides.
    const basket = Array.isArray(j?.items) ? bleepItems(j) : await bleepBasket();
    return { outcome: has(basket, id) ? "added" : "failed", basket };
  }

  async function bleepRemove(id) {
    const before = await bleepBasket();
    if (!before) return { outcome: "failed" };
    const line = before.find((b) => b.item === id);
    if (line) await call(`${BLEEP}/cart/remove/${line.line}`);
    const basket = line ? await bleepBasket() : before;
    if (!basket) return { outcome: "failed" };
    return { outcome: has(basket, id) ? "failed" : "removed", basket };
  }

  // ---- the player's actions -----------------------------------------------------------------

  const read = (basket) => async () => {
    const b = await basket();
    return b ? { outcome: "read", basket: b } : { outcome: "failed" };
  };

  const STORES = {
    juno: { id: JUNO_PRODUCT, add: junoAdd, remove: junoRemove, read: read(junoBasket) },
    bleep: { id: BLEEP_ID, add: bleepAdd, remove: bleepRemove, read: read(bleepBasket) },
  };

  /** Does one action from `/v1/cart/next`: `{ outcome, basket? }` for `/v1/cart/result`. */
  async function perform(a) {
    const store = Object.hasOwn(STORES, a?.store) ? STORES[a.store] : null;
    const valid =
      store &&
      ["add", "remove", "read"].includes(a.kind) &&
      (a.kind === "read" ? a.item === "" : store.id.test(String(a.item)));
    if (!valid) return { outcome: "failed" };
    try {
      const r = await store[a.kind](a.item);
      // Only items and prices go back (Bleep's line ids stay here).
      const basket = r.basket?.map(({ item, price }) => ({ item, price }));
      return basket ? { outcome: r.outcome, basket } : { outcome: r.outcome };
    } catch {
      return { outcome: "failed" };
    }
  }

  return { perform, price };
})();
