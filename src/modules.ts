import { invoke } from "@tauri-apps/api/core";
import { applyFontPx, currentFontPx, FONT_PX_DEFAULT } from "./ui-font";
import { loadUi, saveUi, uiCache } from "./ui-store";

type WindowInfo = { title: string; hwnd: number };
type ToastFn = (msg: string, ms?: number) => void;

const MODS = ["translate", "ocr", "chat"];

export function wireModules(toast: ToastFn) {
  const modules = document.getElementById("modules") as HTMLElement;
  const mtFrom = document.getElementById("mt-from") as HTMLSelectElement;
  const mtTo = document.getElementById("mt-to") as HTMLSelectElement;
  const mtSrc = document.getElementById("mt-src") as HTMLTextAreaElement;
  const mtDst = document.getElementById("mt-dst") as HTMLTextAreaElement;
  const mtStatus = document.getElementById("mt-status") as HTMLElement;
  const ocrWin = document.getElementById("ocr-win") as HTMLSelectElement;
  const ocrOut = document.getElementById("ocr-out") as HTMLTextAreaElement;
  const ocrStatus = document.getElementById("ocr-status") as HTMLElement;
  const chatModel = document.getElementById("chat-model") as HTMLSelectElement;
  const chatLog = document.getElementById("chat-log") as HTMLElement;
  const chatIn = document.getElementById("chat-in") as HTMLInputElement;
  const chatStatus = document.getElementById("chat-status") as HTMLElement;
  const messages: { role: string; content: string }[] = [];
  let mtTimer = 0;
  let mtInFlight = false;
  let mtQueued = false;

  function showMod(id: string) {
    const next = MODS.includes(id) ? id : "translate";
    for (const pane of modules.querySelectorAll<HTMLElement>(".mod-pane")) {
      pane.hidden = pane.id !== `mod-${next}`;
    }
    for (const btn of modules.querySelectorAll<HTMLButtonElement>("[data-mod]")) {
      btn.classList.toggle("on", btn.dataset.mod === next);
    }
    const ui = uiCache();
    if (ui) {
      ui.last_module = next;
      void persist();
    }
  }

  function pickLang(value: string, fallback: string) {
    return value === "en" || value === "es" ? value : fallback;
  }

  function ollamaHostLabel(url: string): string {
    try {
      const u = new URL(url);
      return u.port ? `${u.hostname}:${u.port}` : u.hostname;
    } catch {
      return url.replace(/^https?:\/\//i, "").replace(/\/$/, "") || "127.0.0.1:11434";
    }
  }

  function ollamaDownMsg(): string {
    const url = uiCache()?.ollama_url || "http://127.0.0.1:11434";
    return `Ollama not running on ${ollamaHostLabel(url)}`;
  }

  function ollamaError(err: unknown): string {
    const s = String(err);
    if (/^Ollama not running on /i.test(s)) return s;
    if (
      /connection refused|actively refused|tcp connect|error sending request|timed out|timeout|connect error|11434/i.test(
        s,
      )
    ) {
      return ollamaDownMsg();
    }
    return s;
  }

  async function persist() {
    const ui = uiCache();
    if (!ui) return;
    ui.mt_from = pickLang(mtFrom.value, "es");
    ui.mt_to = pickLang(mtTo.value, "en");
    ui.mt_enrich = false;
    ui.ollama_model = chatModel.value;
    ui.wa_title = ocrWin.value || ui.wa_title;
    ui.font_px = currentFontPx();
    try {
      await saveUi();
    } catch (e) {
      toast(String(e));
    }
  }

  async function loadWindows() {
    try {
      const list = await invoke<WindowInfo[]>("list_app_windows");
      const prev = uiCache()?.wa_title || "WhatsApp";
      ocrWin.replaceChildren();
      const seen = new Set<string>();
      for (const w of list) {
        if (seen.has(w.title)) continue;
        seen.add(w.title);
        const opt = document.createElement("option");
        opt.value = w.title;
        opt.textContent = w.title.slice(0, 80);
        ocrWin.appendChild(opt);
      }
      const hit = [...ocrWin.options].find(
        (o) => o.value.toLowerCase().includes(prev.toLowerCase()) || o.value === prev,
      );
      if (hit) ocrWin.value = hit.value;
      else if (prev) {
        const opt = document.createElement("option");
        opt.value = prev;
        opt.textContent = prev;
        ocrWin.insertBefore(opt, ocrWin.firstChild);
        ocrWin.value = prev;
      }
    } catch (e) {
      ocrStatus.textContent = String(e);
    }
  }

  async function loadModels() {
    chatStatus.textContent = "…";
    try {
      const names = await invoke<string[]>("ollama_models");
      chatModel.replaceChildren();
      for (const n of names) {
        const opt = document.createElement("option");
        opt.value = n;
        opt.textContent = n;
        chatModel.appendChild(opt);
      }
      const savedModel = uiCache()?.ollama_model;
      if (savedModel && names.includes(savedModel)) chatModel.value = savedModel;
      chatStatus.textContent = names.length ? `${names.length} models` : "no models";
    } catch (e) {
      chatStatus.textContent = ollamaError(e);
    }
  }

  async function runTranslate() {
    if (mtInFlight) {
      mtQueued = true;
      return;
    }
    mtInFlight = true;
    try {
      do {
        mtQueued = false;
        const source = mtSrc.value;
        const from = pickLang(mtFrom.value, "es");
        const to = pickLang(mtTo.value, "en");
        if (!source.trim()) {
          mtDst.value = "";
          mtStatus.textContent = "";
          break;
        }
        if (from === to) {
          mtDst.value = source.trim();
          mtStatus.textContent = "same";
          continue;
        }
        mtStatus.textContent = "…";
        try {
          const out = await invoke<{ text: string; cached: boolean; engine: string }>("translate_text", {
            source,
            from,
            to,
            enrich: false,
          });
          if (mtQueued) continue;
          if (
            mtSrc.value !== source ||
            pickLang(mtFrom.value, "es") !== from ||
            pickLang(mtTo.value, "en") !== to
          ) {
            continue;
          }
          mtDst.value = out.text;
          mtStatus.textContent = out.engine === "same" ? "same" : out.cached ? "cache" : out.engine;
        } catch (e) {
          if (mtQueued) continue;
          if (
            mtSrc.value !== source ||
            pickLang(mtFrom.value, "es") !== from ||
            pickLang(mtTo.value, "en") !== to
          ) {
            continue;
          }
          mtDst.value = "";
          mtStatus.textContent = ollamaError(e);
        }
      } while (mtQueued);
    } finally {
      mtInFlight = false;
      if (mtQueued) {
        mtQueued = false;
        void runTranslate();
      }
    }
  }

  function scheduleTranslate() {
    window.clearTimeout(mtTimer);
    mtTimer = window.setTimeout(() => void runTranslate(), 450);
  }

  modules.querySelectorAll<HTMLButtonElement>("[data-mod]").forEach((btn) => {
    btn.addEventListener("click", () => showMod(btn.dataset.mod || "translate"));
  });
  document.getElementById("mt-swap")!.addEventListener("click", () => {
    const from = pickLang(mtFrom.value, "es");
    const to = pickLang(mtTo.value, "en");
    mtFrom.value = to;
    mtTo.value = from;
    const src = mtSrc.value;
    mtSrc.value = mtDst.value;
    mtDst.value = src;
    void persist();
    void runTranslate();
  });
  document.getElementById("mt-copy")!.addEventListener("click", async () => {
    const text = mtDst.value;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      mtStatus.textContent = "copied";
    } catch (e) {
      toast(String(e));
    }
  });
  document.getElementById("mt-clear")!.addEventListener("click", () => {
    mtQueued = false;
    mtSrc.value = "";
    mtDst.value = "";
    mtStatus.textContent = "";
  });
  mtFrom.addEventListener("change", () => {
    void persist();
    void runTranslate();
  });
  mtTo.addEventListener("change", () => {
    void persist();
    void runTranslate();
  });
  mtSrc.addEventListener("input", scheduleTranslate);
  mtSrc.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter" && (ev.ctrlKey || ev.metaKey)) {
      ev.preventDefault();
      void runTranslate();
    }
  });
  document.getElementById("ocr-bind")!.addEventListener("click", async () => {
    await loadWindows();
    const ui = uiCache();
    if (ui && ocrWin.value) {
      ui.wa_title = ocrWin.value;
      await persist();
      ocrStatus.textContent = `bound ${ocrWin.value.slice(0, 40)}`;
    }
  });
  document.getElementById("ocr-go")!.addEventListener("click", async () => {
    ocrStatus.textContent = "ocr…";
    try {
      await persist();
      const text = await invoke<string>("capture_ocr");
      ocrOut.value = text;
      ocrStatus.textContent = text.trim() ? "ok" : "empty";
    } catch (e) {
      ocrStatus.textContent = String(e);
    }
  });
  document.getElementById("ocr-to-mt")!.addEventListener("click", () => {
    const text = ocrOut.value.trim();
    if (!text) {
      ocrStatus.textContent = "nothing to translate";
      return;
    }
    mtSrc.value = text;
    showMod("translate");
    void runTranslate();
  });
  document.getElementById("chat-refresh")!.addEventListener("click", () => void loadModels());
  document.getElementById("chat-send")!.addEventListener("click", async () => {
    const text = chatIn.value.trim();
    if (!text) return;
    chatIn.value = "";
    messages.push({ role: "user", content: text });
    const me = document.createElement("div");
    me.className = "me";
    me.textContent = text;
    chatLog.appendChild(me);
    chatStatus.textContent = "…";
    try {
      await persist();
      const reply = await invoke<string>("ollama_chat", {
        model: chatModel.value,
        messages,
      });
      messages.push({ role: "assistant", content: reply });
      const bot = document.createElement("div");
      bot.className = "bot";
      bot.textContent = reply;
      chatLog.appendChild(bot);
      chatLog.scrollTop = chatLog.scrollHeight;
      chatStatus.textContent = "ok";
    } catch (e) {
      chatStatus.textContent = ollamaError(e);
    }
  });
  chatIn.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter") document.getElementById("chat-send")!.click();
  });

  loadUi()
    .then(async (s) => {
      applyFontPx(s.font_px ?? FONT_PX_DEFAULT);
      mtFrom.value = pickLang(s.mt_from, "es");
      mtTo.value = pickLang(s.mt_to, "en");
      if (mtFrom.value === mtTo.value) {
        mtFrom.value = "es";
        mtTo.value = "en";
      }
      showMod(s.last_module);
      await loadWindows();
      await loadModels();
    })
    .catch((e) => toast(String(e)));
}

export function setFullLayout(full: boolean) {
  document.body.classList.toggle("full", full);
  const modules = document.getElementById("modules") as HTMLElement;
  modules.hidden = !full;
}
