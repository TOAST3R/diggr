// On www.beatport.com: reads the tracks the page in view shows, only when the button's menu
// opens, from the data the page was rendered from (Next.js: `#__NEXT_DATA__`, or the
// `self.__next_f` stream of newer pages). Beatport pages sit behind a bot challenge the player
// must not get around, so the extension reads them and the player never fetches them. Nothing
// here makes a network request or changes the page.
//
// The walk is tolerant: any object with a Beatport sample URL and an id is a track. The player
// checks every field again and rebuilds every address from the ids.

/* exported WRBeatport */
var WRBeatport = (() => {
  const MAX_TRACKS = 500;
  const UUID = /([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})/;
  const SAMPLE = new RegExp(`beatport\\.com/.*?${UUID.source}\\.LOFI\\.mp3`);
  const LOADED = location.href; // the page data describes this address only

  function clean(s, max) {
    const t = String(s ?? "")
      .replace(/[\u0000-\u001f\u007f-\u009f]/g, " ")
      .replace(/\s+/g, " ")
      .trim();
    return [...t].slice(0, max).join("");
  }

  /** Every JSON value the page was rendered from. */
  function payloads(doc) {
    const out = [];
    const next = doc.getElementById("__NEXT_DATA__");
    if (next) {
      try {
        out.push(JSON.parse(next.textContent));
      } catch {
        // not JSON: try the stream
      }
    }
    // App-router pages: `self.__next_f.push([1,"‹escaped text›"])`, lines of `id:JSON`.
    let stream = "";
    for (const s of doc.querySelectorAll("script:not([src])")) {
      for (const m of s.textContent.matchAll(/self\.__next_f\.push\(\[1,("(?:[^"\\]|\\.)*")\]\)/g)) {
        try {
          stream += JSON.parse(m[1]);
        } catch {
          // a chunk that isn't a string
        }
      }
    }
    for (const line of stream.split("\n")) {
      const at = line.search(/[[{]/);
      if (at < 0) continue;
      try {
        out.push(JSON.parse(line.slice(at)));
      } catch {
        // not a JSON line
      }
    }
    return out;
  }

  /** Objects that look like tracks, in document order, each id once. */
  function tracks(values) {
    const seen = new Map();
    const walk = (v, depth) => {
      if (!v || typeof v !== "object" || depth > 40 || seen.size >= MAX_TRACKS * 2) return;
      if (Array.isArray(v)) {
        for (const x of v) walk(x, depth + 1);
        return;
      }
      if (typeof v.sample_url === "string" && SAMPLE.test(v.sample_url) && v.id != null) {
        if (!seen.has(String(v.id))) seen.set(String(v.id), v);
      }
      for (const k in v) walk(v[k], depth + 1);
    };
    for (const v of values) walk(v, 0);
    return [...seen.values()];
  }

  const names = (list) =>
    (Array.isArray(list) ? list : [])
      .map((a) => clean(a?.name ?? a, 200))
      .filter(Boolean)
      .slice(0, 20);

  /** "8A" from `{ camelot_number: 8, camelot_letter: "A" }` or `{ camelot: "8A" }`. */
  function camelot(k) {
    if (!k || typeof k !== "object") return "";
    const code = k.camelot ?? (k.camelot_number != null ? `${k.camelot_number}${k.camelot_letter ?? ""}` : "");
    return /^(1[0-2]|[1-9])[AB]$/.test(String(code)) ? String(code) : "";
  }

  function seconds(t) {
    if (Number.isFinite(t.length_ms)) return Math.min(Math.round(t.length_ms / 1000), 7200);
    const m = String(t.length ?? "").match(/^(?:(\d+):)?(\d+):(\d{2})$/);
    return m ? Math.min((+m[1] || 0) * 3600 + +m[2] * 60 + +m[3], 7200) : 0;
  }

  function price(p) {
    if (!p) return "";
    if (typeof p === "string") return clean(p, 20);
    if (p.display) return clean(p.display, 20);
    if (p.value != null) return clean(`${p.symbol ?? ""}${p.value}${p.symbol ? "" : ` ${p.code ?? ""}`}`, 20);
    return "";
  }

  const uuid = (s) => (String(s ?? "").match(UUID) || [""])[0];
  const slug = (s) => (/^[a-z0-9-]{1,200}$/.test(String(s ?? "")) ? String(s) : "");

  function toListed(t) {
    const r = t.release && typeof t.release === "object" ? t.release : {};
    const image = r.image?.uri ?? r.image?.dynamic_uri ?? t.image?.uri ?? "";
    return {
      id: String(t.id),
      slug: slug(t.slug),
      artists: names(t.artists),
      title: clean(t.name ?? t.title, 200),
      mix: clean(t.mix_name, 200),
      remixers: names(t.remixers),
      sample: uuid(t.sample_url),
      bpm: Number.isInteger(t.bpm) && t.bpm >= 40 && t.bpm <= 250 ? t.bpm : null,
      key: camelot(t.key),
      secs: seconds(t),
      release: r.id != null ? String(r.id) : "",
      release_name: clean(r.name, 200),
      release_slug: slug(r.slug),
      label: clean(r.label?.name ?? t.label?.name, 200),
      catno: clean(r.catalog_number ?? t.catalog_number, 40),
      price: price(t.price),
      image: uuid(image),
    };
  }

  /**
   * The page's tracks as the player takes them, or null when none can be read (or the page
   * was reached without a reload, so its data describes another address). A release or
   * track page keeps only its own tracks, not the recommendations around them.
   */
  function read(doc, address) {
    if (address !== LOADED) return null;
    let list;
    try {
      list = tracks(payloads(doc)).map(toListed);
    } catch {
      return null;
    }
    const path = new URL(address).pathname;
    const own = path.match(/\/(release|track)\/[^/]+\/(\d+)/);
    if (own) {
      const mine = list.filter((t) => (own[1] === "release" ? t.release : t.id) === own[2]);
      if (mine.length > 0) list = mine;
    }
    list = list.filter((t) => t.sample && t.title && t.artists.length > 0).slice(0, MAX_TRACKS);
    if (list.length === 0) return null;
    return { page: address, tracks: list };
  }

  return { read, camelot, seconds };
})();
