// On www.juno.co.uk: reads the records the page in view shows, only when the button's menu
// opens, from markup the browser has already loaded. Juno pages sit behind a bot challenge the
// player must not get around, so the extension reads them and the player never fetches them.
// Nothing here makes a network request or changes the page.
//
// A record's product id, side and track all come from its sample links
// (…/MP3/SF‹product›-‹side›-‹track›.mp3), so a record without samples isn't sent. The player
// checks every field again and rebuilds every address from these ids.

/* exported WRJuno */
var WRJuno = (() => {
  const SAMPLE = /\/SF(\d{1,9}-\d{2})-(\d{2})-(\d{2})\.mp3(?:[?#]|$)/;
  const PRODUCT_LINK = /\/products\/([a-z0-9-]{1,200})\/(\d{1,9}-\d{2})\/?/;
  const IMAGE = /\/([A-Za-z0-9-]{1,40}\.jpg)(?:[?#]|$)/;
  const MAX_RECORDS = 500;
  const MAX_TRACKS = 60;

  /** Text cut to the player's limit, without control characters. */
  function clean(s, max) {
    const t = String(s || "")
      .replace(/[\u0000-\u001f\u007f-\u009f]/g, " ")
      .replace(/\s+/g, " ")
      .trim();
    return [...t].slice(0, max).join("");
  }

  /** "P0H5M1S" → 301. */
  function seconds(iso) {
    const m = String(iso || "").match(/^PT?(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?$/);
    if (!m) return 0;
    const s = (+m[1] || 0) * 3600 + (+m[2] || 0) * 60 + (+m[3] || 0);
    return Math.min(s, 7200);
  }

  /** A microdata property's value: content, then a link, then the text. */
  function value(e) {
    if (!e) return "";
    return e.getAttribute("content") ?? (e.hasAttribute("href") ? e.getAttribute("href") : e.textContent);
  }

  /** The elements carrying `name` that belong to `scope` itself, not to an item inside it. */
  function props(scope, name) {
    return [...scope.querySelectorAll(`[itemprop~="${name}"]`)].filter(
      (e) => e.parentElement.closest("[itemscope]") === scope,
    );
  }

  /** A property's text; for an item (byArtist, recordLabel), its name. */
  function prop(scope, name) {
    const e = props(scope, name)[0];
    if (!e) return "";
    if (e.hasAttribute("itemscope")) return value(props(e, "name")[0]) || e.textContent;
    return value(e);
  }

  function stock(s) {
    if (/InStock/i.test(s)) return "in";
    if (/OutOfStock|SoldOut|Discontinued/i.test(s)) return "out";
    if (/PreOrder/i.test(s)) return "preorder";
    return "";
  }

  /** Sample links in page order, one per sample: [{ product, side, n, a }]. */
  function samples(root) {
    const seen = new Set();
    const out = [];
    for (const a of root.querySelectorAll('a[href*=".mp3"]')) {
      const m = a.getAttribute("href").match(SAMPLE);
      if (!m || seen.has(m[0])) continue;
      seen.add(m[0]);
      out.push({ product: m[1], side: m[2], n: +m[3], a });
    }
    return out;
  }

  function imageName(root) {
    for (const img of root.querySelectorAll("img")) {
      const src = img.getAttribute("src") || img.getAttribute("data-src") || "";
      if (!src.includes("juno.co.uk")) continue;
      const m = src.match(IMAGE);
      if (m) return m[1];
    }
    return "";
  }

  function track(s, title, artist, secs) {
    return {
      side: String(+s.side),
      n: Math.min(Math.max(s.n, 1), 99),
      title: clean(title, 200),
      artist: clean(artist, 200),
      secs,
      sample: `${s.side}-${String(s.n).padStart(2, "0")}`,
    };
  }

  /** A product page: the release's microdata, "Cat:", and its samples paired with its tracks. */
  function readProduct(doc, address) {
    const link = new URL(address).pathname.match(PRODUCT_LINK);
    if (!link) return [];
    const [, slug, product] = link;
    const scope =
      [...doc.querySelectorAll("[itemscope]")].find((e) => props(e, "track").length > 0) ||
      doc.querySelector('[itemtype*="schema.org/Product"], [itemtype*="schema.org/MusicAlbum"]');
    if (!scope) return [];
    const tracks = props(scope, "track");
    const own = samples(doc).filter((s) => s.product === product);
    const artist = prop(scope, "byArtist");
    const offer = props(scope, "offers")[0] || scope;
    const cat = (doc.body.innerText.match(/Cat:\s*([^\n]+)/) || [])[1];
    return [
      {
        product,
        slug,
        artist: clean(artist, 200),
        title: clean(prop(scope, "name") || doc.querySelector("h1")?.textContent, 200),
        label: clean(prop(scope, "recordLabel") || prop(scope, "brand"), 200),
        catno: clean(cat, 40),
        format: "",
        stock: stock(prop(offer, "availability")),
        price: clean(props(offer, "price")[0]?.textContent, 20),
        image: imageName(doc),
        // Samples and tracks are listed in the same order.
        tracks: own.slice(0, MAX_TRACKS).map((s, i) => {
          const t = tracks[i];
          return t
            ? track(s, prop(t, "name"), prop(t, "byArtist"), seconds(prop(t, "duration")))
            : track(s, "", "", 0);
        }),
      },
    ];
  }

  /**
   * A listing (label, artist, genre, chart, search): its items, each found from its sample
   * links up to the smallest element that also links to the item's product page.
   */
  function readListing(doc) {
    const byProduct = new Map();
    for (const s of samples(doc)) {
      if (!byProduct.has(s.product)) byProduct.set(s.product, []);
      byProduct.get(s.product).push(s);
    }
    const records = [];
    for (const [product, own] of byProduct) {
      if (records.length === MAX_RECORDS) break;
      let item = own[0].a.parentElement;
      let link = null;
      while (item && item !== doc.body) {
        link = [...item.querySelectorAll('a[href*="/products/"]')].find(
          (a) => a.getAttribute("href").match(PRODUCT_LINK)?.[2] === product,
        );
        if (link) break;
        item = item.parentElement;
      }
      if (!link) continue;
      const slug = link.getAttribute("href").match(PRODUCT_LINK)[1];
      const text = (sel) => item.querySelector(sel)?.textContent || "";
      const price = (item.innerText.match(/[£€$]\s?\d+(?:[.,]\d{2})?/) || [""])[0];
      records.push({
        product,
        slug,
        artist: clean(text('a[href*="/artists/"]'), 200),
        title: clean(link.textContent, 200),
        label: clean(text('a[href*="/labels/"]'), 200),
        catno: "",
        format: "",
        stock: "",
        price: clean(price, 20),
        image: imageName(item),
        tracks: own
          .slice(0, MAX_TRACKS)
          .map((s) => track(s, s.a.getAttribute("title") || "", "", 0)),
      });
    }
    return records;
  }

  /**
   * The page's records as the player takes them, or null when none can be read. `single` is
   * true when every record shows one track only.
   */
  function read(doc, address, kind) {
    let records;
    try {
      records = kind === "JunoProduct" ? readProduct(doc, address) : readListing(doc);
    } catch {
      return null;
    }
    records = records.filter((r) => r.artist && r.title && r.tracks.length > 0);
    if (records.length === 0) return null;
    return {
      page: address,
      records,
      single: records.every((r) => r.tracks.length === 1),
    };
  }

  return { read, seconds };
})();
