import { CheckCircle2 } from "lucide-react";
import type { AiProviderConfig } from "../types/domain";

interface Props {
  providers: AiProviderConfig[];
  selectedId: string;
  onSelect: (id: string) => void;
  onActivate: (id: string) => void;
}

export function AiConnectionList({ providers, selectedId, onSelect, onActivate }: Props) {
  return <div className="connection-list" aria-label="已保存的聊天配置">
    {providers.map(provider => <div key={provider.providerId} className="connection-row" data-selected={selectedId === provider.providerId}>
      <button type="button" className="connection-choice" aria-label={`编辑 ${provider.displayName}`} aria-pressed={selectedId === provider.providerId} onClick={() => onSelect(provider.providerId)}>
        <span className="connection-name" title={provider.displayName}>{provider.displayName}</span>
        <span className="connection-meta"><span className="connection-model">{provider.model}</span><span className="connection-key">{provider.apiKeyHint ?? (provider.hasApiKey ? "密钥已保存" : "未配置密钥")}</span></span>
      </button>
      {provider.isActive ? <span className="connection-active"><CheckCircle2 size={12} aria-hidden="true" />使用中</span>
        : <button type="button" className="ui-button ui-button-secondary ui-button-small" aria-label={`启用 ${provider.displayName}`} onClick={() => onActivate(provider.providerId)}>启用</button>}
    </div>)}
    {!providers.length ? <p className="ui-hint">尚未保存配置，填写下方信息即可开始。</p> : null}
  </div>;
}
