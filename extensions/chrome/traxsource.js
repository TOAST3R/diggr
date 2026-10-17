// On www.traxsource.com: reads the tracks the page in view shows, only when the button's menu
// opens, from markup the browser has already loaded. Traxsource pages sit behind a bot
// challenge the player must not get around, so the extension reads them and the player never
// fetches them. Nothing here makes a network request or changes the page.
//
// A track is found from its link (/track/‹id›/‹slug›), its row being the largest element
// around it that links to no other track. The player checks every field again and previews
// the tracks by searching YouTube.

/* exported WRTraxsource */
var WRTraxsource = (() => {
  const MAX_TRACKS = 500;
  const TRACK = /\/track\/(\d{1,10})\/([a-z0-9-]{1,200})/;
  const TITLE = /\/title\/(\d{1,10})\/([a-z0-9-]{1,200})/;
  const KEY = /\b([A-G][#b♯♭]?\s?(?:min|maj)(?:or)?)(?![a-z])/i;

  function clean(s, max) {
    const t = String(s ?? "")
      .replace(/[\u0000-\u001f\u007f-\u009f]/g, " ")
      .replace(/\s+/g, " ")
      .trim();
    return [...t].slice(0, max).join("");
  }

  const shown = (e) => e?.innerText ?? e?.textContent ?? "";

  const trackOf = (a) => (a.getAttribute("href") || "").match(TRACK);

  /** The ids of the tracks `e` links to. */
  function ids(e) {
    return new Set([...e.querySelectorAll('a[href*="/track/"]')].map((a) => trackOf(a)?.[1]).filter(Boolean));
  }

  /** Climbs from `start` while the element still holds one track only. */
  function row(start, id, stop) {
    let e = start;
    while (e.parentElement && e.parentElement !== stop) {
      const up = ids(e.parentElement);
      if (up.size > 1 || (up.size === 1 && !up.has(id))) break;
      e = e.parentElement;
    }
    return e;
  }

  function seconds(text) {
    const m = text.match(/\(?\b(?:(\d+):)?(\d{1,2}):(\d{2})\b\)?/);
    return m ? Math.min((+m[1] || 0) * 3600 + +m[2] * 60 + +m[3], 7200) : 0;
  }

  function fields(e, id, slug, title) {
    const texts = (sel) => [...new Set([...e.querySelectorAll(sel)].map((a) => clean(a.textContent, 200)).filter(Boolean))];
    const cell = shown(e.querySelector(".key-bpm")) || null;
    const keyBpm = cell ?? shown(e);
    // Only a number in the key/BPM cell, labelled BPM, or right after the key: rows also
    // show dates, prices and positions.
    const bpm = +(
      (cell && cell.match(/(?<!\d)(\d{2,3})(?!\d)/)) ||
      keyBpm.match(/BPM\D{0,3}(\d{2,3})\b/i) ||
      keyBpm.match(new RegExp(`${KEY.source}[\\s|/,]*(\\d{2,3})(?!\\d)`, "i"))?.slice(1) ||
      []
    )[1];
    const version = e.querySelector(".version")?.cloneNode(true);
    version?.querySelectorAll(".duration").forEach((d) => d.remove());
    const release = [...e.querySelectorAll('a[href*="/title/"]')].map((a) => a.getAttribute("href").match(TITLE)).find(Boolean);
    return {
      id,
      slug,
      artists: texts('a[href*="/artist/"]').slice(0, 20),
      title: clean(title, 200),
      mix: clean(version?.textContent.replace(/\(\d+:\d{2}\)/, ""), 200),
      remixers: [],
      sample: "",
      bpm: bpm >= 40 && bpm <= 250 ? bpm : null,
      key: clean((keyBpm.match(KEY) || [""])[0], 20),
      secs: seconds(e.querySelector(".duration")?.textContent ?? shown(e)),
      release: release?.[1] ?? "",
      release_name: "",
      release_slug: release?.[2] ?? "",
      label: texts('a[href*="/label/"]')[0] ?? "",
      catno: "",
      price: clean((shown(e).match(/[$£€]\s?\d+[.,]\d{2}/) || [""])[0], 20),
      image: "",
    };
  }

  /** A track page: its heading's details, the track's id from the address. */
  function readTrack(doc, address) {
    const m = new URL(address).pathname.match(TRACK);
    const h1 = doc.querySelector("h1");
    if (!m || !h1) return [];
    // The details block: the heading's ancestors up to the first that shows the BPM.
    let e = h1;
    for (let i = 0; i < 6 && e.parentElement && !KEY.test(shown(e)); i++) e = e.parentElement;
    const title = h1.cloneNode(true);
    title.querySelectorAll(".version").forEach((v) => v.remove());
    return [fields(e, m[1], m[2], title.textContent)];
  }

  /** A listing (title, chart, label, artist, genre, search): one row per track link. */
  function readListing(doc) {
    const out = new Map();
    for (const a of doc.querySelectorAll('a[href*="/track/"]')) {
      const m = trackOf(a);
      if (!m || out.has(m[1]) || !a.textContent.trim()) continue;
      out.set(m[1], fields(row(a, m[1], doc.body), m[1], m[2], a.textContent));
      if (out.size === MAX_TRACKS) break;
    }
    return [...out.values()];
  }

  /** The page's tracks as the player takes them, or null when none can be read. */
  function read(doc, address) {
    let list;
    try {
      list = /^\/track\//.test(new URL(address).pathname) ? readTrack(doc, address) : readListing(doc);
    } catch {
      return null;
    }
    list = list.filter((t) => t.title && t.artists.length > 0);
    if (list.length === 0) return null;
    return { page: address, tracks: list };
  }

  return { read, seconds };
})();
