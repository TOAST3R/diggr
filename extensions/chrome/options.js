// Pairing (code → key, kept by the service worker) and the bridge's port.

const $ = (id) => document.getElementById(id);
const ask = (msg) => chrome.runtime.sendMessage(msg);

async function refresh() {
  const s = await chrome.storage.local.get({
    port: WR.DEFAULT_PORT,
    key: null,
    appName: WR.PLACEHOLDER,
  });
  $("port").value = s.port;
  const r = await ask({ type: "status" });
  const app = (await chrome.storage.local.get({ appName: WR.PLACEHOLDER })).appName;
  document.title = `Pair with ${app}`;
  $("title").textContent = `Pair with ${app}`;
  $("how").textContent = `In ${app}, open OPT ▸ Browser… and enter the 6-digit code it shows.`;
  const state = $("state");
  state.classList.toggle("bad", !r.ok);
  state.textContent = r.ok ? `Paired with ${app} on port ${s.port}.` : WR.problem(r.error, app);
}

$("pair").addEventListener("submit", async (e) => {
  e.preventDefault();
  const code = $("code").value.replace(/\D/g, "");
  const port = Number($("port").value) || WR.DEFAULT_PORT;
  const result = $("result");
  if (code.length !== 6) {
    result.className = "bad";
    result.textContent = "The code has 6 digits.";
    return;
  }
  const r = await ask({ type: "pair", code, port });
  const { appName } = await chrome.storage.local.get({ appName: WR.PLACEHOLDER });
  result.className = r.ok ? "" : "bad";
  result.textContent = r.ok ? `Paired: Discogs pages now have a ${appName} button.` : WR.problem(r.error, appName);
  $("code").value = "";
  refresh();
});

$("savePort").addEventListener("click", async () => {
  const port = Number($("port").value);
  if (!(port >= 1024 && port <= 65535)) return;
  await chrome.storage.local.set({ port });
  refresh();
});

$("unpair").addEventListener("click", async () => {
  await ask({ type: "unpair" });
  refresh();
});

refresh();
