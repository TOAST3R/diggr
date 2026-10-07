// Shared by the service worker, the content script, the options page and the popup: which
// Discogs pages the player can dig, and the messages shown when something is missing.
//
// The check here is coarse, only to decide where the button appears: the player parses the
// address itself and refuses anything else.

/* exported WR */
var WR = (() => {
  const PLACEHOLDER = "winamp_rust"; // until the player has answered once
  const DEFAULT_PORT = 47800;

  const LANG = "(?:[a-z]{2}/)?";
  const KINDS = [
    [new RegExp(`^/${LANG}release/\\d+`), "Release"],
    [/^\/[^/]+\/release\/\d+/, "Release"], // legacy: /Artist-Title/release/123
    [new RegExp(`^/${LANG}master/\\d+`), "Master"],
    [/^\/[^/]+\/master\/\d+/, "Master"],
    [new RegExp(`^/${LANG}artist/\\d+`), "Artist"],
    [new RegExp(`^/${LANG}label/\\d+`), "Label"],
    [new RegExp(`^/${LANG}lists/(?:[^/]+/)?\\d+`), "List"],
    [new RegExp(`^/${LANG}user/[^/]+/wantlist`), "Wantlist"],
    [new RegExp(`^/${LANG}user/[^/]+/collection`), "Collection"],
    [new RegExp(`^/${LANG}(?:shop|sell)/item/\\d+`), "Listing"],
    // A seller's shop, or a user's page alone: added to Top Sellers, not sent as tracks.
    [new RegExp(`^/${LANG}seller/[^/]+(?:/profile)?/?$`), "Seller"],
    [new RegExp(`^/${LANG}user/[^/]+/?$`), "Seller"],
  ];

  /** "Label", "Release"… for a supported Discogs page, else null. */
  function pageKind(address) {
    let u;
    try {
      u = new URL(address);
    } catch {
      return null;
    }
    if (u.hostname !== "www.discogs.com" && u.hostname !== "discogs.com") return null;
    const path = u.pathname;
    for (const [re, kind] of KINDS) if (re.test(path)) return kind;
    if (new RegExp(`^/${LANG}wantlist/?$`).test(path) && u.searchParams.get("user")) {
      return "Wantlist";
    }
    return null;
  }

  // Context-menu link patterns (match patterns can't say "digits"; the player checks).
  const LINK_PATTERNS = [];
  for (const host of ["*://discogs.com", "*://www.discogs.com"]) {
    for (const kind of ["release", "master", "artist", "label", "lists"]) {
      LINK_PATTERNS.push(`${host}/${kind}/*`, `${host}/*/${kind}/*`);
    }
    for (const shop of ["shop/item", "sell/item"]) {
      LINK_PATTERNS.push(`${host}/${shop}/*`, `${host}/*/${shop}/*`);
    }
    LINK_PATTERNS.push(`${host}/wantlist*`, `${host}/*/wantlist*`);
    LINK_PATTERNS.push(`${host}/user/*/collection*`, `${host}/*/user/*/collection*`);
  }

  const SUPPORTED =
    "a release, master release, artist, label, wantlist, collection, list, marketplace item or seller";

  const MAX_NAME = 40; // the player's limit for a crate name

  /**
   * A crate name for the page, from its title: "Artist - Title" for a release, master or
   * marketplace item, the name for an artist, label or list, "Wantlist: user" for a wantlist.
   * Discogs' name-variant asterisks and "(2)" numbers go, its long dash becomes "-", and the
   * result fits the 40-character limit. Empty when nothing sensible is left.
   */
  function suggestName(title, kind, address) {
    if (kind === "Collection") {
      let m = null;
      try {
        m = new URL(address).pathname.match(/\/user\/([^/]+)\/collection/);
      } catch {
        // no address: no user
      }
      return m ? fit(`Collection: ${decodeURIComponent(m[1])}`) : "";
    }
    if (kind === "Wantlist") {
      let user = null;
      try {
        const u = new URL(address);
        const m = u.pathname.match(/\/user\/([^/]+)\/wantlist/);
        user = m ? decodeURIComponent(m[1]) : u.searchParams.get("user");
      } catch {
        // no address: no user
      }
      return user ? fit(`Wantlist: ${user}`) : "";
    }
    let t = String(title || "");
    t = t.split(" | ")[0]; // "… | Releases | Discogs"
    t = t.replace(/\s+[-–—]\s*Discogs\s*$/i, ""); // older "… - Discogs"
    t = t.replace(/\s+for sale\b.*$/i, ""); // marketplace items
    t = t.replace(/\s*\(\d{4}[^)]*\)\s*$/, ""); // older "(2015, Vinyl)"
    // Release pages add the format: "Artist – Title – Vinyl, 12", EP". Artist and title are
    // the first two parts (Discogs separates them with a long dash; titles use "-").
    if (kind === "Release" || kind === "Master" || kind === "Listing") {
      const parts = t.split(/\s+[–—]\s+/);
      if (parts.length > 2) t = parts.slice(0, 2).join(" – ");
    }
    // …and a format after a plain "-" too ("Artist - Title - Vinyl").
    t = t.replace(
      /\s+[-–—]\s+(?:Vinyl|LP|CD|CDr|Cassette|Files?|FLAC|MP3|WAV|ALAC|DVD|Box Set|Shellac|Flexi-disc|Lathe Cut|\d+")\b.*$/i,
      "",
    );
    t = t.replace(/\*/g, "").replace(/\s*\(\d+\)/g, "");
    t = t.replace(/\s*[–—]\s*/g, " - ").replace(/\s+/g, " ").trim();
    return fit(t);
  }

  /** Cut to the crate-name limit, at a word when one is near. */
  function fit(name) {
    const chars = [...name];
    if (chars.length <= MAX_NAME) return name;
    let cut = chars.slice(0, MAX_NAME).join("");
    const space = cut.lastIndexOf(" ");
    if (space > MAX_NAME / 2) cut = cut.slice(0, space);
    return cut.replace(/[\s-]+$/, "");
  }

  /** What to tell the user for a failed bridge call. */
  function problem(error, app) {
    switch (error) {
      case "not-running":
        return `${app} isn't running. Start it, then try again.`;
      case "not-paired":
        return `Not paired with ${app}: enter the code from Options ▸ Browser… in the options.`;
      case "unsupported":
        return `${app} can't dig this page. Supported: ${SUPPORTED}.`;
      case "locked":
        return "Too many wrong codes: wait a minute.";
      case "wrong-code":
        return "Wrong code: check Options ▸ Browser… in the player.";
      case "no-code":
        return "No code is shown: open Options ▸ Browser… in the player.";
      default:
        return `${app} refused the request (${error}).`;
    }
  }

  return {
    PLACEHOLDER,
    DEFAULT_PORT,
    pageKind,
    LINK_PATTERNS,
    SUPPORTED,
    MAX_NAME,
    suggestName,
    problem,
  };
})();
