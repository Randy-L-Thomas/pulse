import { invoke } from "@tauri-apps/api/core";

export type UiState = {
  last_module: string;
  wa_title: string;
  mt_from: string;
  mt_to: string;
  mt_enrich: boolean;
  ollama_model: string;
  ollama_url: string;
};

type WindowInfo = { title: string; hwnd: number };
type ToastFn = (msg: string, ms?: number) => void;

export function wireModules(toast: ToastFn) {
  const modules = document.getElementById("modules") as HTMLElement;
  const mtFrom = document.getElementById("mt-from") as HTMLSelectElement;
  const mtTo = document.getElementById("mt-to") as HTMLSelectElement;
  const mtEnrich = document.getElementById("mt-enrich") as HTMLInputElement;
  const mtSrc = document.getElementById("mt-src") as HTMLTextAreaElement;
  const mtDst = document.getElementById("mt-dst") as HTMLTextAreaElement;
  const mtWin = document.getElementById("mt-win") as HTMLSelectElement;
  const mtStatus = document.getElementById("mt-status") as HTMLElement;
  const chatModel = document.getElementById("chat-model") as HTMLSelectElement;
  const chatLog = document.getElementById("chat-log") as HTMLElement;
  const chatIn = document.getElementById("chat-in") as HTMLInputElement;
  const chatStatus = document.getElementById("chat-status") as HTMLElement;
  const messages: { role: string; content: string }[] = [];
  let ui: UiState | null = null;

  function showMod(id: string) {
    for (const pane of modules.querySelectorAll<HTMLElement>(".mod-pane")) {
      pane.hidden = pane.id !== `mod-${id}`;
    }
    for (const btn of modules.querySelectorAll<HTMLButtonElement>("[data-mod]")) {
      btn.classList.toggle("on", btn.dataset.mod === id);
    }
    if (ui) {
      ui.last_module = id;
      void persist();
    }
  }

  async function persist() {
    if (!ui) return;
    ui.mt_from = mtFrom.value;
    ui.mt_to = mtTo.value;
    ui.mt_enrich = mtEnrich.checked;
    ui.ollama_model = chatModel.value;
    ui.wa_title = mtWin.value || ui.wa_title;
    try {
      await invoke("save_ui", { ui });
    } catch (e) {
      toast(String(e));
    }
  }

  async function loadWindows() {
    try {
      const list = await invoke<WindowInfo[]>("list_app_windows");
      const prev = ui?.wa_title || "WhatsApp";
      mtWin.replaceChildren();
      const seen = new Set<string>();
      for (const w of list) {
        if (seen.has(w.title)) continue;
        seen.add(w.title);
        const opt = document.createElement("option");
        opt.value = w.title;
        opt.textContent = w.title.slice(0, 80);
        mtWin.appendChild(opt);
      }
      const hit = [...mtWin.options].find((o) => o.value.toLowerCase().includes(prev.toLowerCase()) || o.value === prev);
      if (hit) mtWin.value = hit.value;
      else if (prev) {
        const opt = document.createElement("option");
        opt.value = prev;
        opt.textContent = prev;
        mtWin.insertBefore(opt, mtWin.firstChild);
        mtWin.value = prev;
      }
    } catch (e) {
      mtStatus.textContent = String(e);
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
      if (ui?.ollama_model && names.includes(ui.ollama_model)) chatModel.value = ui.ollama_model;
      chatStatus.textContent = names.length ? `${names.length} models` : "no models";
    } catch (e) {
      chatStatus.textContent = String(e);
    }
  }

  modules.querySelectorAll<HTMLButtonElement>("[data-mod]").forEach((btn) => {
    btn.addEventListener("click", () => showMod(btn.dataset.mod || "translate"));
  });
  document.getElementById("mt-swap")!.addEventListener("click", () => {
    const a = mtFrom.value;
    mtFrom.value = mtTo.value;
    mtTo.value = a;
    void persist();
  });
  document.getElementById("mt-go")!.addEventListener("click", async () => {
    mtStatus.textContent = "…";
    try {
      await persist();
      const out = await invoke<{ text: string; cached: boolean; engine: string }>("translate_text", {
        source: mtSrc.value,
        from: mtFrom.value,
        to: mtTo.value,
        enrich: mtEnrich.checked,
      });
      mtDst.value = out.text;
      mtStatus.textContent = out.cached ? "cache" : out.engine;
    } catch (e) {
      mtStatus.textContent = String(e);
    }
  });
  document.getElementById("mt-bind")!.addEventListener("click", async () => {
    await loadWindows();
    if (ui && mtWin.value) {
      ui.wa_title = mtWin.value;
      await persist();
      mtStatus.textContent = `bound ${mtWin.value.slice(0, 40)}`;
    }
  });
  document.getElementById("mt-ocr")!.addEventListener("click", async () => {
    mtStatus.textContent = "ocr…";
    try {
      await persist();
      const text = await invoke<string>("capture_ocr");
      mtSrc.value = text;
      mtStatus.textContent = "ocr ok";
      if (text.trim()) document.getElementById("mt-go")!.click();
    } catch (e) {
      mtStatus.textContent = String(e);
    }
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
      chatStatus.textContent = String(e);
    }
  });
  chatIn.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter") document.getElementById("chat-send")!.click();
  });

  invoke<UiState>("get_ui")
    .then(async (s) => {
      ui = s;
      mtFrom.value = s.mt_from || "es";
      mtTo.value = s.mt_to || "en";
      mtEnrich.checked = !!s.mt_enrich;
      showMod(s.last_module === "chat" ? "chat" : "translate");
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
