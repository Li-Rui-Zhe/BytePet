import { ArrowRight, ExternalLink } from "lucide-react";
import type { ReactNode } from "react";
import { deepseekHelpLinks, type DeepSeekHelpLink } from "../services/helpLinks";
import { DEEPSEEK_MODEL } from "../config/aiProviders";

interface DeepSeekGuideProps {
  onBack: () => void;
  onOpenLink: (id: DeepSeekHelpLink) => Promise<void>;
  mode?: "all" | "setup" | "troubleshooting";
}

function GuideSection({ number, title, children }: { number: number; title: string; children: ReactNode }) {
  return <section className="ui-card guide-section"><h3 className="guide-section-heading"><span className="guide-step-number">{number}</span>{title}</h3><div className="guide-copy">{children}</div></section>;
}

export function DeepSeekGuide({ onBack, onOpenLink, mode = "all" }: DeepSeekGuideProps) {
  function link(id: DeepSeekHelpLink, label = deepseekHelpLinks[id].label) {
    return <button type="button" className="ui-button ui-button-secondary ui-button-small guide-external" onClick={() => void onOpenLink(id)} title="在默认浏览器中打开官方页面">{label}<ExternalLink size={12} /></button>;
  }
  return <article aria-label={mode === "troubleshooting" ? "连接问题排查" : "DeepSeek 新手指南"} className="guide-article">
    {mode !== "troubleshooting" ? <>
      <div className="guide-intro"><h3>四步开启 DeepSeek 聊天</h3><p>只需准备自己的 API Key，接口地址和默认模型已经填好。</p></div>
      <p className="ui-notice">这是开放平台 API，不是免费聊天网页。API 按用量计费；测试连接也可能产生少量费用，价格与额度以官方为准。</p>
      <GuideSection number={1} title="登录 DeepSeek 开放平台">
        <p>在默认浏览器中注册或登录账号，找到「API keys / API 密钥」。</p>{link("keys")}
        <p className="guide-url">{deepseekHelpLinks.keys.url}</p>
      </GuideSection>
      <GuideSection number={2} title="创建并复制 API Key">
        <p>创建新密钥，名称可写「桌宠」，使用平台的复制按钮复制完整密钥，不要只复制星号遮挡的片段。</p>
        <p>Key 是调用凭证，不是账号密码。不要发到截图、群聊、Issue 或仓库；泄露后立即在官方平台撤销并更换。</p>
      </GuideSection>
      <GuideSection number={3} title="回到桌宠，粘贴并测试">
        <p>打开「设置」，保持 DeepSeek；若勾选了「使用自定义接口」，先取消勾选。普通用户不用填写 Base URL 和模型 ID。</p>
        <p>在 API Key 输入框按 Ctrl + V（Windows）或 ⌘ + V（macOS）粘贴。只填密钥本身，不加 Bearer、引号或网址。</p>
        <p>默认模型为 <code>{DEEPSEEK_MODEL}</code>。点击「测试连接」，测试只验证当前草稿，不保存、不切换连接。</p>
      </GuideSection>
      <GuideSection number={4} title="保存并开始聊天">
        <p>点击「保存并使用」，确认顶部「正在使用」显示所选配置名称。进入「聊天」发送一句话即可。</p>
        <button type="button" className="ui-button ui-button-primary ui-button-small" onClick={onBack}>打开连接设置<ArrowRight size={13} /></button>
      </GuideSection>
      <section className="ui-card guide-faq">
        <h3>以后怎么更换？</h3>
        <details><summary>多个密钥或切换模型</summary><p>点击「新增配置」，分别命名、选择模型和填入 Key。新增配置不会继承其他配置的密钥。「仅保存，不切换」可保存备用；列表「启用」使用已保存的信息，一次只启用一个。</p><p>修改正在使用的配置并保存，会影响后续聊天。DeepSeek 模型可在此配置内直接切换，不必启用自定义接口。</p></details>
        <details><summary>已保存密钥、更换和删除</summary><p>密钥保存在系统凭据库。输入框留空会保留此配置原 Key；眼睛按钮只显示当前输入，不会读回已存 Key。要更换，粘贴新 Key 后保存。</p><p>删除配置只删除本地信息，不撤销平台密钥；正在使用的配置须先切换后再删除。换电脑或系统账号时需要重新填写 Key。</p></details>
      </section>
    </> : null}
    {mode !== "setup" ? <>
      <div className="guide-intro"><h3>遇到问题，按情况排查</h3><p>先看错误提示，避免连续重复测试。反馈时请隐藏 Key、私人聊天和本地路径等信息。</p></div>
      <section className="ui-card guide-faq">
        <h3>连接问题</h3>
        <details><summary>401 / 密钥认证失败</summary><p>重新复制完整密钥，去掉误加的空格、引号或 Bearer；确认密钥未被撤销。无法再查看完整 Key 时，可以在官方平台新建密钥。</p></details>
        <details><summary>402 / 余额不足</summary><p>检查 API 可用余额，是否充值由你决定。聊天网页可用，不代表开放平台 API 有可用额度。</p>{link("balance")}</details>
        <details><summary>400 / 422 / 参数或模型错误</summary><p>DeepSeek 用户先选择内置模型后重新测试；自定义接口请核对模型 ID、API 地址及该服务的 Chat Completions 支持情况。</p></details>
        <details><summary>429 / 500 / 503</summary><p>可能是限流、服务故障或繁忙。稍等后重试，避免连续点击测试。</p></details>
        <details><summary>超时 / 网络连接失败</summary><p>检查网络、系统代理和防火墙，确认浏览器能打开官方平台。能打开网页不代表 API 请求一定成功。</p></details>
        <details><summary>测试通过，但还不能聊天</summary><p>确认已点击「保存并使用」，顶部配置名称正确。测试成功只代表该次短请求成功；额度、网络和服务状态仍可能变化。</p></details>
        <details><summary>自定义接口返回网页或空回复</summary><p>填写兼容 API 地址，而不是服务商的聊天网页。Base URL 自动追加 /chat/completions，也接受完整接口地址；模型名称需与服务商文档一致。</p></details>
        <div className="guide-actions">{link("errors")}{link("pricing")}{link("docs")}</div>
      </section>
      <section className="ui-card guide-faq">
        <h3>桌面与数据</h3>
        <details><summary>找不到工具箱或想退出应用</summary><p>从系统托盘菜单选择「显示工具箱」恢复；「退出」才会结束运行。收起或最小化不等于退出。</p></details>
        <details><summary>数据在哪里？复制数据库能带走密钥吗？</summary><p>在「设置 → 本地数据 → 打开数据目录」查看。不要在应用运行时随意删除或移动数据库；先退出再自行备份整个数据目录。数据库不包含系统凭据本体，换机器需要重填 Key，旧版本备份仍可能含明文密钥。</p></details>
        <details><summary>为什么没有永久记住所有对话？</summary><p>记录保存在本机，但聊天只发送有限的近期上下文。「记忆」不是已经完成的自动长期记忆系统。</p></details>
      </section>
      <section className="ui-card guide-faq"><h3>其他服务由高级用户 DIY</h3><p>项目主要维护和测试 DeepSeek。高级设置使用通用 OpenAI 兼容 Chat Completions 协议，不逐家适配原生协议，也不保证所有服务始终可用。</p><p>仍欢迎提交 Issue 和兼容性反馈；请提供服务商、模型、复现步骤与脱敏错误，改进会按通用性和维护成本评估。</p></section>
      <button type="button" className="ui-button ui-button-secondary" onClick={onBack}>打开连接设置<ArrowRight size={13} /></button>
    </> : null}
    <p className="guide-footnote">核对日期：2026-10-25。教程可离线阅读；外部页面、测试连接与 AI 聊天需要联网。平台页面与价格请以官方为准。</p>
  </article>;
}
