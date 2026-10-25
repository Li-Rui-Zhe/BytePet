import type { AiProviderConfig } from "../types/domain";
import deepseekModels from "./deepseekModels.json";

export type AiConnectionMode = "deepseek" | "custom";
export const DEEPSEEK_BASE_URL = "https://api.deepseek.com";
export const DEEPSEEK_MODEL = deepseekModels.defaultModel;
export const DEEPSEEK_MODELS = deepseekModels.models;
export const AI_PROTOCOL = "openai-compatible";

export function connectionMode(providerId: string): AiConnectionMode | null {
  if (providerId === "deepseek" || /^deepseek-profile-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(providerId)) return "deepseek";
  if (providerId === "custom" || /^custom-profile-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(providerId)) return "custom";
  return null;
}

export function newConnectionId(mode: AiConnectionMode): string {
  return `${mode}-profile-${crypto.randomUUID()}`;
}

export interface AiConnectionDraft {
  baseUrl: string;
  model: string;
  apiKey: string;
}

export function createConnectionDraft(mode: AiConnectionMode, saved?: AiProviderConfig): AiConnectionDraft {
  return {
    baseUrl: mode === "deepseek" ? DEEPSEEK_BASE_URL : saved?.baseUrl ?? "",
    model: mode === "deepseek"
      ? saved && connectionMode(saved.providerId) === "deepseek" && DEEPSEEK_MODELS.some(item => item.id === saved.model) ? saved.model : DEEPSEEK_MODEL
      : saved?.model ?? "",
    apiKey: "",
  };
}

export function validateConnectionDraft(mode: AiConnectionMode, draft: AiConnectionDraft, hasSavedKey: boolean): string | null {
  if (mode === "deepseek" && !draft.apiKey.trim() && !hasSavedKey) return "请填写 DeepSeek API Key。";
  if (!draft.model.trim()) return "请填写模型名称。";
  if (mode === "deepseek" && !DEEPSEEK_MODELS.some(item => item.id === draft.model.trim())) return "请选择列表中的 DeepSeek 模型。";
  try {
    const url = new URL(draft.baseUrl.trim());
    if (!["http:", "https:"].includes(url.protocol) || url.username || url.password || url.search || url.hash) {
      return "请填写不含账号、密码或查询参数的 HTTP / HTTPS 接口地址。";
    }
  } catch {
    return "请填写完整的接口地址，例如 https://api.example.com/v1。";
  }
  return null;
}
