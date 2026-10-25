import { afterEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { DeepSeekGuide } from "../components/DeepSeekGuide";
import { HelpPanel } from "../components/HelpPanel";
import { deepseekHelpLinks, openDeepSeekHelpLink, type DeepSeekHelpLink } from "../services/helpLinks";

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke }));
afterEach(() => { vi.unstubAllGlobals(); vi.clearAllMocks(); });

describe("DeepSeek onboarding", () => {
  it("only bundles official HTTPS links, without secret URL parameters", () => {
    for (const { url } of Object.values(deepseekHelpLinks)) {
      const parsed = new URL(url);
      expect(parsed.protocol).toBe("https:");
      expect(["platform.deepseek.com", "api-docs.deepseek.com"]).toContain(parsed.hostname);
      expect(parsed.username + parsed.password + parsed.search + parsed.hash).toBe("");
    }
  });

  it("opens native links by ID, never by user-supplied URL", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {}, open: vi.fn() });
    native.invoke.mockResolvedValue(undefined);
    await openDeepSeekHelpLink("keys");
    expect(native.invoke).toHaveBeenCalledExactlyOnceWith("open_deepseek_help_link", { id: "keys" });
  });

  it("rejects unknown IDs before native or browser actions", async () => {
    const open = vi.fn();
    vi.stubGlobal("window", { open });
    for (const id of ["https://evil.example", "__proto__", "constructor"]) {
      await expect(openDeepSeekHelpLink(id as DeepSeekHelpLink)).rejects.toThrow("不支持");
    }
    expect(native.invoke).not.toHaveBeenCalled();
    expect(open).not.toHaveBeenCalled();
  });

  it("supports browser preview with a separate, opener-isolated tab", async () => {
    const open = vi.fn(() => null);
    vi.stubGlobal("window", { open });
    await openDeepSeekHelpLink("docs");
    expect(open).toHaveBeenCalledExactlyOnceWith(deepseekHelpLinks.docs.url, "_blank", "noopener,noreferrer");
  });

  it("renders the full offline tutorial without requests or clipboard access", () => {
    const onOpenLink = vi.fn();
    const html = renderToStaticMarkup(<DeepSeekGuide onBack={vi.fn()} onOpenLink={onOpenLink} />);
    for (const text of ["登录 DeepSeek 开放平台", "创建并复制 API Key", "回到桌宠，粘贴并测试", "保存并开始聊天", "Ctrl + V", "系统凭据库", "401", "402", "429", "不逐家适配", "欢迎提交 Issue", "测试只验证当前草稿", "按用量计费"]) {
      expect(html).toContain(text);
    }
    expect(onOpenLink).not.toHaveBeenCalled();
  });

  it("starts with local use, not mandatory API configuration", () => {
    const html = renderToStaticMarkup(<HelpPanel onClose={vi.fn()} onSettings={vi.fn()} onWelcome={vi.fn()} />);
    for (const text of ["指南分类", "先开始", "连接 AI", "问题排查", "桌宠快速上手", "不填 API Key", "显示工具箱", "退出", "不代表角色会自动永久记住"]) expect(html).toContain(text);
    expect(html).not.toContain("创建并复制 API Key");
  });

  it("can deep-link to setup and keeps troubleshooting separate", () => {
    const setup = renderToStaticMarkup(<HelpPanel initialTopic="deepseek" onClose={vi.fn()} onSettings={vi.fn()} onWelcome={vi.fn()} />);
    expect(setup).toContain("创建并复制 API Key");
    expect(setup).not.toContain("401 / 密钥认证失败");
    expect(setup).not.toContain("DeepSeek 已保存并启用");
    const troubleshooting = renderToStaticMarkup(<DeepSeekGuide mode="troubleshooting" onBack={vi.fn()} onOpenLink={vi.fn()} />);
    expect(troubleshooting).toContain("401 / 密钥认证失败");
    expect(troubleshooting).toContain("400 / 422");
    expect(troubleshooting).not.toContain("创建并复制 API Key");
  });
});
