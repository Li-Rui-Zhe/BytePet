import { useEffect, useRef, useState } from "react";
import { DeepSeekGuide } from "./DeepSeekGuide";
import { PanelShell } from "./PanelShell";
import { deepseekHelpLinks, openDeepSeekHelpLink, type DeepSeekHelpLink } from "../services/helpLinks";

export type HelpTopic = "start" | "deepseek" | "troubleshooting";
interface Props { onClose: () => void; onSettings: () => void; onWelcome: () => void; initialTopic?: HelpTopic }
const TOPICS: { id: HelpTopic; label: string }[] = [
  { id: "start", label: "先开始" }, { id: "deepseek", label: "连接 AI" }, { id: "troubleshooting", label: "问题排查" },
];

export function HelpPanel({ onClose, onSettings, onWelcome, initialTopic = "start" }: Props) {
  const [topic, setTopic] = useState<HelpTopic>(initialTopic);
  const [error, setError] = useState<string | null>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  useEffect(() => { if (bodyRef.current) bodyRef.current.scrollTop = 0; setError(null); }, [topic]);

  async function openLink(id: DeepSeekHelpLink): Promise<void> {
    setError(null);
    try { await openDeepSeekHelpLink(id); }
    catch { setError(`页面打开失败，可手动访问：${deepseekHelpLinks[id].url}`); }
  }
  return <PanelShell title="使用指南" subtitle="离线可读 · 按需查看" preferredSize={{ width: 400, height: 580 }} className="help-shell" onClose={onClose}>
    <div className="help-layout">
      <div className="ui-tabs help-tabs" aria-label="指南分类">
        {TOPICS.map(item => <button key={item.id} type="button" data-active={topic === item.id} aria-pressed={topic === item.id} onClick={() => setTopic(item.id)}>{item.label}</button>)}
      </div>
      {error ? <p role="alert" className="ui-notice ui-notice-danger help-error">{error}</p> : null}
      <div ref={bodyRef} className="help-body ui-scroll">
        {topic === "start" ? <article aria-label="桌宠快速上手" className="guide-article">
          <div className="guide-welcome"><h3>先陪你待一会儿。</h3><p>不填 API Key，也能使用角色、拖拽、自主动作和哄睡。想聊天时，再连接 AI。</p></div>
          <ol className="guide-basics">
            <li><strong>移动你的伙伴</strong><p>按住角色拖动，松手后会落地；快速甩动可抛出。想让它休息，可以在工具箱或托盘中选择「哄睡」。</p></li>
            <li><strong>找到需要的功能</strong><p>点击桌面的工具箱，再选择聊天、记忆、人物、主题或设置。人物页可以调整角色大小；面板边缘可以拖动调整尺寸。</p></li>
            <li><strong>随时收起，不怕找不到</strong><p>收起工具箱后，在系统托盘中选择「显示工具箱」恢复。想彻底结束运行，请在托盘中选择「退出」。</p></li>
          </ol>
          <section className="ui-card guide-copy"><h3 className="guide-section-heading">关于聊天和数据</h3><p>「记忆」目前主要用于查看聊天记录和人物存档，不代表角色会自动永久记住所有对话。不同人物的聊天独立保存。</p><p>人物与聊天记录保存在本机；使用 AI 时，必要的对话上下文会发送给你选择的接口服务。密钥保存在系统凭据库，不要公开分享。</p></section>
          <div className="guide-actions"><button className="ui-button ui-button-primary" type="button" onClick={() => setTopic("deepseek")}>我想开启 AI 聊天</button><button className="ui-button ui-button-quiet" type="button" onClick={onWelcome}>重新查看入门引导</button></div>
        </article> : <DeepSeekGuide mode={topic === "deepseek" ? "setup" : "troubleshooting"} onBack={onSettings} onOpenLink={openLink} />}
      </div>
    </div>
  </PanelShell>;
}
