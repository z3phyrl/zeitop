
let z = zeitop;

/* ===== STORAGE KEY ===== */
const STORAGE_KEY = "zeitop-macro-state-v10";

/* ===== STATIC ICONS (BASE64 GOES HERE) ===== */
const ICONS = {
  startStream: "", // "data:image/png;base64,..."
  stopStream: "",
  startRec: "",
  stopRec: "",
  scene1: "",
  scene2: "",
  scene3: "",
  scene4: "",
  muteMic: "",
  muteDesk: "",
  screenshot: "",
  discord: ""
};

/* ===== DEFAULT OBS KEYS ===== */
const defaultKeys = [
  { label: "Start Rec", combo: "OBS Start",   mode: "SYSTEM", iconDataUrl: ICONS.startStream },
  { label: "Stop Rec",  combo: "OBS Stop",    mode: "SYSTEM", iconDataUrl: ICONS.stopStream },
  { label: "Mute Mic",    combo: "OBS Record",  mode: "SYSTEM", iconDataUrl: ICONS.startRec },
  { label: "Unmute Mic",     combo: "OBS StopRec", mode: "SYSTEM", iconDataUrl: ICONS.stopRec },
  { label: "Scene 1",      combo: "Scene #1",    mode: "MEDIA",  iconDataUrl: ICONS.scene1 },
  { label: "Scene 2",      combo: "Scene #2",    mode: "MEDIA",  iconDataUrl: ICONS.scene2 },
  { label: "Scene 3",      combo: "Scene #3",    mode: "MEDIA",  iconDataUrl: ICONS.scene3 },
  { label: "Scene 4",      combo: "Scene #4",    mode: "MEDIA",  iconDataUrl: ICONS.scene4 },
  { label: "Mute Mic",     combo: "Toggle Mic",  mode: "AUDIO",  iconDataUrl: ICONS.muteMic },
  { label: "Mute Desk",    combo: "Toggle Aud",  mode: "AUDIO",  iconDataUrl: ICONS.muteDesk },
  { label: "Screenshot",   combo: "Screenshot",  mode: "SYSTEM", iconDataUrl: ICONS.screenshot },
  { label: "Discord",      combo: "Open Discord",mode: "SYSTEM", iconDataUrl: ICONS.discord }
];

/* ===== CALLBACKS FOR EACH BUTTON ===== */
const buttonActions = [
  () => {z.request("obs", "record_start", () => {}, "obs_ctl1")},
  () => {z.request("obs", "record_stop", () => {}, "obs_ctl2")},
  () => {z.request("obs", "mic_mute", () => {}, "obs_ctl3")},
  () => {z.request("obs", "mic_unmute", () => {}, "obs_ctl4")},
  () => console.log("Scene 1"),
  () => console.log("Scene 2"),
  () => console.log("Scene 3"),
  () => console.log("Scene 4"),
  () => console.log("Toggle Mic Mute"),
  () => console.log("Toggle Desktop Mute"),
  () => console.log("Screenshot"),
  () => console.log("Open Discord")
];

/* ===== STATE LOAD / SAVE ===== */
function loadState() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}
function saveState() {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(keyState));
}
let keyState = loadState() || JSON.parse(JSON.stringify(defaultKeys));

/* ===== SYSTEM ELEMENTS ===== */
const cpuFill = document.getElementById("cpu-fill");
const cpuValue = document.getElementById("cpu-value");
const ramFill = document.getElementById("ram-fill");
const ramValue = document.getElementById("ram-value");

/* ===== MUSIC ELEMENTS ===== */
const trackTitle = document.getElementById("track-title");
const trackArtist = document.getElementById("track-artist");
const trackProgressFill = document.getElementById("track-progress-fill");
const trackCurrent = document.getElementById("track-current");
const trackDuration = document.getElementById("track-duration");
const btnPrev = document.getElementById("btn-prev");
const btnPlay = document.getElementById("btn-play");
const btnNext = document.getElementById("btn-next");

/* ===== HELPERS ===== */
function setMeter(fillEl, valueEl, v) {
  const val = Math.max(0, Math.min(100, Math.round(v)));
  fillEl.style.width = val + "%";
  valueEl.textContent = val + "%";
}
function formatTime(sec) {
  sec = Math.max(0, Math.floor(sec));
  const m = Math.floor(sec / 60);
  const s = ("0" + (sec % 60)).slice(-2);
  return m + ":" + s;
}

/* PUBLIC HOOKS */
function updateSystemStatus({ cpu, ram }) {
  if (cpu != null) setMeter(cpuFill, cpuValue, cpu);
  if (ram != null) setMeter(ramFill, ramValue, ram);
}
function updateNowPlaying({ title, artist, position, duration, isPlaying }) {
  if (title != null) trackTitle.textContent = title || "Unknown track";
  if (artist != null) trackArtist.textContent = artist || "Unknown artist";
  const pos = position || 0;
  const dur = duration || 0;
  trackCurrent.textContent = formatTime(pos);
  trackDuration.textContent = formatTime(dur);
  trackProgressFill.style.width = dur > 0 ? (pos / dur * 100) + "%" : "0%";
  btnPlay.textContent = isPlaying ? "⏸" : "▶";
}

/* PLAYER BUTTONS (wire to API later) */
btnPlay.addEventListener("click", () => console.log("play/pause"));
btnPrev.addEventListener("click", () => console.log("prev"));
btnNext.addEventListener("click", () => console.log("next"));

/* MACRO CLICK ANIM */
function pressKey(el) {
  el.classList.add("pressed");
  setTimeout(() => el.classList.remove("pressed"), 130);
}

/* RENDER KEYS */
function renderKeys() {
  const grid = document.getElementById("key-grid");
  grid.innerHTML = "";

  keyState.forEach((k, i) => {
    const el = document.createElement("button");
    el.className = "key";

    const icon = document.createElement("div");
    icon.className = "key-icon";
    if (k.iconDataUrl) {
      icon.style.backgroundImage = `url(${k.iconDataUrl})`;
    } else {
      icon.style.backgroundImage = "none";
    }

    const label = document.createElement("div");
    label.className = "key-text";
    label.textContent = k.label;

    el.appendChild(icon);
    el.appendChild(label);

    el.addEventListener("click", () => {
      pressKey(el);
      if (buttonActions[i]) buttonActions[i]();
    });

    grid.appendChild(el);
  });
}

/* SLIDERS – pointer-based, smooth, no scroll */
function initSlider(cfg) {
  const wrap = document.getElementById(cfg.id);
  const thumb = wrap.querySelector(".slider-thumb");
  const valueEl = document.getElementById(cfg.valueId);
  const PADDING = 10;

  let value = cfg.initial;
  let dragging = false;
  let metrics = null;

  function computeMetrics() {
    const rect = wrap.getBoundingClientRect();
    const h = rect.height;
    const th = thumb.offsetHeight || 10;
    const minY = PADDING;
    const maxY = h - PADDING - th;
    return { rectTop: rect.top, th, minY, maxY };
  }

  function setValue(v) {
    v = Math.max(0, Math.min(100, Math.round(v)));
    value = v;
    valueEl.textContent = v + "%";

    if (!metrics) metrics = computeMetrics();
    const { minY, maxY } = metrics;
    const y = maxY - (v / 100) * (maxY - minY);
    thumb.style.top = y + "px";
  }

  function valueFromClientY(clientY) {
    if (!metrics) metrics = computeMetrics();
    const { rectTop, th, minY, maxY } = metrics;
    let y = clientY - rectTop - th / 2;
    y = Math.max(minY, Math.min(maxY, y));
    const raw = ((maxY - y) / (maxY - minY)) * 100;
    return raw;
  }

  function startDrag(e) {
    if (e.button !== undefined && e.button !== 0) return;
    dragging = true;
    metrics = computeMetrics();
    wrap.setPointerCapture(e.pointerId);
    const v = valueFromClientY(e.clientY);
    setValue(v);
  }

  function moveDrag(e) {
    if (!dragging) return;
    const v = valueFromClientY(e.clientY);
    setValue(v);
  }

  function endDrag(e) {
    if (!dragging) return;
    dragging = false;
    metrics = null;
    try { wrap.releasePointerCapture(e.pointerId); } catch {}
  }

  wrap.addEventListener("pointerdown", startDrag);
  wrap.addEventListener("pointermove", moveDrag);
  wrap.addEventListener("pointerup", endDrag);
  wrap.addEventListener("pointercancel", endDrag);

  setTimeout(() => setValue(cfg.initial), 30);
}

/* INIT */
renderKeys();
updateSystemStatus({ cpu: 0, ram: 0 });
updateNowPlaying({ title: "No music", artist: "Waiting…", position: 0, duration: 0, isPlaying: false });

initSlider({ id: "slider-master", valueId: "slider-master-value", initial: 70 });
initSlider({ id: "slider-aux",    valueId: "slider-aux-value",    initial: 40 });
initSlider({ id: "slider-mic",    valueId: "slider-mic-value",    initial: 55 });
initSlider({ id: "slider-sys",    valueId: "slider-sys-value",    initial: 30 });

