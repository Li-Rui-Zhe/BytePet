import { afterEach, describe, expect, it, vi } from "vitest";
import { copyText } from "../utils/clipboard";

afterEach(() => vi.unstubAllGlobals());

describe("copying chat messages", () => {
  it("waits for successful clipboard writing", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    await expect(copyText("hello")).resolves.toBeUndefined();
    expect(writeText).toHaveBeenCalledExactlyOnceWith("hello");
  });

  it("reports unavailable clipboard access instead of false success", async () => {
    vi.stubGlobal("navigator", {});
    await expect(copyText("hello")).rejects.toThrow("手动选择文字复制");
    vi.stubGlobal("navigator", undefined);
    await expect(copyText("hello")).rejects.toThrow("无法访问剪贴板");
  });

  it("turns permission failures into a readable retry message", async () => {
    vi.stubGlobal("navigator", { clipboard: { writeText: vi.fn().mockRejectedValue(new Error("NotAllowedError")) } });
    await expect(copyText("hello")).rejects.toThrow("复制失败，请重试");
  });
});
