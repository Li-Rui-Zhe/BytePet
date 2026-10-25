import { useState } from "react";
import { PanelShell } from "./PanelShell";

export const WELCOME_STEPS = [
  { title: "先认识一下你的桌面伙伴", text: "按住角色可以拖动，松手后它会落地。点击桌面的工具箱，可以聊天、管理人物或调整设置。", note: "工具箱收起后，可以从系统托盘恢复。" },
  { title: "先陪伴，不用急着配置 AI", text: "没有 API Key，也可以使用本地角色、拖拽和自主动作。你可以先玩一会儿，再决定要不要开启 AI 聊天。", note: "跳过引导不会影响这些本地功能。" },
  { title: "想聊天时，再连接 DeepSeek", text: "在设置里填写自己的 DeepSeek API Key，选择模型并保存。也可以保存多个带名称的配置，随时切换一个使用。", note: "API 按用量计费；只有你主动测试或聊天时才发起请求。" },
];

interface Props { onFinish: () => void; onConfigure: () => void; onGuide: () => void }

export function WelcomePanel({ onFinish, onConfigure, onGuide }: Props) {
  const [step, setStep] = useState(0);
  const current = WELCOME_STEPS[step];
  const last = step === WELCOME_STEPS.length - 1;
  return <PanelShell title="欢迎来到 BytePet" subtitle="轻量入门" preferredSize={{ width: 380, height: 420 }} className="onboarding-shell" hideNavigation onClose={onFinish}>
    <div className="onboarding-layout">
      <div key={step} className="onboarding-body ui-scroll">
        <div className="onboarding-progress" aria-label={`入门步骤 ${step + 1} / ${WELCOME_STEPS.length}`}>
          <div className="onboarding-step-caption"><span>{["桌面互动", "先体验，再配置", "开启 AI 聊天"][step]}</span><span>{String(step + 1).padStart(2, "0")} / 03</span></div>
          <div className="onboarding-progress-track" aria-hidden="true">{WELCOME_STEPS.map((_, index) => <span key={index} data-complete={index <= step} />)}</div>
        </div>
        <h3 className="onboarding-title">{current.title}</h3>
        <p className="onboarding-description">{current.text}</p>
        <p className="onboarding-note">{current.note}</p>
      </div>
      <div className="onboarding-footer">
        <div className="onboarding-actions">
          <button type="button" className="ui-link onboarding-skip" onClick={onFinish}>{last ? "先开始陪伴" : "跳过，先使用"}</button>
          <div className="onboarding-next-actions">
            {step > 0 ? <button type="button" className="ui-button ui-button-quiet" onClick={() => setStep(step - 1)}>上一步</button> : null}
            <button type="button" className="ui-button ui-button-primary" onClick={last ? onConfigure : () => setStep(step + 1)}>{last ? "配置 AI 聊天" : "下一步"}</button>
          </div>
        </div>
        <button type="button" className="ui-link onboarding-guide" onClick={onGuide}>查看详细指南</button>
      </div>
    </div>
  </PanelShell>;
}
