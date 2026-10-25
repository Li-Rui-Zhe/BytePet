import { CalendarDays, ChevronLeft, Copy, MessageCircle, Pencil, Search, Trash2, UserRound } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { CharacterProfile, ChatHistoryDay, ChatMessage, ChatSession } from "../types/domain";
import { chatHistoryDays, chatHistoryMessagesForDay } from "../storage/database/tauriClient";
import { PanelShell } from "./PanelShell";
import { copyText } from "../utils/clipboard";

interface MemoryPanelProps {
  character: CharacterProfile | null;
  characters: CharacterProfile[];
  sessions: ChatSession[];
  messages: ChatMessage[];
  onDeleteHistoryDay: (dateKey: string) => Promise<void>;
  onSwitchCharacter: (characterId: string) => Promise<void>;
  onManageCharacters: () => void;
  onClose: () => void;
}

function parseDate(value: string): Date {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? new Date(0) : date;
}

function formatDateKey(value: string): string {
  const date = parseDate(value);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function formatDateLabel(key: string): string {
  const today = formatDateKey(new Date().toISOString());
  const yesterdayDate = new Date();
  yesterdayDate.setDate(yesterdayDate.getDate() - 1);
  const yesterday = formatDateKey(yesterdayDate.toISOString());
  if (key === today) return "今天";
  if (key === yesterday) return "昨天";
  return key;
}

function formatTime(value: string): string {
  const date = parseDate(value);
  return date.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function roleLabel(role: string): string {
  if (role === "assistant") return "AI";
  if (role === "user") return "你";
  return role;
}

export function MemoryPanel({
  character,
  characters,
  sessions,
  messages,
  onDeleteHistoryDay,
  onSwitchCharacter,
  onManageCharacters,
  onClose,
}: MemoryPanelProps) {
  const [tab, setTab] = useState<"history" | "profiles">("history");
  const [selectedCharacterId, setSelectedCharacterId] = useState<string | null>(null);
  const [selectedDayKey, setSelectedDayKey] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [copiedMessageId, setCopiedMessageId] = useState<string | null>(null);
  const [historyDays, setHistoryDays] = useState<ChatHistoryDay[]>([]);
  const [dayMessages, setDayMessages] = useState<ChatMessage[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [pendingDeleteDayKey, setPendingDeleteDayKey] = useState<string | null>(null);

  const profileCharacter = selectedCharacterId
    ? characters.find((item) => item.id === selectedCharacterId) ?? character
    : character;

  const selectedDay = useMemo(
    () => historyDays.find((day) => day.dateKey === selectedDayKey) ?? null,
    [historyDays, selectedDayKey]
  );

  const filteredMessages = useMemo(() => {
    const keyword = query.trim().toLowerCase();
    if (!keyword) return dayMessages;
    return dayMessages.filter((message) => `${roleLabel(message.role)} ${message.content}`.toLowerCase().includes(keyword));
  }, [query, dayMessages]);

  useEffect(() => {
    if (tab !== "history" || !character) return;
    let disposed = false;

    async function loadDays(): Promise<void> {
      setHistoryLoading(true);
      setHistoryError(null);
      try {
        const days = await chatHistoryDays();
        if (!disposed) setHistoryDays(days);
      } catch (error) {
        if (!disposed) setHistoryError(error instanceof Error ? error.message : "历史聊天加载失败");
      } finally {
        if (!disposed) setHistoryLoading(false);
      }
    }

    void loadDays();
    return () => {
      disposed = true;
    };
  }, [tab, character?.id, messages.length]);

  useEffect(() => {
    if (!selectedDayKey) {
      setDayMessages([]);
      return;
    }

    const dateKey = selectedDayKey;
    let disposed = false;
    async function loadMessages(): Promise<void> {
      setHistoryLoading(true);
      setHistoryError(null);
      try {
        const items = await chatHistoryMessagesForDay(dateKey);
        if (!disposed) setDayMessages(items);
      } catch (error) {
        if (!disposed) setHistoryError(error instanceof Error ? error.message : "当天聊天加载失败");
      } finally {
        if (!disposed) setHistoryLoading(false);
      }
    }

    void loadMessages();
    return () => {
      disposed = true;
    };
  }, [selectedDayKey, character?.id, messages.length]);

  const handleDeleteDay = async (day: ChatHistoryDay): Promise<void> => {
    if (pendingDeleteDayKey !== day.dateKey) {
      setPendingDeleteDayKey(day.dateKey);
      return;
    }

    setHistoryLoading(true);
    setHistoryError(null);
    try {
      await onDeleteHistoryDay(day.dateKey);
      const days = await chatHistoryDays();
      setHistoryDays(days);
      if (selectedDayKey === day.dateKey) {
        setSelectedDayKey(null);
        setDayMessages([]);
        setQuery("");
      }
      setPendingDeleteDayKey(null);
    } catch (error) {
      setHistoryError(error instanceof Error ? error.message : "删除失败");
    } finally {
      setHistoryLoading(false);
    }
  };

  const handleCopyMessage = async (message: ChatMessage): Promise<void> => {
    setCopiedMessageId(null);
    setHistoryError(null);
    try {
      await copyText(message.content);
      setCopiedMessageId(message.id);
      window.setTimeout(() => setCopiedMessageId((current) => (current === message.id ? null : current)), 1200);
    } catch (error) {
      setHistoryError(error instanceof Error ? error.message : "复制失败，请手动选择文字复制。");
    }
  };

  return (
    <PanelShell title="记忆" subtitle={character ? `${character.name} 的独立记忆` : "人物存档"} preferredSize={{ width: 380, height: 540 }} className="memory-shell" onClose={onClose}>
      <div className="memory-layout">
        <div className="ui-tabs" aria-label="记忆视图">
          <button data-active={tab === "history"} aria-pressed={tab === "history"} type="button" onClick={() => setTab("history")}>
            历史聊天
          </button>
          <button data-active={tab === "profiles"} aria-pressed={tab === "profiles"} type="button" onClick={() => setTab("profiles")}>
            人物存档
          </button>
        </div>

        {tab === "history" ? (
          <div className="memory-history ui-scroll">
            {historyError ? <p role="alert" className="ui-notice ui-notice-danger mb-2">{historyError}</p> : null}
            {selectedDay ? (
              <div className="flex min-h-0 flex-col gap-2">
                <div className="flex items-center gap-2">
                  <button className="ui-icon-button ui-icon-button-small" type="button" title="返回" aria-label="返回日期列表" onClick={() => setSelectedDayKey(null)}>
                    <ChevronLeft size={15} />
                  </button>
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-[12px] font-semibold text-slate-700">{formatDateLabel(selectedDay.dateKey)}</p>
                    <p className="text-[10px] text-slate-400">{dayMessages.length || selectedDay.messageCount} 条消息 · {character?.name ?? "当前人物"}</p>
                  </div>
                  <button className="ui-icon-button ui-icon-button-small ui-button-danger" type="button" title="删除" aria-label="删除当天聊天" onClick={() => void handleDeleteDay(selectedDay)}>
                    <Trash2 size={14} />
                  </button>
                </div>

                {pendingDeleteDayKey === selectedDay.dateKey ? (
                  <div className="ui-notice ui-notice-danger">
                    <p className="text-[11px] leading-relaxed text-rose-600">确认删除 {formatDateLabel(selectedDay.dateKey)} 的聊天记录？删除后无法恢复。</p>
                    <div className="mt-2 grid grid-cols-2 gap-2">
                      <button className="ui-button ui-button-secondary ui-button-danger" type="button" onClick={() => void handleDeleteDay(selectedDay)}>
                        确认删除
                      </button>
                      <button className="ui-button ui-button-secondary" type="button" onClick={() => setPendingDeleteDayKey(null)}>
                        取消
                      </button>
                    </div>
                  </div>
                ) : null}

                <label className="ui-search">
                  <Search size={14} className="shrink-0 text-sky-500" />
                  <input
                    className="ui-input"
                    aria-label="查找当天聊天"
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                    placeholder="查找当天聊天"
                  />
                </label>

                <div className="space-y-2">
                  {historyLoading && !dayMessages.length ? <p className="ui-empty">正在加载...</p> : null}
                  {filteredMessages.length ? filteredMessages.map((message) => (
                    <div key={message.id} className="memory-message">
                      <div className="mb-1 flex items-center gap-2">
                        <span className="font-semibold text-sky-700">{roleLabel(message.role)}</span>
                        <span className="text-[10px] text-slate-400">{formatTime(message.createdAt)}</span>
                        <button className="ui-button ui-button-quiet ui-button-small ml-auto" type="button" onClick={() => void handleCopyMessage(message)}>
                          <Copy size={11} />
                          {copiedMessageId === message.id ? "已复制" : "复制"}
                        </button>
                      </div>
                      <p className="select-text whitespace-pre-wrap break-words leading-relaxed">{message.content}</p>
                    </div>
                  )) : !historyLoading ? <p className="ui-empty">没有匹配的聊天内容。</p> : null}
                </div>
              </div>
            ) : (
              <div className="space-y-2">
                {historyLoading && !historyDays.length ? <p className="ui-empty">正在加载历史聊天...</p> : null}
                {historyDays.length ? historyDays.map((day) => (
                  <div key={day.dateKey} className="ui-card memory-day">
                    <div className="flex items-start gap-2">
                      <button
                        className="min-w-0 flex flex-1 items-start gap-2 text-left"
                        type="button"
                        onClick={() => {
                          setSelectedDayKey(day.dateKey);
                          setPendingDeleteDayKey(null);
                          setQuery("");
                        }}
                      >
                        <MessageCircle size={14} className="mt-0.5 shrink-0 text-sky-500" />
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <CalendarDays size={13} className="shrink-0 text-violet-500" />
                            <p className="truncate text-[12px] font-semibold text-slate-700">{formatDateLabel(day.dateKey)}</p>
                            <span className="ml-auto shrink-0 text-[10px] text-slate-400">{day.messageCount} 条</span>
                          </div>
                          <p className="mt-1 line-clamp-2 text-[11px] leading-relaxed text-slate-500">{day.preview || "暂无聊天内容"}</p>
                        </div>
                      </button>
                      <button className="ui-icon-button ui-icon-button-small ui-button-danger" type="button" title="删除" aria-label={`删除 ${day.dateKey} 的聊天`} onClick={() => void handleDeleteDay(day)}>
                        <Trash2 size={13} />
                      </button>
                    </div>
                    {pendingDeleteDayKey === day.dateKey ? (
                      <div className="ui-notice ui-notice-danger mt-2">
                        <p className="text-[11px] leading-relaxed text-rose-600">确认删除 {formatDateLabel(day.dateKey)} 的聊天记录？删除后无法恢复。</p>
                        <div className="mt-2 grid grid-cols-2 gap-2">
                          <button className="ui-button ui-button-secondary ui-button-danger" type="button" onClick={() => void handleDeleteDay(day)}>
                            确认删除
                          </button>
                          <button className="ui-button ui-button-secondary" type="button" onClick={() => setPendingDeleteDayKey(null)}>
                            取消
                          </button>
                        </div>
                      </div>
                    ) : null}
                  </div>
                )) : !historyLoading ? <p className="ui-empty">这个人物还没有聊天记录。</p> : null}
              </div>
            )}
          </div>
        ) : (
          <div className="memory-history ui-scroll">
            <div className="space-y-2">
              {characters.map((profile) => {
                const isLoaded = character?.id === profile.id;
                const count = isLoaded ? messages.filter((message) => message.characterId === profile.id).length : 0;
                const profileSessions = isLoaded ? sessions.filter((session) => session.characterId === profile.id).length : 0;
                const selected = profileCharacter?.id === profile.id;
                return (
                  <article
                    key={profile.id}
                    className="ui-card memory-profile"
                    data-selected={selected}
                  >
                    <div className="flex items-start justify-between gap-2">
                      <button type="button" className="memory-profile-select" onClick={() => setSelectedCharacterId(profile.id)} aria-pressed={selected}>
                        <div className="flex items-center gap-1.5 text-[12px] font-semibold text-slate-800">
                          <UserRound size={14} />
                          <span className="truncate">{profile.name}</span>
                          {profile.isActive ? <span className="ui-status">当前</span> : null}
                        </div>
                        <p className="mt-1 line-clamp-2 text-[11px] leading-relaxed text-slate-500">
                          {profile.memorySummary || "暂无记忆摘要"}
                        </p>
                        <p className="mt-1 text-[10px] text-slate-400">
                          {isLoaded ? `${profileSessions} 个会话 · 最近 ${count} 条消息` : "切换后查看聊天记录"}
                        </p>
                      </button>
                      <div className="flex shrink-0 gap-1">
                        <button
                          type="button"
                          className="ui-icon-button ui-icon-button-small"
                          title="去人物页编辑"
                          aria-label={`管理 ${profile.name}`}
                          onClick={onManageCharacters}
                        >
                          <Pencil size={13} />
                        </button>
                        <button
                          type="button"
                          className="ui-button ui-button-secondary ui-button-small"
                          disabled={profile.isActive}
                          onClick={() => void onSwitchCharacter(profile.id)}
                        >
                          切换
                        </button>
                      </div>
                    </div>
                  </article>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </PanelShell>
  );
}
