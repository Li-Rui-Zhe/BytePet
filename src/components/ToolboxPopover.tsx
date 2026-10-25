import { Minimize2, Moon, Settings, Sparkles } from "lucide-react";
import { ActionBar } from "./ActionBar";
import { getToolboxLayout, useViewportSize } from "./boxLayout";
import { useAppStore } from "../stores/useAppStore";
import type { ActivePanel } from "../types/domain";

interface ToolboxPopoverProps {
  activePanel: ActivePanel | null;
  onAction: (panel: ActivePanel) => void;
}

export function ToolboxPopover({ activePanel, onAction }: ToolboxPopoverProps) {
  const viewport = useViewportSize();
  const boxPosition = useAppStore((s) => s.boxPosition);
  const hideToolbox = useAppStore((s) => s.hideToolbox);
  const lullToSleep = useAppStore((s) => s.lullToSleep);
  const layout = getToolboxLayout(boxPosition, viewport);

  return (
    <section
      className="toolbox-popover no-drag pointer-events-auto fixed z-40"
      data-interactive="true"
      data-placement={layout.placement}
      style={{
        left: layout.x,
        top: layout.y,
        width: layout.width,
        minHeight: layout.height,
      }}
      aria-label="工具箱"
    >
      <div className="toolbox-popover-header">
        <span className="toolbox-kicker">
          <Sparkles size={13} />
          工具箱
        </span>
        <span className="toolbox-shortcut">拖动可移动</span>
        <button
          className="toolbox-minimize-button"
          type="button"
          title="收起工具箱（系统托盘可恢复）"
          aria-label="收起工具箱"
          onClick={hideToolbox}
        >
          <Minimize2 size={14} />
        </button>
      </div>
      <ActionBar activePanel={activePanel} hidePanels={["settings", "help"]} onAction={onAction} />
      <div className="toolbox-extra-actions">
        <button
          className={`toolbox-action toolbox-settings-action ${activePanel === "settings" ? "active" : ""}`}
          type="button"
          title="AI 模型与系统偏好"
          aria-label="设置"
          aria-pressed={activePanel === "settings"}
          onClick={() => onAction("settings")}
        >
          <span className="toolbox-action-icon">
            <Settings size={16} />
          </span>
          <span className="toolbox-action-text">设置</span>
        </button>
        <button className="toolbox-action toolbox-sleep-action" type="button" title="一键哄睡" onClick={lullToSleep}>
          <span className="toolbox-action-icon">
            <Moon size={16} />
          </span>
          <span className="toolbox-action-text">哄睡</span>
        </button>
      </div>
    </section>
  );
}
