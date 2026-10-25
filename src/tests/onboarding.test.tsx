import { afterEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { hasCompletedOnboarding, completeOnboarding } from "../systems/settings/onboarding";
import { WelcomePanel, WELCOME_STEPS } from "../components/WelcomePanel";
import { AiConnectionList } from "../components/AiConnectionList";
import type { AiProviderConfig } from "../types/domain";
import { SettingsPanel } from "../components/SettingsPanel";
import { getPanelLayout } from "../components/boxLayout";

afterEach(() => vi.unstubAllGlobals());

describe("lightweight onboarding", () => {
  it("remembers explicit completion or skipping without storing credentials", () => {
    const values = new Map<string, string>();
    const storage = { getItem: (key: string) => values.get(key) ?? null, setItem: (key: string, value: string) => { values.set(key, value); } };
    expect(hasCompletedOnboarding(storage)).toBe(false);
    completeOnboarding(storage);
    expect(hasCompletedOnboarding(storage)).toBe(true);
    expect([...values.values()]).toEqual(["true"]);
  });
  it("never blocks use when preference storage is unavailable", () => {
    expect(hasCompletedOnboarding({ getItem: () => { throw new Error("blocked"); } })).toBe(false);
    expect(() => completeOnboarding({ setItem: () => { throw new Error("blocked"); } })).not.toThrow();
  });
  it("starts with local interaction, allows skipping and does not ask for a key", () => {
    const html = renderToStaticMarkup(<WelcomePanel onFinish={vi.fn()} onConfigure={vi.fn()} onGuide={vi.fn()} />);
    expect(WELCOME_STEPS).toHaveLength(3);
    expect(html).toContain("跳过，先使用");
    expect(html).toContain("拖动");
    expect(html).not.toContain("type=\"password\"");
    expect(html).not.toContain("toolbox-nav");
  });
});

describe("saved connection list", () => {
  it("places the official model selector outside the DIY advanced section", () => {
    const html = renderToStaticMarkup(<SettingsPanel dataPaths={null} onOpenDataDir={vi.fn()} onClose={vi.fn()} onOpenGuide={vi.fn()} />);
    expect(html).toContain('aria-label="DeepSeek 聊天模型"');
    expect(html.indexOf('aria-label="DeepSeek 聊天模型"')).toBeLessThan(html.indexOf('data-testid="connection-advanced"'));
    expect(html).toContain('value="deepseek-v4-pro"');
    expect(html).toContain("仅保存");
    expect(html).toContain("查看独立指南");
  });

  it("shows names, models, masked hints and only one active configuration", () => {
    const base: AiProviderConfig = { providerId: "deepseek", providerType: "openai-compatible", displayName: "日常密钥", baseUrl: "https://api.deepseek.com", model: "deepseek-flash", hasApiKey: true, apiKeyHint: "•••• 1234", isActive: true, enabled: true, stream: true, temperature: 0.8, maxOutputTokens: 1200, timeoutMs: 60000, updatedAt: 0 };
    const html = renderToStaticMarkup(<AiConnectionList providers={[base, { ...base, providerId: "backup", displayName: "备用密钥", apiKeyHint: "•••• 5678", isActive: false }]} selectedId="deepseek" onSelect={vi.fn()} onActivate={vi.fn()} />);
    expect(html).toContain("日常密钥");
    expect(html).toContain("备用密钥");
    expect(html).toContain("•••• 1234");
    expect(html.match(/使用中/g)).toHaveLength(1);
    expect(html).toContain("启用 备用密钥");
  });
});

describe("compact panel layout", () => {
  it("keeps a larger preferred panel in view before the toolbox anchor is initialized", () => {
    for (const viewport of [{ width: 320, height: 568 }, { width: 800, height: 600 }, { width: 1280, height: 720 }]) {
      for (const size of [{ width: 400, height: 590 }, { width: 400, height: 460 }, { width: 420, height: 620 }]) {
        const panel = getPanelLayout(null, viewport, size);
        expect(panel.x).toBeGreaterThanOrEqual(14);
        expect(panel.y).toBeGreaterThanOrEqual(14);
        expect(panel.x + panel.width).toBeLessThanOrEqual(viewport.width - 14);
        expect(panel.y + panel.height).toBeLessThanOrEqual(viewport.height - 14);
      }
    }
  });

  it("separates the fixed action area from the scrollable settings form", () => {
    const html = renderToStaticMarkup(<SettingsPanel dataPaths={null} onOpenDataDir={vi.fn()} onClose={vi.fn()} onOpenGuide={vi.fn()} />);
    expect(html).toContain('class="settings-scroll ui-scroll"');
    expect(html).toContain('class="settings-footer"');
    expect(html).toContain('class="settings-action-grid"');
    expect(html).not.toContain('class="flex h-8 flex-1');
  });
});
