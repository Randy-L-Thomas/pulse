import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

type Status = "ok" | "degraded" | "down";

type Action = {
  id: string;
  label: string;
  enabled: boolean;
};

type Cell = {
  id: string;
  label: string;
  status: Status;
  primary: string;
  detail: string;
  copy_text: string;
  actions: Action[];
};

type NetState = {
  status: Status;
  icmp_ms: number | null;
  https_ms: number | null;
  loss_pct: number;
  history: number[];
  detail: string;
  copy_text: string;
};

type Snapshot = {
  cells: Cell[];
  net: NetState;
};

const cellsEl = document.getElementById("cells") as HTMLElement;
const spark = document.getElementById("spark") as HTMLCanvasElement;
const netPrimary = document.getElementById("net-primary") as HTMLElement;
const netDetail = document.getElementById("net-detail") as HTMLElement;
const radial = document.getElementById("radial") as HTMLElement;
const toastEl = document.getElementById("toast") as HTMLElement;
const clockEl = document.getElementById("clock") as HTMLElement;
const pinBtn = document.getElementById("btn-pin") as HTMLButtonElement;
const win = getCurrentWindow();

let pinned = true;
let menuCell: Cell | null = null;
let lastNet: NetState | null = null;

function toast(msg: string) {
  toastEl.textContent = msg;
  toastEl.hidden = false;
  window.setTimeout(() => {
    toastEl.hidden = true;
  }, 2800);
}

function drawSpark(history: number[]) {
  const dpr = window.devicePixelRatio || 1;
  const cssW = spark.clientWidth || 560;
  const cssH = spark.clientHeight || 120;
  spark.width = Math.floor(cssW * dpr);
  spark.height = Math.floor(cssH * dpr);
  const ctx = spark.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, cssW, cssH);
  const pts = history.filter((n) => Number.isFinite(n) && n >= 0);
  if (pts.length < 2) return;
  const max = Math.max(80, ...pts);
  ctx.lineWidth = 1.6;
  ctx.strokeStyle = "#3fd4b0";
  ctx.shadowColor = "rgba(63, 212, 176, 0.55)";
  ctx.shadowBlur = 8;
  ctx.beginPath();
  pts.forEach((v, i) => {
    const x = (i / (pts.length - 1)) * (cssW - 8) + 4;
    const y = cssH - 6 - (v / max) * (cssH - 14);
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  });
  ctx.stroke();
}

function renderCells(cells: Cell[]) {
  cellsEl.replaceChildren();
  for (const cell of cells) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = `cell ${cell.status}`;
    btn.dataset.id = cell.id;
    btn.innerHTML = `<span class="pip"></span><span class="cell-name">${escapeHtml(
      cell.label,
    )}</span><span class="read">${escapeHtml(cell.primary)}</span><span class="detail">${escapeHtml(
      cell.detail,
    )}</span>`;
    btn.addEventListener("click", (ev) => openRadial(ev, cell));
    cellsEl.appendChild(btn);
  }
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
}

function openRadial(ev: MouseEvent, cell: Cell) {
  ev.stopPropagation();
  menuCell = cell;
  radial.hidden = false;
  radial.style.left = `${ev.clientX}px`;
  radial.style.top = `${ev.clientY}px`;
  for (const spoke of radial.querySelectorAll<HTMLButtonElement>(".spoke")) {
    const action = cell.actions.find((a) => a.id === spoke.dataset.action);
    spoke.disabled = !action?.enabled;
  }
}

function closeRadial() {
  radial.hidden = true;
  menuCell = null;
}

function renderNet(net: NetState) {
  lastNet = net;
  const ms = net.icmp_ms ?? net.https_ms;
  netPrimary.textContent = ms == null ? "down" : `${Math.round(ms)} ms`;
  netPrimary.style.color =
    net.status === "ok" ? "var(--ok)" : net.status === "degraded" ? "var(--warn)" : "var(--down)";
  netDetail.textContent = net.detail;
  drawSpark(net.history);
}

function applySnapshot(snap: Snapshot) {
  renderNet(snap.net);
  renderCells(snap.cells);
}

async function runAction(cell: Cell, action: string) {
  if (action === "copy") {
    await navigator.clipboard.writeText(cell.copy_text || cell.primary);
    toast("copied");
    return;
  }
  try {
    const msg = await invoke<string>("run_action", { cellId: cell.id, action });
    toast(msg || "ok");
  } catch (err) {
    const text = String(err);
    toast(text);
    if (text.includes("schtasks") || text.includes("Start-ScheduledTask")) {
      await navigator.clipboard.writeText(text);
    }
  }
}

document.getElementById("btn-half")!.addEventListener("click", () => invoke("set_width_mode", { mode: "half" }));
document.getElementById("btn-full")!.addEventListener("click", () => invoke("set_width_mode", { mode: "full" }));
document.getElementById("btn-min")!.addEventListener("click", () => win.minimize());
document.getElementById("btn-close")!.addEventListener("click", () => win.close());
pinBtn.addEventListener("click", async () => {
  pinned = !pinned;
  await win.setAlwaysOnTop(pinned);
  pinBtn.classList.toggle("on", pinned);
  pinBtn.setAttribute("aria-pressed", String(pinned));
});

radial.addEventListener("click", async (ev) => {
  const spoke = (ev.target as HTMLElement).closest<HTMLButtonElement>(".spoke");
  if (!spoke || !menuCell || spoke.disabled) return;
  const action = spoke.dataset.action!;
  const cell = menuCell;
  closeRadial();
  await runAction(cell, action);
});

document.addEventListener("click", (ev) => {
  if (!radial.hidden && !(ev.target as HTMLElement).closest(".radial, .cell")) closeRadial();
});
document.addEventListener("keydown", (ev) => {
  if (ev.key === "Escape") closeRadial();
});

document.querySelector(".trace")!.addEventListener("click", async (ev) => {
  if ((ev.target as HTMLElement).closest("canvas") || (ev.target as HTMLElement).closest(".trace")) {
    if (!lastNet) return;
    await navigator.clipboard.writeText(lastNet.copy_text);
    toast("copied rtt");
  }
});

function tickClock() {
  const d = new Date();
  clockEl.textContent = d.toTimeString().slice(0, 8);
}
tickClock();
window.setInterval(tickClock, 1000);
window.addEventListener("resize", () => {
  if (lastNet) drawSpark(lastNet.history);
});

listen<Snapshot>("snapshot", (ev) => applySnapshot(ev.payload));
invoke<Snapshot>("get_snapshot")
  .then(applySnapshot)
  .catch((err) => toast(String(err)));
