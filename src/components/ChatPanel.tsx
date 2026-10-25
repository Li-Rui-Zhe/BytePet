import { SendHorizonal, Square } from "lucide-react";
import { useEffect, useLayoutEffect, useMemo, useRef } from "react";
import type { ChatMessage, CharacterProfile } from "../types/domain";
import { useAppStore } from "../stores/useAppStore";
import { PanelShell } from "./PanelShell";
import { chatInputHeight, shouldSendChatOnEnter } from "../utils/chatComposer";

interface ChatPanelProps {
  character: CharacterProfile | null;
  messages: ChatMessage[];
  onClose: () => void;
}

export function ChatPanel({ character, messages, onClose }: ChatPanelProps) {
  const input = useAppStore((s) => s.chatDrafts[character?.id ?? ""] ?? "");
  const setChatDraft = useAppStore((s) => s.setChatDraft);
  const pendingChatMessage = useAppStore((s) => s.pendingChatMessage);
  const cancelPendingChat = useAppStore((s) => s.cancelPendingChat);
  const streamingContent = useAppStore((s) => s.streamingContent);
  const chatStopping = useAppStore((s) => s.chatStopping);
  const chatStreaming = useAppStore((s) => s.chatStreaming);
  const sendAiChat = useAppStore((s) => s.sendAiChat);
  const abortAiRequest = useAppStore((s) => s.abortAiRequest);
  const error = useAppStore((s) => s.error);
  const openPanel = useAppStore((s) => s.openPanel);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const isBusy = chatStreaming;
  const characterName = character?.name?.trim() || "当前人物";

  const visibleMessages = useMemo(
    () => messages.filter((m) => !character || m.characterId === character.id).slice(-20),
    [character, messages]
  );

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [visibleMessages.length, streamingContent, chatStreaming]);

  useLayoutEffect(() => {
    const textarea = inputRef.current;
    if (!textarea) return;
    const fitInput = () => {
      textarea.style.height = "0px";
      textarea.style.height = `${chatInputHeight(textarea.scrollHeight)}px`;
    };
    fitInput();
    // Reflow wrapped drafts when the user resizes the panel.
    let previousWidth = textarea.clientWidth;
    const observer = new ResizeObserver(() => {
      if (textarea.clientWidth === previousWidth) return;
      previousWidth = textarea.clientWidth;
      fitInput();
    });
    observer.observe(textarea);
    return () => observer.disconnect();
  }, [input, character?.id]);

  async function submit(): Promise<void> {
    if (!input.trim() || isBusy || pendingChatMessage) return;
    await sendAiChat(input);
  }

  function handleAbort(): void {
    void abortAiRequest().catch(() => {});
  }

  return (
    <PanelShell title="聊天" subtitle={characterName} preferredSize={{ width: 380, height: 480 }} className="chat-shell" onClose={onClose}>
      <div className="chat-layout">
        {error ? <div role="alert" className="ui-notice ui-notice-danger chat-notice">
          <p className="whitespace-pre-wrap break-words">{error}</p>
          <button className="ui-link" type="button" onClick={() => openPanel("settings")}>打开聊天连接设置</button>
        </div> : null}
        {pendingChatMessage ? <div role="status" className="ui-notice chat-notice">
          <p>等待角色落地、恢复后发送。消息仍保留在输入框中。</p>
          <button type="button" className="ui-link" onClick={cancelPendingChat}>取消等待，继续编辑</button>
        </div> : null}
        <div className="chat-messages ui-scroll" role="log" aria-label="聊天消息">
          {!visibleMessages.length && !chatStreaming ? <div className="ui-empty chat-empty"><p>想说点什么？</p><span>和 {characterName} 打个招呼吧。</span></div> : null}
          {visibleMessages.map((message) => (
            <div key={message.id} className={`flex ${message.role === "user" ? "justify-end" : "justify-start"}`}>
              <div
                className={`chat-bubble ${message.role === "user" ? "chat-bubble-user" : "chat-bubble-assistant"}`}
              >
                {message.content}
                {message.metadataJson?.includes("interrupted") ? (
                  <span className="mt-1 block text-[10px] font-semibold text-amber-500">已中断</span>
                ) : null}
              </div>
            </div>
          ))}

          {/* Streaming message */}
          {chatStreaming && streamingContent ? (
            <div className="flex justify-start">
              <div className="chat-bubble chat-bubble-assistant">
                {streamingContent}
                <span className="ml-0.5 inline-block animate-pulse text-sky-400">|</span>
              </div>
            </div>
          ) : null}

          {/* Waiting indicator (thinking state, no chunks yet) */}
          {chatStreaming && !streamingContent ? (
            <div className="flex justify-start">
              <div className="chat-bubble chat-bubble-assistant">
                <span className="inline-block animate-pulse">{chatStopping ? "正在停止…" : "思考中…"}</span>
              </div>
            </div>
          ) : null}

          <div ref={messagesEndRef} />
        </div>

        <div className="chat-composer">
          <textarea
            ref={inputRef}
            rows={2}
            className="ui-input ui-textarea chat-input"
            aria-label="聊天输入"
            aria-describedby="chat-keyboard-hint"
            value={input}
            placeholder={`和 ${characterName} 说点什么`}
            disabled={Boolean(pendingChatMessage)}
            onChange={(event) => { if (character) setChatDraft(character.id, event.target.value); }}
            onKeyDown={(event) => {
              if (shouldSendChatOnEnter({ key: event.key, shiftKey: event.shiftKey, isComposing: event.nativeEvent.isComposing, keyCode: event.nativeEvent.keyCode })) {
                event.preventDefault();
                void submit();
              }
            }}
          />

          <div className="chat-composer-toolbar">
            <span id="chat-keyboard-hint" className="chat-keyboard-hint">Enter 发送<span> · Shift + Enter 换行</span></span>
            {isBusy ? (
              <button
                className="ui-button ui-button-secondary ui-button-danger chat-send"
                type="button"
                aria-label={chatStopping ? "正在停止生成" : "停止生成"}
                title={chatStopping ? "正在停止…" : "停止生成"}
                disabled={chatStopping}
                onClick={handleAbort}
              >
                <Square size={12} />
              </button>
            ) : (
              <button
                className="ui-button ui-button-primary chat-send"
                type="button"
                aria-label="发送消息"
                title="发送消息（Enter）"
                disabled={!input.trim() || Boolean(pendingChatMessage) || !character}
                onClick={() => void submit()}
              >
                <SendHorizonal size={16} />
              </button>
            )}
          </div>
        </div>
      </div>
    </PanelShell>
  );
}
