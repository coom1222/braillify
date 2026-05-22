import { writeText as tauriWriteText } from "@tauri-apps/plugin-clipboard-manager";

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function copyText(text: string): Promise<void> {
  if (isTauri()) {
    await tauriWriteText(text);
    return;
  }
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  throw new Error("Clipboard API unavailable");
}
