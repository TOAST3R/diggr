// Shared by the service worker, the content script, the options page and the popup: which
// Discogs, Bandcamp and Juno pages the player can dig, and the messages shown when something is
// missing.
//
// The check here is coarse, only to decide where the button appears: the player parses the
// address itself and refuses anything else.

/* exported WR */
var WR = (() => {
  const PLACEHOLDER = "Diggr"; // until the player has answered once
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

  // Bandcamp: ‹name›.bandcamp.com/ or /music (a label or artist), /album/…, /track/….
  const BANDCAMP_HOST = /^[a-z0-9][a-z0-9-]{0,62}\.bandcamp\.com$/;
  const BANDCAMP_KINDS = [
    [/^\/(?:music\/?)?$/, "BandcampLabel"],
    [/^\/album\/[a-z0-9-]+\/?$/, "BandcampAlbum"],
    [/^\/track\/[a-z0-9-]+\/?$/, "BandcampTrack"],
  ];

  // A Bandcamp fan's collection or wishlist: bandcamp.com/‹user› and /‹user›/wishlist, except
  // Bandcamp's own pages (the player checks the name strictly).
  const FAN_HOST = /^(?:www\.)?bandcamp\.com$/;
  const FAN_PATH = /^\/([a-z0-9_-]{1,64})(?:\/wishlist)?\/?$/;
  const NOT_FANS = new Set([
    "about", "album", "api", "artists", "contact", "cookies", "discover", "download", "fans",
    "feed", "gift_cards", "guide", "help", "login", "logout", "privacy", "search", "settings",
    "signup", "tag", "terms_of_use", "track", "yum", "pricing",
  ]);

  // Mixes: a YouTube video, or a SoundCloud track (‹user›/‹name›, not one of SoundCloud's own
  // pages nor a user's lists). The app reads its tracklist itself; only the address is sent.
  const YOUTUBE_HOST = /^(?:www\.|m\.|music\.)?youtube\.com$/;
  const SOUNDCLOUD_HOST = /^(?:www\.|m\.)?soundcloud\.com$/;
  const SOUNDCLOUD_TRACK = /^\/([a-z0-9_-]{1,100})\/([a-z0-9_-]{1,100})\/?$/;
  const NOT_SOUNDCLOUD_USERS = new Set([
    "discover", "stream", "you", "search", "upload", "charts", "settings", "messages",
    "notifications", "pages", "jobs", "terms-of-use", "people", "tags", "mobile", "imprint",
    "pro", "artists", "feed", "signin", "logout",
  ]);
  const NOT_SOUNDCLOUD_TRACKS = new Set([
    "sets", "tracks", "albums", "likes", "reposts", "followers", "following", "popular-tracks",
    "comments", "spotlight", "playlists", "toptracks",
  ]);

  // Juno: a product page, or a page listing records (label, artist, genre, chart, search, new
  // releases). Its first path part tells; equipment, the cart and the account have no records.
  // www and the regional sites (es., de.…), which share product ids.
  const JUNO_HOST = /^(?:www|[a-z]{2})\.juno\.co\.uk$/;
  const JUNO_PRODUCT = /^\/products\/[a-z0-9-]+\/\d{1,9}-\d{2}\/?$/;
  const JUNO_LISTING = /^\/(?:[a-z0-9-]+\/)*$/;
  const NOT_JUNO_LISTINGS = new Set([
    "", "products", "dj-equipment", "equipment", "cart", "basket", "checkout", "account",
    "login", "register", "help", "about", "contact", "merchandise", "gift-vouchers", "wishlist",
  ]);

  // Beatport and Traxsource: any page listing tracks (track, release or title, chart, label,
  // artist, genre, search…). The cart, the account and the home page have none to read.
  const STORE_HOSTS = { "www.beatport.com": "Beatport", "www.traxsource.com": "Traxsource" };
  const NOT_STORE_PAGES = new Set([
    "", "cart", "checkout", "account", "my-account", "library", "my-beatport", "login",
    "signup", "register", "help", "about", "contact", "download", "downloads", "subscriptions",
  ]);

  // Bleep: a release page, or any page listing records (label, artist, search, new
  // releases…). The basket, checkout and account have none to read.
  const BLEEP_HOST = /^(?:www\.)?bleep\.com$/;
  const BLEEP_RELEASE = /^\/release\/\d{1,10}-[a-z0-9-]+\/?$/;
  const NOT_BLEEP_PAGES = new Set([
    "basket", "cart", "checkout", "account", "my-account", "login", "sign-in", "register",
    "gift-vouchers", "help", "about", "contact", "terms", "privacy", "player", "rest",
  ]);

  /**
   * "Label", "Release"… for a supported Discogs page; "BandcampLabel", "BandcampAlbum",
   * "BandcampTrack" or "BandcampFan" for a Bandcamp one; "JunoProduct" or "JunoListing" for a
   * Juno one; "BeatportPage" or "TraxsourcePage"; "BleepRelease" or "BleepPage"; else null.
   */
  function pageKind(address) {
    let u;
    try {
      u = new URL(address);
    } catch {
      return null;
    }
    if (u.protocol === "https:" && JUNO_HOST.test(u.hostname)) {
      if (JUNO_PRODUCT.test(u.pathname)) return "JunoProduct";
      const first = u.pathname.split("/")[1];
      return JUNO_LISTING.test(u.pathname) && !NOT_JUNO_LISTINGS.has(first) ? "JunoListing" : null;
    }
    if (u.protocol === "https:" && STORE_HOSTS[u.hostname]) {
      const first = u.pathname.split("/")[1].toLowerCase();
      return NOT_STORE_PAGES.has(first) ? null : `${STORE_HOSTS[u.hostname]}Page`;
    }
    if (u.protocol === "https:" && BLEEP_HOST.test(u.hostname)) {
      if (BLEEP_RELEASE.test(u.pathname)) return "BleepRelease";
      return NOT_BLEEP_PAGES.has(u.pathname.split("/")[1].toLowerCase()) ? null : "BleepPage";
    }
    if (u.protocol === "https:" && YOUTUBE_HOST.test(u.hostname)) {
      return u.pathname === "/watch" && /^[A-Za-z0-9_-]{11}$/.test(u.searchParams.get("v") || "")
        ? "YouTubeMix"
        : null;
    }
    if (u.protocol === "https:" && SOUNDCLOUD_HOST.test(u.hostname)) {
      const m = u.pathname.match(SOUNDCLOUD_TRACK);
      return m && !NOT_SOUNDCLOUD_USERS.has(m[1]) && !NOT_SOUNDCLOUD_TRACKS.has(m[2]) ? "SoundCloudMix" : null;
    }
    if (u.protocol === "https:" && FAN_HOST.test(u.hostname)) {
      const m = u.pathname.match(FAN_PATH);
      return m && !NOT_FANS.has(m[1]) ? "BandcampFan" : null;
    }
    if (u.protocol === "https:" && BANDCAMP_HOST.test(u.hostname)) {
      for (const [re, kind] of BANDCAMP_KINDS) if (re.test(u.pathname)) return kind;
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

  // Context-menu link patterns (match patterns can't say "digits"; the player checks). A label
  // link has its own item (Send label), so it is kept apart.
  const LINK_PATTERNS = [];
  const LABEL_LINK_PATTERNS = [];
  for (const host of ["*://discogs.com", "*://www.discogs.com"]) {
    LABEL_LINK_PATTERNS.push(`${host}/label/*`, `${host}/*/label/*`);
    for (const kind of ["release", "master", "artist", "lists"]) {
      LINK_PATTERNS.push(`${host}/${kind}/*`, `${host}/*/${kind}/*`);
    }
    for (const shop of ["shop/item", "sell/item"]) {
      LINK_PATTERNS.push(`${host}/${shop}/*`, `${host}/*/${shop}/*`);
    }
    LINK_PATTERNS.push(`${host}/wantlist*`, `${host}/*/wantlist*`);
    LINK_PATTERNS.push(`${host}/user/*/collection*`, `${host}/*/user/*/collection*`);
  }
  LINK_PATTERNS.push("https://*.bandcamp.com/album/*", "https://*.bandcamp.com/track/*");
  LABEL_LINK_PATTERNS.push("https://*.bandcamp.com/", "https://*.bandcamp.com/music");

  const SUPPORTED =
    "a release, master release, artist, label, wantlist, collection, list, marketplace item " +
    "or seller on Discogs; a label, album or track on Bandcamp, or a fan's collection or " +
    "wishlist (bandcamp.com/‹user›); a product, label, artist, genre, chart or search page " +
    "on Juno; a track, release, chart, label, artist, genre or search page on Beatport or " +
    "Traxsource, a release or list of records on Bleep (from this button only), or a mix on " +
    "YouTube or SoundCloud";

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
    // A mix: "Mix: ‹title›", without the site's own words.
    if (kind === "YouTubeMix" || kind === "SoundCloudMix") {
      t = t.replace(/^\(\d+\)\s*/, "").replace(/\s*-\s*YouTube\s*$/i, "");
      t = t.replace(/^Stream\s+/i, "").split(/\s+\|\s+Listen online/i)[0];
      // SoundCloud: "Stream ‹title› by ‹uploader›".
      if (kind === "SoundCloudMix" && t.lastIndexOf(" by ") > 0) t = t.slice(0, t.lastIndexOf(" by "));
      return fit(`Mix: ${t.replace(/\s+/g, " ").trim()}`);
    }
    // Bandcamp: "Album | Artist", "Track | Artist" (a label's own page: "Music | Label").
    if (kind === "BandcampAlbum" || kind === "BandcampTrack") {
      const [what, who] = t.split(" | ");
      return fit(who ? `${who.trim()} - ${what.trim()}` : what.trim());
    }
    if (kind === "BandcampLabel") return fit(t.split(" | ").pop().trim());
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

  /** A Bandcamp page's kind (see pageKind). */
  function isBandcamp(kind) {
    return typeof kind === "string" && kind.startsWith("Bandcamp");
  }

  /** A Juno page's kind (see pageKind). */
  function isJuno(kind) {
    return typeof kind === "string" && kind.startsWith("Juno");
  }

  /** A YouTube or SoundCloud mix's page (see pageKind). */
  function isMix(kind) {
    return kind === "YouTubeMix" || kind === "SoundCloudMix";
  }

  /** The store whose page this is ("Juno", "Beatport", "Traxsource", "Bleep"), else null. */
  function storeOf(kind) {
    if (isJuno(kind)) return "Juno";
    if (kind === "BleepRelease" || kind === "BleepPage") return "Bleep";
    if (kind === "BeatportPage") return "Beatport";
    if (kind === "TraxsourcePage") return "Traxsource";
    return null;
  }

  return {
    PLACEHOLDER,
    isBandcamp,
    isJuno,
    isMix,
    storeOf,
    DEFAULT_PORT,
    pageKind,
    LINK_PATTERNS,
    LABEL_LINK_PATTERNS,
    SUPPORTED,
    MAX_NAME,
    suggestName,
    fit,
    problem,
  };
})();
