import { BookOpen, CheckCircle2, ExternalLink, Eye, EyeOff, FolderOpen, Loader2, Save, TestTube, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { AI_PROTOCOL, DEEPSEEK_MODELS, connectionMode, newConnectionId, createConnectionDraft, validateConnectionDraft, type AiConnectionDraft, type AiConnectionMode } from "../config/aiProviders";
import type { AiProviderConfig, DataPaths, SaveAiProviderRequest } from "../types/domain";
import { deleteAiConnection, getAiConfigPublic, saveAiProviderConfig, setActiveAiProvider, testAiConnection } from "../storage/database/tauriClient";
import { PanelShell } from "./PanelShell";
import { deepseekHelpLinks, openDeepSeekHelpLink, type DeepSeekHelpLink } from "../services/helpLinks";
import { AiConnectionList } from "./AiConnectionList";

interface SettingsPanelProps {
  dataPaths: DataPaths | null;
  onOpenDataDir: () => Promise<string>;
  onClose: () => void;
  onOpenGuide: () => void;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : typeof error === "string" ? error : "操作失败，请重试。";
}

const inputClass = "ui-input";

export function SettingsPanel({ dataPaths, onOpenDataDir, onClose, onOpenGuide }: SettingsPanelProps) {
  const [providers, setProviders] = useState<AiProviderConfig[]>([]);
  const [selectedId, setSelectedId] = useState("deepseek");
  const mode = connectionMode(selectedId) ?? "deepseek";
  const [drafts, setDrafts] = useState<Record<string, AiConnectionDraft>>({
    deepseek: createConnectionDraft("deepseek"),
    custom: createConnectionDraft("custom"),
  });
  const [names, setNames] = useState<Record<string, string>>({ deepseek: "DeepSeek", custom: "自定义接口" });
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [loading, setLoading] = useState(true);
  const [showKey, setShowKey] = useState(false);
  const [busy, setBusy] = useState<"save" | "test" | "delete" | "activate" | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [helpError, setHelpError] = useState<string | null>(null);
  const draft = drafts[selectedId] ?? createConnectionDraft(mode);
  const savedProvider = providers.find((provider) => provider.providerId === selectedId);
  const savedConnections = providers.filter(provider => connectionMode(provider.providerId) && provider.providerType === AI_PROTOCOL);
  const active = providers.find((provider) => provider.isActive);
  const legacyNative = providers.some((provider) => provider.providerType !== AI_PROTOCOL);

  useEffect(() => {
    let disposed = false;
    void getAiConfigPublic().then((configs) => {
      if (disposed) return;
      setProviders(configs);
      const editable = configs.filter(item => connectionMode(item.providerId) && item.providerType === AI_PROTOCOL);
      setDrafts({
        deepseek: createConnectionDraft("deepseek", configs.find((item) => item.providerId === "deepseek")),
        custom: createConnectionDraft("custom", configs.find((item) => item.providerId === "custom")),
        ...Object.fromEntries(editable.map(item => [item.providerId, createConnectionDraft(connectionMode(item.providerId)!, item)])),
      });
      setNames({ deepseek: "DeepSeek", custom: "自定义接口", ...Object.fromEntries(editable.map(item => [item.providerId, item.displayName])) });
      setSelectedId(editable.find(item => item.isActive)?.providerId ?? "deepseek");
    }).catch((error: unknown) => {
      if (!disposed) setStatus(errorMessage(error));
    }).finally(() => { if (!disposed) setLoading(false); });
    return () => { disposed = true; };
  }, []);

  function updateDraft(update: Partial<AiConnectionDraft>): void {
    setDrafts((current) => ({ ...current, [selectedId]: { ...(current[selectedId] ?? createConnectionDraft(mode)), ...update } }));
    setStatus(null);
  }

  function changeMode(next: AiConnectionMode): void {
    selectConnection(next);
  }

  function selectConnection(id: string): void {
    setSelectedId(id);
    setShowKey(false);
    setStatus(null);
    setHelpError(null);
    setConfirmDelete(false);
  }

  function addConnection(): void {
    const id = newConnectionId(mode);
    setDrafts(current => ({ ...current, [id]: createConnectionDraft(mode) }));
    setNames(current => ({ ...current, [id]: `${mode === "deepseek" ? "DeepSeek" : "自定义接口"} ${Object.keys(current).filter(key => connectionMode(key) === mode).length + 1}` }));
    selectConnection(id);
  }

  function buildRequest(): SaveAiProviderRequest {
    const validation = validateConnectionDraft(mode, draft, savedProvider?.hasApiKey ?? false);
    if (validation) throw new Error(validation);
    const displayName = (names[selectedId] ?? "").trim();
    if (!displayName || displayName.length > 40) throw new Error("请填写 1 至 40 个字符的配置名称。");
    const request: SaveAiProviderRequest = {
      providerId: selectedId, providerType: AI_PROTOCOL, displayName,
      baseUrl: draft.baseUrl.trim().replace(/\/+$/, ""), model: draft.model.trim(),
      stream: true, temperature: 0.8, maxOutputTokens: 1200, timeoutMs: 60000,
    };
    if (draft.apiKey.trim()) request.apiKey = draft.apiKey.trim();
    return request;
  }

  async function persist(activate: boolean): Promise<void> {
    setBusy("save");
    setStatus(null);
    try {
      const request = buildRequest();
      const saved = await saveAiProviderConfig(request);
      setProviders(current => [...current.filter(item => item.providerId !== saved.providerId), saved]);
      setDrafts((current) => ({ ...current, [selectedId]: { ...draft, apiKey: "" } }));
      if (activate) await setActiveAiProvider(selectedId);
      setProviders(await getAiConfigPublic());
      setStatus(`${request.displayName} 已保存${activate ? "并启用" : "；未切换当前连接"}。`);
    } catch (error) {
      setStatus(errorMessage(error));
    } finally {
      setBusy(null);
    }
  }

  async function testDraft(): Promise<void> {
    setBusy("test");
    setStatus(null);
    try {
      const result = await testAiConnection(buildRequest());
      setStatus(`连接测试通过，当前草稿尚未保存。${result ? `\n${result}` : ""}`);
    } catch (error) { setStatus(errorMessage(error)); }
    finally { setBusy(null); }
  }

  async function activateSaved(id: string): Promise<void> {
    setBusy("activate");
    setStatus(null);
    try {
      await setActiveAiProvider(id);
      setProviders(await getAiConfigPublic());
      setConfirmDelete(false);
      setStatus("已启用所选配置，将用于后续聊天；未保存的草稿没有被提交。");
    } catch (error) { setStatus(errorMessage(error)); }
    finally { setBusy(null); }
  }

  async function deleteSelected(): Promise<void> {
    if (!confirmDelete) { setConfirmDelete(true); return; }
    setBusy("delete");
    try {
      await deleteAiConnection(selectedId);
      const configs = await getAiConfigPublic();
      setProviders(configs);
      setDrafts(current => { const next = { ...current }; delete next[selectedId]; return next; });
      setNames(current => { const next = { ...current }; delete next[selectedId]; return next; });
      selectConnection(configs.find(item => item.isActive && connectionMode(item.providerId))?.providerId ?? "deepseek");
      setStatus("已删除此配置及本地密钥。平台上的密钥不会被撤销。");
    } catch (error) { setStatus(errorMessage(error)); }
    finally { setBusy(null); }
  }

  async function openHelpLink(id: DeepSeekHelpLink): Promise<void> {
    setHelpError(null);
    try { await openDeepSeekHelpLink(id); }
    catch (error) { setHelpError(`${errorMessage(error)}\n可手动访问：${deepseekHelpLinks[id].url}`); }
  }

  return (
    <PanelShell title="设置" subtitle="聊天连接与本地数据" preferredSize={{ width: 380, height: 560 }} className="settings-shell" onClose={onClose}>
      <div className="settings-layout">
        <div className="settings-scroll ui-scroll">
          <fieldset disabled={loading || busy !== null} className="settings-fields">
            <section className="ui-card">
              <div className="ui-section-heading">
                <h3>聊天配置</h3>
                <button type="button" className="ui-link" onClick={addConnection}>＋ 新增配置</button>
              </div>
              {active ? <p className="settings-current"><CheckCircle2 size={13} /><span>正在使用 · {active.displayName}</span></p> : null}
              <AiConnectionList providers={savedConnections} selectedId={selectedId} onSelect={selectConnection} onActivate={id => void activateSaved(id)} />
              <p className="ui-hint">点击配置编辑；一次只启用一个。</p>
            </section>

            <section className="ui-card settings-editor">
              <div className="ui-section-heading"><h3>{mode === "deepseek" ? "DeepSeek" : "自定义接口"}</h3><span className="ui-badge">{savedProvider ? "编辑配置" : "新配置"}</span></div>
              <label className="ui-field">配置名称
                <input aria-label="聊天配置名称" className={inputClass} maxLength={40} value={names[selectedId] ?? (mode === "deepseek" ? "DeepSeek" : "自定义接口")} onChange={event => { setNames(current => ({ ...current, [selectedId]: event.target.value })); setStatus(null); }} placeholder="例如：日常使用" />
              </label>
              {mode === "deepseek" ? <label className="ui-field">聊天模型
                <select aria-label="DeepSeek 聊天模型" className={inputClass} value={draft.model} onChange={event => updateDraft({ model: event.target.value })}>
                  {DEEPSEEK_MODELS.map(model => <option key={model.id} value={model.id}>{model.label}</option>)}
                </select>
              </label> : null}
              <label className="ui-field">API Key{mode === "custom" ? <span className="ui-field-note">免鉴权接口可留空</span> : null}
                <div className="settings-key-input">
                  <input aria-label={`${mode === "deepseek" ? "DeepSeek" : "自定义接口"} API Key`} className={inputClass} type={showKey ? "text" : "password"} autoComplete="off" value={draft.apiKey} placeholder={loading ? "读取配置中…" : savedProvider?.hasApiKey ? "已保存，留空保留原密钥" : "粘贴你的 API Key"} onChange={event => updateDraft({ apiKey: event.target.value })} />
                  <button className="ui-icon-button" type="button" aria-label={showKey ? "隐藏密钥" : "显示密钥"} onClick={() => setShowKey(value => !value)}>{showKey ? <EyeOff size={16} /> : <Eye size={16} />}</button>
                </div>
                {savedProvider?.hasApiKey ? <span className="ui-hint">{savedProvider.apiKeyHint ?? "密钥已保存"} · 更换时再填入新密钥</span> : null}
              </label>
              <div className="settings-help-links">
                {mode === "deepseek" ? <button type="button" aria-label="获取 DeepSeek API Key" className="ui-link" onClick={() => void openHelpLink("keys")}>获取 API Key<ExternalLink size={12} /></button> : null}
                <button aria-label="DeepSeek 新手指南" className="ui-link" type="button" onClick={() => { setShowKey(false); onOpenGuide(); }}><BookOpen size={13} />查看独立指南</button>
              </div>
              <details data-testid="connection-advanced" className="ui-details" open={mode === "custom" || undefined}>
                <summary>高级设置</summary>
                <div className="ui-details-body">
                  <label className="ui-check"><input type="checkbox" checked={mode === "custom"} onChange={event => changeMode(event.target.checked ? "custom" : "deepseek")} />使用自定义接口</label>
                  {mode === "custom" ? <>
                    <label className="ui-field">接口地址（Base URL）<input className={inputClass} value={draft.baseUrl} placeholder="https://api.example.com/v1" onChange={event => updateDraft({ baseUrl: event.target.value })} /></label>
                    <p className="ui-hint">自动追加 /chat/completions，也接受完整地址。</p>
                    <label className="ui-field">模型名称<input className={inputClass} value={draft.model} placeholder="服务商提供的模型 ID" onChange={event => updateDraft({ model: event.target.value })} /></label>
                    <p className="ui-hint">DIY 功能：仅支持 OpenAI 兼容 Chat Completions。项目主要维护 DeepSeek，不逐家适配其他服务，欢迎反馈。</p>
                    <button type="button" className="ui-link" onClick={() => changeMode("deepseek")}>新手建议：切回 DeepSeek</button>
                  </> : <p className="ui-hint">其他服务可自行配置兼容接口。DeepSeek 模型直接在上方选择。</p>}
                  {legacyNative ? <p className="ui-hint">旧版原生接口配置已保留但停用，请改用兼容接口。</p> : null}
                </div>
              </details>
            </section>
          </fieldset>
          {helpError ? <div role="alert" className="ui-notice">{helpError}</div> : null}
          <details className="ui-card ui-details settings-data">
            <summary><FolderOpen size={14} />本地数据</summary>
            <div className="ui-details-body"><p className="ui-hint">角色和聊天记录保存在本机，密钥保存在系统凭据库。</p>
              <p className="settings-data-path">{dataPaths?.root ?? "正在读取…"}</p>
              <button className="ui-button ui-button-secondary" type="button" onClick={() => void onOpenDataDir().catch((error: unknown) => setStatus(errorMessage(error)))}>打开数据目录</button>
            </div>
          </details>
        </div>
        <div className="settings-footer">
          {status ? <div role="status" className="ui-notice settings-status">{status}</div> : null}
          {confirmDelete ? <div role="alert" className="ui-notice ui-notice-danger">确定删除「{savedProvider?.displayName}」及本地密钥？再次点击「确认删除」完成操作。<button className="ui-link" type="button" onClick={() => setConfirmDelete(false)}>取消</button></div> : null}
          <fieldset disabled={loading || busy !== null} className="settings-action-grid" aria-label="配置操作">
            <button className="ui-button ui-button-primary" type="button" onClick={() => void persist(true)}>{busy === "save" ? <Loader2 size={15} className="animate-spin" /> : <Save size={15} />}保存并使用</button>
            <button className="ui-button ui-button-secondary" type="button" onClick={() => void testDraft()}>{busy === "test" ? <Loader2 size={15} className="animate-spin" /> : <TestTube size={15} />}测试连接</button>
            <button className="ui-button ui-button-quiet" type="button" onClick={() => void persist(false)}>仅保存，不切换</button>
            <button className="ui-button ui-button-quiet ui-button-danger" type="button" title={savedProvider?.isActive ? "请先启用另一个配置再删除" : "删除此配置及本地密钥"} disabled={!savedProvider || savedProvider.isActive} onClick={() => void deleteSelected()}><Trash2 size={14} />{confirmDelete ? "确认删除" : "删除配置"}</button>
          </fieldset>
          <p className="ui-hint settings-cost-note">测试可能产生少量 API 费用，且不会保存配置。</p>
        </div>
      </div>
    </PanelShell>
  );
}
