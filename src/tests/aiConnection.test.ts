import { describe, expect, it } from "vitest";
import { AI_PROTOCOL, DEEPSEEK_BASE_URL, DEEPSEEK_MODEL, DEEPSEEK_MODELS, connectionMode, newConnectionId, createConnectionDraft, validateConnectionDraft } from "../config/aiProviders";
import type { AiProviderConfig } from "../types/domain";

const savedCustom: AiProviderConfig = {
  providerId: "custom", providerType: AI_PROTOCOL, displayName: "自定义接口",
  baseUrl: "http://localhost:11434/v1", model: "local-model", enabled: true,
  stream: true, temperature: 0.8, maxOutputTokens: 1200, timeoutMs: 60000,
  hasApiKey: true, isActive: true, updatedAt: 0,
};

describe("chat connection settings", () => {
  it("uses the official DeepSeek defaults and does not inherit a custom key", () => {
    expect(createConnectionDraft("deepseek", savedCustom)).toEqual({ baseUrl: DEEPSEEK_BASE_URL, model: DEEPSEEK_MODEL, apiKey: "" });
  });

  it("restores custom endpoint and model without reading the saved key", () => {
    expect(createConnectionDraft("custom", savedCustom)).toEqual({ baseUrl: savedCustom.baseUrl, model: savedCustom.model, apiKey: "" });
  });

  it("restores each official model choice without reading its saved key", () => {
    for (const model of DEEPSEEK_MODELS) {
      const saved = { ...savedCustom, providerId: "deepseek", baseUrl: DEEPSEEK_BASE_URL, model: model.id };
      const draft = createConnectionDraft("deepseek", saved);
      expect(draft.model).toBe(model.id);
      expect(draft.apiKey).toBe("");
      expect(validateConnectionDraft("deepseek", draft, true)).toBeNull();
    }
  });

  it("keeps arbitrary custom model IDs out of the official model picker", () => {
    const draft = { ...createConnectionDraft("deepseek"), model: "custom-only-model" };
    expect(validateConnectionDraft("deepseek", draft, true)).toContain("DeepSeek 模型");
    expect(validateConnectionDraft("custom", draft, true)).toBeNull();
  });

  it("gives new profiles unique IDs and never carries another profile's typed key", () => {
    for (const mode of ["deepseek", "custom"] as const) {
      const id = newConnectionId(mode);
      expect(connectionMode(id)).toBe(mode);
      expect(newConnectionId(mode)).not.toBe(id);
      const draft = createConnectionDraft(mode);
      expect(draft.apiKey).toBe("");
    }
    expect(connectionMode("legacy-deepseek-example")).toBeNull();
    expect(connectionMode("deepseek-profile-not-a-uuid")).toBeNull();
  });

  it("requires a new or already-saved DeepSeek key", () => {
    const draft = createConnectionDraft("deepseek");
    expect(validateConnectionDraft("deepseek", draft, false)).toContain("API Key");
    expect(validateConnectionDraft("deepseek", draft, true)).toBeNull();
  });

  it("allows an unauthenticated local compatible endpoint", () => {
    expect(validateConnectionDraft("custom", { baseUrl: "http://localhost:11434/v1", model: "local-model", apiKey: "" }, false)).toBeNull();
  });

  it("rejects incomplete models and unsafe URL credentials", () => {
    expect(validateConnectionDraft("custom", { baseUrl: "https://example.com/v1", model: " ", apiKey: "" }, false)).toContain("模型");
    for (const baseUrl of ["file:///test", "https://user:pass@example.com/v1", "https://example.com/v1?key=test", "not-a-url"]) {
      expect(validateConnectionDraft("custom", { baseUrl, model: "test", apiKey: "" }, false)).not.toBeNull();
    }
  });
});
