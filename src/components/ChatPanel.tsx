import { SendHorizonal, Square } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { ChatMessage, CharacterProfile } from "../types/domain";
import { useAppStore } from "../stores/useAppStore";
import { PanelShell } from "./PanelShell";

interface ChatPanelProps {
  character: CharacterProfile | null;
  messages: ChatMessage[];
  onClose: () => void;
}

export function ChatPanel({ character, messages, onClose }: ChatPanelProps) {
  const [input, setInput] = useState("");
  const streamingContent = useAppStore((s) => s.streamingContent);
  const chatStreaming = useAppStore((s) => s.chatStreaming);
  const sendAiChat = useAppStore((s) => s.sendAiChat);
  const abortAiRequest = useAppStore((s) => s.abortAiRequest);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const isBusy = chatStreaming;
  const characterName = character?.name?.trim() || "当前人物";

  const visibleMessages = useMemo(
    () => messages.filter((m) => !character || m.characterId === character.id).slice(-20),
    [character, messages]
  );

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [visibleMessages.length, streamingContent, chatStreaming]);

  async function submit(): Promise<void> {
    if (!input.trim() || isBusy) return;
    const content = input;
    setInput("");
    await sendAiChat(content);
  }

  function handleAbort(): void {
    void abortAiRequest();
  }

  return (
    <PanelShell title="聊天" subtitle={character ? `${character.name} 的独立上下文` : "本地上下文"} onClose={onClose}>
      <div className="flex h-full flex-col gap-3">
        <div className="min-h-0 flex-1 space-y-2 overflow-y-auto pr-1">
          {visibleMessages.map((message) => (
            <div key={message.id} className={`flex ${message.role === "user" ? "justify-end" : "justify-start"}`}>
              <div
                className={`max-w-[78%] rounded-[8px] px-3 py-2 text-[12px] leading-relaxed ${
                  message.role === "user" ? "bg-blue-500 text-white" : "border border-sky-100 bg-white/82 text-slate-700"
                }`}
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
              <div className="max-w-[78%] rounded-[8px] border border-sky-100 bg-white/82 px-3 py-2 text-[12px] leading-relaxed text-slate-700">
                {streamingContent}
                <span className="ml-0.5 inline-block animate-pulse text-sky-400">|</span>
              </div>
            </div>
          ) : null}

          {/* Waiting indicator (thinking state, no chunks yet) */}
          {chatStreaming && !streamingContent ? (
            <div className="flex justify-start">
              <div className="rounded-[8px] border border-sky-100 bg-white/82 px-3 py-2 text-[12px] text-slate-400">
                <span className="inline-block animate-pulse">思考中...</span>
              </div>
            </div>
          ) : null}

          <div ref={messagesEndRef} />
        </div>

        <div className="rounded-[8px] border border-sky-100 bg-white/65 p-2">
          <textarea
            className="h-16 w-full resize-none rounded-[8px] border border-transparent bg-white/75 px-3 py-2 text-[12px] text-slate-700 outline-none focus:border-cyan-200"
            value={input}
            placeholder={`和 ${characterName} 说点什么`}
            onChange={(event) => setInput(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
                event.preventDefault();
                void submit();
              }
            }}
          />

          {isBusy ? (
            <button
              className="mt-2 flex h-8 w-full items-center justify-center gap-1.5 rounded-[8px] border border-rose-200 bg-white text-[12px] font-semibold text-rose-600 shadow-sm transition hover:border-rose-300 hover:bg-rose-50 active:scale-[0.99]"
              type="button"
              onClick={handleAbort}
            >
              <Square size={12} />
              停止生成
            </button>
          ) : (
            <button
              className="mt-2 flex h-8 w-full items-center justify-center gap-1.5 rounded-[8px] bg-blue-500 text-[12px] font-semibold text-white shadow-sm transition hover:bg-blue-600 disabled:cursor-not-allowed disabled:bg-slate-300"
              type="button"
              disabled={!input.trim()}
              onClick={() => void submit()}
            >
              <SendHorizonal size={14} />
              发送
            </button>
          )}
        </div>
      </div>
    </PanelShell>
  );
}
