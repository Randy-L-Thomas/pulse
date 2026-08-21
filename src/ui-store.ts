import { invoke } from "@tauri-apps/api/core";

export type UiState = {
  last_module: string;
  wa_title: string;
  mt_from: string;
  mt_to: string;
  mt_enrich: boolean;
  ollama_model: string;
  ollama_url: string;
  font_px?: number;
  cell_order: string[];
};

let cache: UiState | null = null;
let loading: Promise<UiState> | null = null;

function normalize(s: UiState): UiState {
  return {
    ...s,
    cell_order: Array.isArray(s.cell_order) ? s.cell_order : [],
  };
}

export function uiCache(): UiState | null {
  return cache;
}

export function loadUi(): Promise<UiState> {
  if (!loading) {
    loading = invoke<UiState>("get_ui").then((s) => {
      cache = normalize(s);
      return cache;
    });
  }
  return loading;
}

export async function saveUi(): Promise<void> {
  if (!cache) return;
  await invoke("save_ui", { ui: cache });
}
