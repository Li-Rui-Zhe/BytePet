export const CHAT_INPUT_MIN_HEIGHT = 42;
export const CHAT_INPUT_MAX_HEIGHT = 126;

export function chatInputHeight(scrollHeight: number): number {
  return Math.max(CHAT_INPUT_MIN_HEIGHT, Math.min(CHAT_INPUT_MAX_HEIGHT, scrollHeight));
}

export function shouldSendChatOnEnter(event: {
  key: string;
  shiftKey: boolean;
  isComposing?: boolean;
  keyCode?: number;
}): boolean {
  // Some Windows IMEs report keyCode 229 even when isComposing is false.
  return event.key === "Enter" && !event.shiftKey && !event.isComposing && event.keyCode !== 229;
}
