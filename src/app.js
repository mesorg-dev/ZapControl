const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (id) => document.getElementById(id);

let strategies = [];
let settings = null;
let running = false;
let testing = false;

function log(msg) {
  const el = $("log");
  const ts = new Date().toLocaleTimeString();
  el.textContent += `[${ts}] ${msg}\n`;
  el.scrollTop = el.scrollHeight;
}

function setStatus(on, label) {
  running = on;
  $("status-dot").classList.toggle("on", on);
  $("status-dot").classList.toggle("off", !on);
  $("status-text").textContent = label || (on ? "работает" : "стоп");
  const btn = $("btn-toggle");
  btn.textContent = on ? "Остановить" : "Запустить";
  btn.classList.toggle("running", on);
}

async function refreshStatus() {
  try {
    const st = await invoke("get_status");
    setStatus(st.running, st.running ? "работает" : "стоп");
    $("service-badge").classList.toggle("hidden", !st.service_installed);
    if (!st.winws_path_ok) {
      log("winws.exe не найден — положи папку bin/ рядом с zapret.exe");
    }
  } catch (e) {
    log("status error: " + e);
  }
}

async function loadStrategies() {
  strategies = await invoke("get_strategies");
  const sel = $("strategy-select");
  sel.innerHTML = "";
  for (const s of strategies) {
    const opt = document.createElement("option");
    opt.value = s.id;
    opt.textContent = s.name;
    sel.appendChild(opt);
  }
}

async function loadSettings() {
  settings = await invoke("get_settings");
  $("strategy-select").value = settings.lastStrategy || "general";
  $("sw-autostart").checked = settings.autostartMode === "tray" || settings.autostartMode === "service";
  $("sw-roblox").checked = !!settings.robloxFixEnabled;
  $("inp-roblox-url").value = settings.robloxJsonUrl || "";
  $("sel-autostart-mode").value = settings.autostartMode || "none";
}

async function saveSettingsFromUi() {
  settings.autostartMode = $("sel-autostart-mode").value;
  settings.robloxJsonUrl = $("inp-roblox-url").value.trim();
  settings.robloxFixEnabled = $("sw-roblox").checked;
  settings.lastStrategy = $("strategy-select").value;
  try {
    settings = await invoke("save_settings", { settings });
    log("настройки сохранены");
  } catch (e) {
    log("ошибка сохранения: " + e);
    throw e;
  }
}

/* ---------- events from backend ---------- */

listen("test-progress", (e) => {
  const d = e.payload;
  const pct = Math.round((d.index / d.total) * 100);
  $("progress-bar").style.width = pct + "%";
  $("progress-label").textContent = d.strategyName;
  $("progress-count").textContent = `${d.index} / ${d.total}`;
  log(`тест [${d.index}/${d.total}] ${d.strategyName}`);
});

listen("test-strategy-done", (e) => {
  const d = e.payload;
  log(`${d.strategyName}: OK=${d.okCount} ERR=${d.errCount}`);
  renderStrategyResult(d);
});

listen("test-target", () => {});

listen("test-finished", (e) => {
  const d = e.payload;
  finishTestUi(d);
});

listen("status-changed", (e) => {
  setStatus(e.payload.running);
  if (e.payload.strategy) {
    $("strategy-select").value = e.payload.strategy;
  }
});

listen("log", (e) => log(String(e)));

/* ---------- results rendering ---------- */

function renderStrategyResult(d) {
  const body = $("results-body");
  if (body.querySelector(".empty")) body.innerHTML = "";
  const div = document.createElement("div");
  div.className = "report";
  div.id = "rep-" + d.strategyId;
  div.innerHTML = `
    <div class="report-head">
      <span>${d.strategyName}</span>
      <span class="score ${d.okCount > 0 ? "ok" : "bad"}">OK ${d.okCount} · ERR ${d.errCount}</span>
    </div>`;
  body.prepend(div);
}

function finishTestUi(summary) {
  testing = false;
  $("btn-cancel").classList.add("hidden");
  $("btn-auto").disabled = false;
  $("progress-wrap").classList.add("hidden");

  if (summary.bestStrategy) {
    const name = strategies.find((s) => s.id === summary.bestStrategy)?.name || summary.bestStrategy;
    const bl = $("best-label");
    bl.textContent = "лучшая: " + name;
    bl.classList.remove("hidden");
    $("strategy-select").value = summary.bestStrategy;
    settings.lastStrategy = summary.bestStrategy;
    log("лучшая стратегия: " + name);
  }
}

/* ---------- actions ---------- */

$("btn-toggle").addEventListener("click", async () => {
  const id = $("strategy-select").value;
  try {
    if (running) {
      await invoke("stop_bypass");
      setStatus(false);
      log("остановлено");
    } else {
      $("btn-toggle").disabled = true;
      log("запуск " + id + "…");
      await invoke("start_bypass", { strategyId: id });
      setStatus(true);
      log("запущено: " + id);
    }
  } catch (e) {
    log("ошибка: " + e);
    alert(e);
  } finally {
    $("btn-toggle").disabled = false;
  }
});

$("btn-auto").addEventListener("click", async () => {
  if (testing) return;
  testing = true;
  $("progress-wrap").classList.remove("hidden");
  $("btn-cancel").classList.remove("hidden");
  $("btn-auto").disabled = true;
  $("results-body").innerHTML = '<p class="muted empty">идёт тест…</p>';
  $("best-label").classList.add("hidden");
  try {
    const summary = await invoke("run_auto_test", { strategyIds: null });
    finishTestUi(summary);
  } catch (e) {
    log("тест: " + e);
    alert(e);
    finishTestUi({ bestStrategy: null, reports: [] });
  }
});

$("btn-cancel").addEventListener("click", async () => {
  await invoke("cancel_auto_test");
  log("отмена запрошена…");
});

$("sw-autostart").addEventListener("change", async (ev) => {
  if (!settings) return;
  const mode = ev.target.checked
    ? ($("sel-autostart-mode").value === "service" ? "service" : "tray")
    : "none";
  $("sel-autostart-mode").value = mode;
  settings.autostartMode = mode;
  try {
    await invoke("save_settings", { settings });
    log("автозапуск: " + mode);
  } catch (e) {
    log("автозапуск error: " + e);
    ev.target.checked = false;
  }
});

$("btn-service-install").addEventListener("click", async () => {
  const id = $("strategy-select").value;
  try {
    log("установка службы (UAC)…");
    await invoke("install_service", { strategyId: id });
    log("служба установлена");
    await refreshStatus();
  } catch (e) {
    log("служба: " + e);
    alert(e);
  }
});

$("btn-service-remove").addEventListener("click", async () => {
  try {
    log("удаление службы (UAC)…");
    await invoke("remove_service");
    log("служба удалена");
    await refreshStatus();
  } catch (e) {
    log("служба: " + e);
    alert(e);
  }
});

$("btn-settings").addEventListener("click", () => {
  $("settings-modal").classList.remove("hidden");
  refreshRobloxStatus();
});

$("btn-settings-close").addEventListener("click", () => {
  $("settings-modal").classList.add("hidden");
});

$("btn-settings-save").addEventListener("click", async () => {
  try {
    await saveSettingsFromUi();
    $("settings-modal").classList.add("hidden");
  } catch (e) {}
});

$("btn-roblox-update").addEventListener("click", async () => {
  try {
    await saveSettingsFromUi();
    log("roblox: скачиваю и применяю…");
    await invoke("roblox_apply");
    log("roblox: применено");
    settings.robloxFixEnabled = true;
    $("sw-roblox").checked = true;
    await refreshRobloxStatus();
  } catch (e) {
    log("roblox: " + e);
    alert(e);
  }
});

$("btn-roblox-remove").addEventListener("click", async () => {
  try {
    await invoke("roblox_remove");
    log("roblox: снято");
    settings.robloxFixEnabled = false;
    $("sw-roblox").checked = false;
    await refreshRobloxStatus();
  } catch (e) {
    log("roblox: " + e);
    alert(e);
  }
});

$("sw-roblox").addEventListener("change", async (ev) => {
  if (!settings) return;
  settings.robloxFixEnabled = ev.target.checked;
  try {
    await invoke("save_settings", { settings });
  } catch (e) {
    log(e);
  }
});

async function refreshRobloxStatus() {
  try {
    const st = await invoke("roblox_status");
    const line = $("roblox-status-line");
    if (st.applied) {
      line.textContent = `применено · записей: ${st.entryCount}` + (st.updated ? ` · обновлено: ${st.updated}` : "");
    } else {
      line.textContent = "не применено";
    }
  } catch (e) {
    log("roblox status: " + e);
  }
}

$("btn-log-toggle").addEventListener("click", () => {
  document.querySelector(".log-strip").classList.toggle("collapsed");
});

/* ---------- init ---------- */

(async function init() {
  log("zapret-gui starting…");
  await loadStrategies();
  await loadSettings();
  await refreshStatus();
  setInterval(refreshStatus, 5000);
  log("готово");
})();
