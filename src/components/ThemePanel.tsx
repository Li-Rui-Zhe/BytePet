import { Check, RotateCcw, Save, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import {
  ACTIVE_THEME_KEY,
  DEFAULT_THEME,
  SYSTEM_THEMES,
  applyTheme,
  createThemeFromPrimary,
  readActiveTheme,
  readCustomThemes,
  resetTheme,
  saveCustomThemes,
  setActiveTheme,
  type Theme,
} from "../theme/theme";
import { PanelShell } from "./PanelShell";

interface ThemePanelProps {
  onClose: () => void;
}

const COLOR_FIELDS: Array<{ key: keyof Omit<Theme, "id" | "name">; label: string }> = [
  { key: "primary", label: "主色" },
  { key: "background", label: "背景" },
  { key: "panelBg", label: "弹窗" },
  { key: "buttonBg", label: "按钮" },
  { key: "text", label: "文字" },
  { key: "border", label: "边框" },
  { key: "accent", label: "强调" },
];

function isHexColor(value: string): boolean {
  return /^#[0-9a-f]{6}$/i.test(value);
}

function colorInputValue(value: string, fallback = "#ffffff"): string {
  return isHexColor(value) ? value : fallback;
}

function ThemeCard({
  theme,
  active,
  removable = false,
  onApply,
  onDelete,
}: {
  theme: Theme;
  active: boolean;
  removable?: boolean;
  onApply: () => void;
  onDelete?: () => void;
}) {
  const swatches = [theme.primary, theme.background, theme.panelBg, theme.buttonBg, theme.border, theme.text, theme.accent];

  return (
    <div
      className={`theme-card ${active ? "active" : ""}`}
      data-removable={removable}
    >
      <button className="theme-choice" type="button" onClick={onApply} aria-pressed={active} title={theme.name}>
      <span className="theme-card-top">
        <span className="theme-card-name">{theme.name}</span>
        {active ? <Check size={13} /> : null}
      </span>
      <span className="theme-swatches">
        {swatches.map((color, index) => (
          <span key={`${theme.id}-${index}`} className="theme-swatch" style={{ background: color }} />
        ))}
      </span>
      </button>
      {removable ? (
        <button
          className="theme-delete"
          type="button"
          aria-label="删除我的主题"
          title="删除我的主题"
          onClick={(event) => {
            event.stopPropagation();
            onDelete?.();
          }}
        >
          <Trash2 size={12} />
        </button>
      ) : null}
    </div>
  );
}

export function ThemePanel({ onClose }: ThemePanelProps) {
  const [activeTheme, setActiveThemeState] = useState(readActiveTheme);
  const [customThemes, setCustomThemesState] = useState(readCustomThemes);
  const [draft, setDraft] = useState<Theme>(() => ({
    ...createThemeFromPrimary(DEFAULT_THEME.primary),
    id: "draft",
    name: "自定义预览",
  }));

  useEffect(() => {
    const update = () => {
      setActiveThemeState(readActiveTheme());
      setCustomThemesState(readCustomThemes());
    };
    window.addEventListener("pet-theme-change", update);
    return () => window.removeEventListener("pet-theme-change", update);
  }, []);

  function applyAndSync(theme: Theme): void {
    setActiveTheme(theme);
    setActiveThemeState(theme);
  }

  function updateDraft(key: keyof Omit<Theme, "id" | "name">, value: string): void {
    setDraft((current) => {
      const next = key === "primary"
        ? { ...createThemeFromPrimary(value, current), id: current.id, name: current.name }
        : { ...current, [key]: value };
      applyTheme(next);
      return next;
    });
  }

  function saveDraft(): void {
    const now = Date.now();
    const saved: Theme = {
      ...draft,
      id: `custom-${now}`,
      name: `我的主题 ${customThemes.length + 1}`,
    };
    const next = [saved, ...customThemes];
    saveCustomThemes(next);
    applyAndSync(saved);
  }

  function deleteCustomTheme(themeId: string): void {
    const next = customThemes.filter((theme) => theme.id !== themeId);
    saveCustomThemes(next);
    setCustomThemesState(next);
    if (localStorage.getItem(ACTIVE_THEME_KEY) === themeId) {
      resetTheme();
      setActiveThemeState(DEFAULT_THEME);
    }
  }

  function resetDefault(): void {
    resetTheme();
    setActiveThemeState(DEFAULT_THEME);
    setDraft({ ...createThemeFromPrimary(DEFAULT_THEME.primary), id: "draft", name: "自定义预览" });
  }

  return (
    <PanelShell title="主题颜色" subtitle="选择一套颜色，或自己搭配" preferredSize={{ width: 380, height: 560 }} className="theme-shell" onClose={onClose}>
      <div className="panel-scroll ui-scroll">
        <div className="space-y-3 pb-2">
          <section>
            <h3 className="theme-section-title">系统主题</h3>
            <div className="theme-grid">
              {SYSTEM_THEMES.map((theme) => (
                <ThemeCard
                  key={theme.id}
                  theme={theme}
                  active={activeTheme.id === theme.id}
                  onApply={() => applyAndSync(theme)}
                />
              ))}
            </div>
          </section>

          <section className="ui-card theme-custom-box">
            <div className="flex items-center justify-between gap-2">
              <h3 className="theme-section-title">自定义颜色</h3>
              <span className="text-[10px]" style={{ color: "var(--pet-muted-text)" }}>
                修改即预览
              </span>
            </div>
            <div className="theme-color-grid">
              {COLOR_FIELDS.map((field) => (
                <label key={field.key} className="theme-color-field">
                  <span>{field.label}</span>
                  <input
                    type="color"
                    aria-label={`${field.label}颜色`}
                    value={colorInputValue(draft[field.key])}
                    onChange={(event) => updateDraft(field.key, event.target.value)}
                  />
                </label>
              ))}
            </div>
            <div className="mt-2 grid grid-cols-2 gap-2">
              <button className="ui-button ui-button-secondary" type="button" onClick={resetDefault}>
                <RotateCcw size={13} />
                重置默认
              </button>
              <button className="ui-button ui-button-primary" type="button" onClick={saveDraft}>
                <Save size={13} />
                保存主题
              </button>
            </div>
          </section>

          <section>
            <h3 className="theme-section-title">我的主题</h3>
            {customThemes.length ? (
              <div className="theme-grid">
                {customThemes.map((theme) => (
                  <ThemeCard
                    key={theme.id}
                    theme={theme}
                    active={activeTheme.id === theme.id}
                    removable
                    onApply={() => applyAndSync(theme)}
                    onDelete={() => deleteCustomTheme(theme.id)}
                  />
                ))}
              </div>
            ) : (
              <div className="ui-empty">保存后会出现在这里。</div>
            )}
          </section>
        </div>
      </div>
    </PanelShell>
  );
}
