// On bleep.com: reads the records the page in view shows, only when the button's menu opens
// (or a record's ▶ is clicked), from markup the browser has already loaded. Bleep pages sit
// behind a bot challenge the player must not get around, so the extension reads them and the
// player never fetches them. Nothing here makes a network request or changes the page (the
// basket requests are `carts.js`', made only for the player's cart pill).
//
// A release page (/release/‹id›-‹name›) shows its formats with prices and its tracks; a list
// (label, artist, search…) shows tiles with no tracks, which the player brings in from Discogs.
// The player checks every field again.

/* exported WRBleep */
var WRBleep = (() => {
  const MAX_RECORDS = 500;
  const RELEASE = /\/release\/(\d{1,10})-([a-z0-9-]{1,200})/;
  const IMAGE = /\/r\/s\/(\d{1,10}(?:-\d{1,3})?\.jpg)/;

  function clean(s, max) {
    const t = String(s ?? "")
      .replace(/[\u0000-\u001f\u007f-\u009f]/g, " ")
      .replace(/\s+/g, " ")
      .trim();
    return [...t].slice(0, max).join("");
  }

  const text = (root, sel, max = 200) => clean(root.querySelector(sel)?.textContent, max);

  function seconds(s) {
    const m = String(s ?? "").match(/(?:(\d+):)?(\d{1,2}):(\d{2})/);
    return m ? Math.min((+m[1] || 0) * 3600 + +m[2] * 60 + +m[3], 7200) : 0;
  }

  /** What every record has: ids, names, label, catalogue number and cover. */
  function base(root, id, slug) {
    return {
      id,
      slug,
      artist: text(root, "dd.artist [itemprop=name]") || text(root, "dd.artist"),
      title: text(root, "dd.release-title"),
      label: text(root, "dd.label"),
      catno: text(root, "dd.catalogue-number", 40),
      image: (root.querySelector("img")?.getAttribute("src") || "").match(IMAGE)?.[1] ?? "",
    };
  }

  /** A release page: the main product, its formats with their first price, its tracks. */
  function readRelease(doc, address) {
    const m = new URL(address).pathname.match(RELEASE);
    const product = doc.querySelector(`#product-${m?.[1]}`);
    if (!m || !product) return [];
    // A format's product id is on its first buy button (the variant its price shows), for
    // the player's cart pill.
    const formats = [...product.querySelectorAll(".product-formats-list > li")].slice(0, 8).map((f) => ({
      name: text(f, ".media-format-name", 30),
      price: text(f, ".price-wrapper", 20),
      id: f.querySelector('.buy-button[data-itemtype="ReleaseFormat"]')?.dataset.itemid ?? "",
    }));
    const tracks = [...doc.querySelectorAll(`#track-list-${m[1]} li.track[data-id]`)]
      .slice(0, 60)
      .map((t) => ({
        id: t.dataset.id,
        n: +text(t, ".track-number") || 0,
        title: text(t, ".js-play-link"),
        artist: text(t, ".track-artist"),
        secs: seconds(text(t, ".track-duration")),
      }))
      .filter((t) => /^\d{1,10}$/.test(t.id) && t.n >= 1 && t.n <= 99 && t.title);
    const sold = formats.filter((f) => f.name).map((f) => (/^\d{1,10}$/.test(f.id) ? f : { ...f, id: "" }));
    return [{ ...base(product, m[1], m[2]), formats: sold, tracks }];
  }

  /** A list's tile: no tracks, only the names of its formats. */
  function readTile(tile) {
    const link = [...tile.querySelectorAll('a[href*="/release/"]')]
      .map((a) => a.getAttribute("href").match(RELEASE))
      .find(Boolean);
    if (!link || link[1] !== tile.dataset.id) return null;
    const formats = [...tile.querySelectorAll(".available-formats-list li.format")]
      .slice(0, 8)
      .map((f) => ({ name: clean(f.textContent, 30) }))
      .filter((f) => f.name);
    return { ...base(tile, link[1], link[2]), formats, tracks: [] };
  }

  function tiles(doc) {
    return [...doc.querySelectorAll('li.product[data-type="ReleaseProduct"][data-id]')];
  }

  function readList(doc) {
    const seen = new Set();
    const out = [];
    for (const tile of tiles(doc)) {
      const r = readTile(tile);
      if (!r || seen.has(r.id)) continue;
      seen.add(r.id);
      out.push(r);
      if (out.length === MAX_RECORDS) break;
    }
    return out;
  }

  /** The page's records as the player takes them, or null when none can be read. */
  function read(doc, address) {
    let records;
    try {
      records = RELEASE.test(new URL(address).pathname) ? readRelease(doc, address) : readList(doc);
    } catch {
      return null;
    }
    records = records.filter((r) => r.artist && r.title);
    return records.length ? { page: address, records } : null;
  }

  return { read, readTile, tiles, seconds };
})();
