import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type ToastFn = (msg: string, ms?: number) => void;

let inFlight = false;

export function updateErrorText(e: unknown): string {
  if (typeof e === "string" && e.trim()) return e;
  if (e instanceof Error && e.message.trim()) return e.message;
  if (e && typeof e === "object") {
    const o = e as Record<string, unknown>;
    if (typeof o.message === "string" && o.message.trim()) return o.message;
    if (typeof o.error === "string" && o.error.trim()) return o.error;
  }
  const s = String(e);
  return s === "[object Object]" ? "update failed" : s;
}

function paintStatus(status: string) {
  const el = document.getElementById("update-status");
  if (el) el.textContent = status;
  if (status !== "checking…") return;
  const last = document.getElementById("update-last");
  if (last) last.textContent = `last checked ${new Date().toLocaleTimeString()}`;
}

function setBusy(busy: boolean) {
  for (const id of ["btn-update", "btn-check-update"]) {
    const btn = document.getElementById(id) as HTMLButtonElement | null;
    if (btn) btn.disabled = busy;
  }
}

/** Same path as Settings → Update → Check now: check, downloadAndInstall, relaunch. */
export async function runAppUpdate(onStatus?: (status: string) => void): Promise<void> {
  if (inFlight) return;
  inFlight = true;
  setBusy(true);
  try {
    paintStatus("checking…");
    onStatus?.("checking…");
    const update = await check();
    if (!update) {
      paintStatus("up to date");
      onStatus?.("up to date");
      return;
    }
    const downloading = `v${update.version} — downloading`;
    paintStatus(downloading);
    onStatus?.(downloading);
    await update.downloadAndInstall();
    paintStatus("restarting");
    onStatus?.("restarting");
    await relaunch();
  } catch (e) {
    paintStatus(updateErrorText(e));
    throw e;
  } finally {
    inFlight = false;
    setBusy(false);
  }
}

export function wireTitlebarUpdate(toast: ToastFn) {
  const btn = document.getElementById("btn-update") as HTMLButtonElement;
  btn.addEventListener("click", async () => {
    if (inFlight) return;
    const label = btn.textContent;
    btn.textContent = "…";
    try {
      await runAppUpdate((status) => {
        toast(status, status === "up to date" ? 2800 : 5000);
      });
    } catch (e) {
      toast(updateErrorText(e), 5000);
    } finally {
      btn.textContent = label;
    }
  });
}
