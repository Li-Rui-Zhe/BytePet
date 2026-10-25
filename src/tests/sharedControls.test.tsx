import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { RangeSlider } from "../components/RangeSlider";
import { ThemePanel } from "../components/ThemePanel";
import { SkinPanel } from "../components/SkinPanel";
import { ChatPanel } from "../components/ChatPanel";
import { createMockBootstrap } from "../storage/database/mockData";
import { chatInputHeight, shouldSendChatOnEnter } from "../utils/chatComposer";

describe("shared range slider", () => {
  it("preserves native labels, constraints, disabled state and value", () => {
    const html = renderToStaticMarkup(<RangeSlider label="角色大小" min={0} max={100} value={25} step={5} aria-valuetext="25%" disabled onChange={vi.fn()} />);
    for (const text of ['type="range"', 'aria-label="角色大小"', 'aria-valuetext="25%"', 'min="0"', 'max="100"', 'step="5"', 'value="25"', 'disabled=""', '--range-progress:25%']) expect(html).toContain(text);
  });
  it("clamps the visual progress even for out-of-bounds or zero-width ranges", () => {
    for (const [value, min, max, progress] of [[-10, 0, 100, 0], [110, 0, 100, 100], [2, 2, 2, 0]]) {
      const html = renderToStaticMarkup(<RangeSlider label="范围" value={value} min={min} max={max} onChange={vi.fn()} />);
      expect(html).toContain(`--range-progress:${progress}%`);
    }
  });
});

describe("shared panel controls", () => {
  it("uses the shared slider in the character panel without removing scale semantics", () => {
    const data = createMockBootstrap();
    const html = renderToStaticMarkup(<SkinPanel character={data.character} characters={data.characters} frameAssets={[{ id: "rick_default", name: "默认素材", path: null, builtIn: true, importedAt: null, shortActionKeys: ["action1"] }]} onChooseAndImportFrameAsset={vi.fn()} onImportFrameAssetFromPath={vi.fn()} onDeleteFrameAsset={vi.fn()} onCreateCharacter={vi.fn()} onSwitchCharacter={vi.fn()} onUpdateCharacter={vi.fn()} onPreviewCharacterScale={vi.fn()} onUpdateCharacterScale={vi.fn()} onDeleteCharacter={vi.fn()} onClose={vi.fn()} />);
    expect(html).toContain('class="ui-range"');
    expect(html).toContain('aria-valuetext="100%"');
    expect(html).not.toContain("accent-sky-");
    expect(html).toContain("panel-scroll ui-scroll");
  });
  it("uses native theme-selection buttons and shared action buttons", () => {
    const html = renderToStaticMarkup(<ThemePanel onClose={vi.fn()} />);
    expect(html).toContain('class="theme-choice"');
    expect(html).not.toContain('role="button"');
    expect(html).toContain("ui-button ui-button-primary");
    expect(html).toContain('aria-label="主色颜色"');
  });
  it("uses a common composer and selectable, wrapping chat bubbles", () => {
    const data = createMockBootstrap();
    const html = renderToStaticMarkup(<ChatPanel character={data.character} messages={data.messages} onClose={vi.fn()} />);
    expect(html).toContain("chat-composer");
    expect(html).toContain("ui-input ui-textarea chat-input");
    expect(html).toContain("chat-messages ui-scroll");
    expect(html).toContain('rows="2"');
    expect(html).toContain('aria-label="发送消息"');
    expect(html).toContain('aria-describedby="chat-keyboard-hint"');
    expect(html).toContain('aria-label="功能导航"');
    expect(html).toContain('class="panel-heading"');
    expect(html).not.toContain("w-full");
    expect(html).not.toContain("bg-blue-500");
  });
});

describe("compact chat composer", () => {
  it("grows from two lines to a bounded scrolling area", () => {
    expect(chatInputHeight(0)).toBe(42);
    expect(chatInputHeight(63)).toBe(63);
    expect(chatInputHeight(500)).toBe(126);
  });
  it("sends on Enter but preserves multiline input and IME confirmation", () => {
    expect(shouldSendChatOnEnter({ key: "Enter", shiftKey: false })).toBe(true);
    expect(shouldSendChatOnEnter({ key: "Enter", shiftKey: true })).toBe(false);
    expect(shouldSendChatOnEnter({ key: "Enter", shiftKey: false, isComposing: true })).toBe(false);
    expect(shouldSendChatOnEnter({ key: "Enter", shiftKey: false, keyCode: 229 })).toBe(false);
    expect(shouldSendChatOnEnter({ key: "a", shiftKey: false })).toBe(false);
  });
});
