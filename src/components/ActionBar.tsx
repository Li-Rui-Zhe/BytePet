import clsx from "clsx";
import { BookOpen, Brain, MessageCircle, Palette, Settings, UserRoundPlus } from "lucide-react";
import type { ActivePanel } from "../types/domain";

interface ActionBarProps {
  activePanel: ActivePanel | null;
  compact?: boolean;
  hidePanels?: ActivePanel[];
  onAction: (panel: ActivePanel) => void;
}

const ACTIONS: Array<{ panel: ActivePanel; label: string; hint: string; icon: typeof MessageCircle }> = [
  { panel: "chat", label: "聊天", hint: "和当前人物聊天", icon: MessageCircle },
  { panel: "memory", label: "记忆", hint: "历史聊天和人物存档", icon: Brain },
  { panel: "skin", label: "人物", hint: "新建或管理人物", icon: UserRoundPlus },
  { panel: "theme", label: "主题", hint: "更改界面颜色", icon: Palette },
  { panel: "settings", label: "设置", hint: "AI 模型与系统偏好", icon: Settings },
  { panel: "help", label: "指南", hint: "使用指南与轻量入门引导", icon: BookOpen },
];

export function ActionBar({ activePanel, compact = false, hidePanels = [], onAction }: ActionBarProps) {
  const visibleActions = ACTIONS.filter((action) => !hidePanels.includes(action.panel));

  return (
    <nav aria-label="功能导航" className={clsx(compact ? "toolbox-nav compact" : "toolbox-nav")}>
      {visibleActions.map((action) => {
        const Icon = action.icon;
        const active = activePanel === action.panel;

        return (
          <button
            key={action.panel}
            className={clsx("toolbox-action", active && "active")}
            type="button"
            title={action.hint}
            aria-label={action.label}
            aria-pressed={active}
            onClick={() => onAction(action.panel)}
          >
            <span className="toolbox-action-icon">
              <Icon size={compact ? 14 : 16} />
            </span>
            <span className="toolbox-action-text">{action.label}</span>
          </button>
        );
      })}
    </nav>
  );
}
