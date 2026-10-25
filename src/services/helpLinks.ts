import { invoke } from "@tauri-apps/api/core";
import links from "../config/deepseekHelpLinks.json";

export type DeepSeekHelpLink = keyof typeof links;
export const deepseekHelpLinks = links;

/** Only named, bundled official destinations may be opened from the app. */
export async function openDeepSeekHelpLink(id: DeepSeekHelpLink): Promise<void> {
  if (!Object.hasOwn(links, id)) throw new Error("不支持的帮助链接。");
  if (window.__TAURI_INTERNALS__) {
    await invoke("open_deepseek_help_link", { id });
    return;
  }
  const opened = window.open(links[id].url, "_blank", "noopener,noreferrer");
  // noopener may deliberately make window.open return null even on success.
  if (opened) opened.opener = null;
}
