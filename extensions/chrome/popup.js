// The toolbar popup: running and paired, what's playing, sends in progress, and what happened
// to the last link sent from another site.

const $ = (id) => document.getElementById(id);
const el = (tag, text, cls) => Object.assign(document.createElement(tag), { textContent: text, className: cls || "" });

$("options").addEventListener("click", (e) => {
  e.preventDefault();
  chrome.runtime.openOptionsPage();
});

(async () => {
  const r = await chrome.runtime.sendMessage({ type: "status" });
  const { appName } = await chrome.storage.local.get({ appName: WR.PLACEHOLDER });
  $("app").textContent = appName;
  const state = $("state");
  if (!r.ok) {
    state.classList.add("bad");
    state.textContent = WR.problem(r.error, appName);
  } else {
    state.textContent = "Running · paired";
    const p = r.json.playing;
    $("playing").append(el("h2", "Now playing"));
    $("playing").append(
      p ? el("div", `${p.artist ? p.artist + " – " : ""}${p.title} (${p.crate})`) : el("div", "Nothing", "dim"),
    );
    if (r.json.sends.length) {
      $("sends").append(el("h2", "Sends in progress"));
      const ul = document.createElement("ul");
      for (const s of r.json.sends) {
        ul.append(el("li", s.total ? `${s.page}: ${s.done} of ${s.total} releases` : `${s.page}: starting`));
      }
      $("sends").append(ul);
    }
  }
  const { lastLink } = await chrome.storage.session.get("lastLink");
  if (lastLink) {
    $("last").append(el("h2", "Last link"));
    $("last").append(el("div", lastLink.text, lastLink.ok ? "" : "bad"));
  }
})();
