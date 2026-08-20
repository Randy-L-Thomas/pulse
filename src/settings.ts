import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { openUrl } from "@tauri-apps/plugin-opener";
import { disable as disableAutostart, enable as enableAutostart, isEnabled as autostartEnabled } from "@tauri-apps/plugin-autostart";

export type UtilCfg = { warn_pct: number; crit_pct: number };
export type GpuCfg = UtilCfg & { vram_warn_pct: number; vram_crit_pct: number };
export type HttpCfg = {
  id: string;
  label: string;
  url: string;
  also?: string | null;
  open?: string | null;
  open_fallback?: string | null;
  start_program?: string | null;
  start_args?: string[] | null;
  start_cwd?: string | null;
  stop_program?: string | null;
  stop_args?: string[] | null;
  restart_program?: string | null;
  restart_args?: string[] | null;
  task?: string | null;
  stdio_match?: string | null;
};
export type FileCfg = {
  id: string;
  label: string;
  path: string;
  bool_field?: string | null;
  string_field?: string | null;
  ok_value?: string | null;
  open?: string | null;
};
export type ProcessCfg = {
  id: string;
  label: string;
  exe_name: string;
  path_contains: string[];
  exclude_names: string[];
  exclude_paths: string[];
  warn_gb: number;
  crit_gb: number;
  open_exe?: string | null;
};
export type Settings = {
  height: number;
  half_width: number;
  full_width: number;
  monitor_width: number;
  monitor_height: number;
  stale_secs: number;
  show: { path: boolean; cpu: boolean; memory: boolean; gpu: boolean };
  net: { host: string; https_url: string; history: number };
  path: { dns_host: string };
  cpu: UtilCfg;
  memory: UtilCfg;
  gpu: GpuCfg;
  http: HttpCfg[];
  file: FileCfg[];
  process: ProcessCfg[];
  launch_allow: string[];
};

const RELEASES_URL = "https://github.com/Randy-L-Thomas/pulse/releases/latest";

type ToastFn = (msg: string, ms?: number) => void;

export function wireSettings(toast: ToastFn) {
  const overlay = document.getElementById("settings") as HTMLElement;
  const openBtn = document.getElementById("btn-settings") as HTMLButtonElement;
  const closeBtn = document.getElementById("btn-settings-close") as HTMLButtonElement;
  const saveBtn = document.getElementById("btn-settings-save") as HTMLButtonElement;
  let snapshot: Settings | null = null;

  function showTab(id: string) {
    for (const tab of overlay.querySelectorAll<HTMLElement>(".settings-tab")) {
      tab.hidden = tab.id !== `tab-${id}`;
    }
    for (const btn of overlay.querySelectorAll<HTMLButtonElement>("[data-tab]")) {
      btn.classList.toggle("on", btn.dataset.tab === id);
    }
  }

  async function load() {
    snapshot = await invoke<Settings>("get_settings");
    const s = snapshot;
    (document.getElementById("set-mon-w") as HTMLInputElement).value = String(s.monitor_width);
    (document.getElementById("set-mon-h") as HTMLInputElement).value = String(s.monitor_height);
    (document.getElementById("show-path") as HTMLInputElement).checked = s.show.path;
    (document.getElementById("show-cpu") as HTMLInputElement).checked = s.show.cpu;
    (document.getElementById("show-memory") as HTMLInputElement).checked = s.show.memory;
    (document.getElementById("show-gpu") as HTMLInputElement).checked = s.show.gpu;
    (document.getElementById("cpu-warn") as HTMLInputElement).value = String(s.cpu.warn_pct);
    (document.getElementById("cpu-crit") as HTMLInputElement).value = String(s.cpu.crit_pct);
    (document.getElementById("mem-warn") as HTMLInputElement).value = String(s.memory.warn_pct);
    (document.getElementById("mem-crit") as HTMLInputElement).value = String(s.memory.crit_pct);
    (document.getElementById("gpu-warn") as HTMLInputElement).value = String(s.gpu.warn_pct);
    (document.getElementById("gpu-crit") as HTMLInputElement).value = String(s.gpu.crit_pct);
    (document.getElementById("gpu-vram-warn") as HTMLInputElement).value = String(s.gpu.vram_warn_pct ?? 80);
    (document.getElementById("gpu-vram-crit") as HTMLInputElement).value = String(s.gpu.vram_crit_pct ?? 95);
    (document.getElementById("set-allow") as HTMLInputElement).value = (s.launch_allow || []).join(", ");
    const pinOn = document.getElementById("btn-pin")?.classList.contains("on") ?? true;
    document.getElementById("set-pin")!.classList.toggle("on", pinOn);
    try {
      (document.getElementById("set-autostart") as HTMLInputElement).checked = await autostartEnabled();
    } catch {
      (document.getElementById("set-autostart") as HTMLInputElement).checked = false;
    }
    renderHttp(s.http);
    renderProcs(s.process);
    const meta = await invoke<{ version: string; config_path: string }>("app_meta");
    (document.getElementById("update-meta") as HTMLElement).textContent =
      `v${meta.version}  ·  ${meta.config_path}`;
  }

  function renderHttp(list: HttpCfg[]) {
    const box = document.getElementById("http-list") as HTMLElement;
    box.replaceChildren();
    list.forEach((h, i) => {
      box.appendChild(row([
        field("id", h.id, i, "http"),
        field("label", h.label, i, "http"),
        field("url", h.url, i, "http", true),
        field("open", h.open ?? "", i, "http", true),
      ], () => {
        snapshot!.http.splice(i, 1);
        renderHttp(snapshot!.http);
      }));
    });
  }

  function renderProcs(list: ProcessCfg[]) {
    const box = document.getElementById("proc-list") as HTMLElement;
    box.replaceChildren();
    list.forEach((p, i) => {
      box.appendChild(row([
        field("id", p.id, i, "proc"),
        field("label", p.label, i, "proc"),
        field("exe", p.exe_name, i, "proc"),
        field("warn_gb", String(p.warn_gb), i, "proc"),
        field("crit_gb", String(p.crit_gb), i, "proc"),
      ], () => {
        snapshot!.process.splice(i, 1);
        renderProcs(snapshot!.process);
      }));
    });
  }

  function field(name: string, value: string, index: number, kind: string, wide = false) {
    const input = document.createElement("input");
    input.type = "text";
    input.dataset.k = name;
    input.dataset.i = String(index);
    input.dataset.kind = kind;
    input.value = value;
    input.className = wide ? "wide" : "";
    input.placeholder = name;
    input.addEventListener("change", () => {
      if (!snapshot) return;
      const i = Number(input.dataset.i);
      const v = input.value;
      if (kind === "http") {
        const h = snapshot.http[i];
        if (name === "id") h.id = v;
        if (name === "label") h.label = v;
        if (name === "url") h.url = v;
        if (name === "open") h.open = v || null;
      } else {
        const p = snapshot.process[i];
        if (name === "id") p.id = v;
        if (name === "label") p.label = v;
        if (name === "exe") p.exe_name = v;
        if (name === "warn_gb") p.warn_gb = Number(v) || 0;
        if (name === "crit_gb") p.crit_gb = Number(v) || 0;
      }
    });
    return input;
  }

  function row(inputs: HTMLElement[], onRemove: () => void) {
    const wrap = document.createElement("div");
    wrap.className = "stack-row";
    for (const el of inputs) wrap.appendChild(el);
    const rm = document.createElement("button");
    rm.type = "button";
    rm.className = "tb";
    rm.textContent = "Remove";
    rm.addEventListener("click", onRemove);
    wrap.appendChild(rm);
    return wrap;
  }

  function readForm(): Settings {
    const s = snapshot!;
    s.monitor_width = Number((document.getElementById("set-mon-w") as HTMLInputElement).value) || 1920;
    s.monitor_height = Number((document.getElementById("set-mon-h") as HTMLInputElement).value) || 440;
    s.show.path = (document.getElementById("show-path") as HTMLInputElement).checked;
    s.show.cpu = (document.getElementById("show-cpu") as HTMLInputElement).checked;
    s.show.memory = (document.getElementById("show-memory") as HTMLInputElement).checked;
    s.show.gpu = (document.getElementById("show-gpu") as HTMLInputElement).checked;
    s.cpu.warn_pct = Number((document.getElementById("cpu-warn") as HTMLInputElement).value);
    s.cpu.crit_pct = Number((document.getElementById("cpu-crit") as HTMLInputElement).value);
    s.memory.warn_pct = Number((document.getElementById("mem-warn") as HTMLInputElement).value);
    s.memory.crit_pct = Number((document.getElementById("mem-crit") as HTMLInputElement).value);
    s.gpu.warn_pct = Number((document.getElementById("gpu-warn") as HTMLInputElement).value);
    s.gpu.crit_pct = Number((document.getElementById("gpu-crit") as HTMLInputElement).value);
    s.gpu.vram_warn_pct = Number((document.getElementById("gpu-vram-warn") as HTMLInputElement).value);
    s.gpu.vram_crit_pct = Number((document.getElementById("gpu-vram-crit") as HTMLInputElement).value);
    s.launch_allow = (document.getElementById("set-allow") as HTMLInputElement).value
      .split(",")
      .map((x) => x.trim())
      .filter(Boolean);
    return s;
  }

  openBtn.addEventListener("click", async () => {
    overlay.hidden = false;
    showTab("general");
    try {
      await load();
    } catch (e) {
      toast(String(e));
    }
  });
  closeBtn.addEventListener("click", () => {
    overlay.hidden = true;
  });
  window.addEventListener("pointerdown", (ev) => {
    if (overlay.hidden) return;
    const t = ev.target as Node;
    if (overlay.contains(t) || openBtn.contains(t)) return;
    overlay.hidden = true;
  });
  document.getElementById("set-half")!.addEventListener("click", () => {
    document.getElementById("btn-half")!.click();
  });
  document.getElementById("set-full")!.addEventListener("click", () => {
    document.getElementById("btn-full")!.click();
  });
  document.getElementById("set-pin")!.addEventListener("click", () => {
    document.getElementById("btn-pin")!.click();
    window.setTimeout(() => {
      const pinOn = document.getElementById("btn-pin")!.classList.contains("on");
      document.getElementById("set-pin")!.classList.toggle("on", pinOn);
    }, 50);
  });
  document.getElementById("set-autostart")!.addEventListener("change", async (ev) => {
    const on = (ev.target as HTMLInputElement).checked;
    try {
      if (on) await enableAutostart();
      else await disableAutostart();
    } catch (e) {
      toast(String(e));
      (ev.target as HTMLInputElement).checked = await autostartEnabled().catch(() => !on);
    }
  });
  overlay.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((btn) => {
    btn.addEventListener("click", () => showTab(btn.dataset.tab || "general"));
  });
  document.getElementById("btn-add-http")!.addEventListener("click", () => {
    if (!snapshot) return;
    snapshot.http.push({ id: "svc", label: "svc", url: "http://127.0.0.1:80/health" });
    renderHttp(snapshot.http);
  });
  document.getElementById("btn-add-proc")!.addEventListener("click", () => {
    if (!snapshot) return;
    snapshot.process.push({
      id: "app",
      label: "App",
      exe_name: "app",
      path_contains: [],
      exclude_names: ["pulse"],
      exclude_paths: [],
      warn_gb: 3,
      crit_gb: 4,
    });
    renderProcs(snapshot.process);
  });
  document.getElementById("btn-preset-tk421")!.addEventListener("click", async () => {
    try {
      await invoke("apply_preset", { name: "tk421" });
      toast("TK421 preset");
      await load();
    } catch (e) {
      toast(String(e));
    }
  });
  document.getElementById("btn-preset-generic")!.addEventListener("click", async () => {
    try {
      await invoke("apply_preset", { name: "generic" });
      toast("generic preset");
      await load();
    } catch (e) {
      toast(String(e));
    }
  });
  saveBtn.addEventListener("click", async () => {
    if (!snapshot) return;
    try {
      const msg = await invoke<string>("save_settings", { cfg: readForm() });
      toast(msg);
      overlay.hidden = true;
    } catch (e) {
      toast(String(e));
    }
  });
  document.getElementById("btn-open-releases")!.addEventListener("click", async () => {
    try {
      await openUrl(RELEASES_URL);
    } catch {
      toast("open " + RELEASES_URL, 4000);
    }
  });
  document.getElementById("btn-check-update")!.addEventListener("click", async () => {
    const status = document.getElementById("update-status") as HTMLElement;
    const last = document.getElementById("update-last") as HTMLElement;
    status.textContent = "checking…";
    last.textContent = `last checked ${new Date().toLocaleTimeString()}`;
    try {
      const update = await check();
      if (!update) {
        status.textContent = "up to date";
        return;
      }
      status.textContent = `v${update.version} — downloading`;
      await update.downloadAndInstall();
      status.textContent = "restarting";
      await relaunch();
    } catch (e) {
      status.textContent = String(e);
      toast(String(e), 5000);
    }
  });

  document.addEventListener("keydown", (ev) => {
    if (ev.key === "Escape" && !overlay.hidden) overlay.hidden = true;
  });
}
