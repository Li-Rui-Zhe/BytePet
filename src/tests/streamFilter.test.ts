import { describe, expect, it } from "vitest";
import { partitionStreamBuffer } from "../systems/chat/streamFilter";

describe("stream buffer filtering", () => {
  it("shows short replies immediately", () => {
    expect(partitionStreamBuffer("你好，我在这里。")).toEqual({
      visible: "你好，我在这里。",
      pending: "",
    });
  });

  it("keeps a partial action marker hidden between chunks", () => {
    const first = partitionStreamBuffer("我会陪着你。[[PET_");
    expect(first).toEqual({ visible: "我会陪着你。", pending: "[[PET_" });

    const second = partitionStreamBuffer(first.pending + "ACTION:comfort]]");
    expect(second.visible).toBe("");
    expect(second.pending).toBe("[[PET_ACTION:comfort]]");
  });

  it("removes complete tool calls while preserving surrounding text", () => {
    expect(
      partitionStreamBuffer('先别着急。<tool_call>{"name":"x"}</tool_call>我们慢慢来。')
    ).toEqual({
      visible: "先别着急。我们慢慢来。",
      pending: "",
    });
  });
});
