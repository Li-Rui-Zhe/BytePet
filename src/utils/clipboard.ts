/** Resolve only after the browser confirms that text reached the clipboard. */
export async function copyText(text: string): Promise<void> {
  if (typeof navigator === "undefined" || !navigator.clipboard?.writeText) {
    throw new Error("当前环境无法访问剪贴板，请手动选择文字复制。");
  }
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    throw new Error("复制失败，请重试，或手动选择文字复制。");
  }
}
