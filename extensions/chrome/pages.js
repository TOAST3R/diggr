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
    LINK_PATTERNS.push(`${host}/wantlist*`, `${host}/*/wantlist*`);
  }

  const SUPPORTED = "a release, master release, artist, label, wantlist or list";

  /** What to tell the user for a failed bridge call. */
  function problem(error, app) {
    switch (error) {
      case "not-running":
        return `${app} isn't running. Start it, then try again.`;
      case "not-paired":
        return `Not paired with ${app}: enter the code from OPT ▸ Browser… in the options.`;
      case "unsupported":
        return `${app} can't dig this page. Supported: ${SUPPORTED}.`;
      case "locked":
        return "Too many wrong codes: wait a minute.";
      case "wrong-code":
        return "Wrong code: check OPT ▸ Browser… in the player.";
      case "no-code":
        return "No code is shown: open OPT ▸ Browser… in the player.";
      default:
        return `${app} refused the request (${error}).`;
    }
  }

  return { PLACEHOLDER, DEFAULT_PORT, pageKind, LINK_PATTERNS, SUPPORTED, problem };
})();
