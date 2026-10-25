import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { BoxButton } from "../components/BoxButton";
import { ChatPanel } from "../components/ChatPanel";
import { MemoryPanel } from "../components/MemoryPanel";
import { PetSprite } from "../components/PetSprite";
import { SettingsPanel } from "../components/SettingsPanel";
import { SkinPanel } from "../components/SkinPanel";
import { ThemePanel } from "../components/ThemePanel";
import { HelpPanel, type HelpTopic } from "../components/HelpPanel";
import type { ActivePanel } from "../types/domain";
import { WelcomePanel } from "../components/WelcomePanel";
import { hasCompletedOnboarding, completeOnboarding } from "../systems/settings/onboarding";
import { ToolboxPopover } from "../components/ToolboxPopover";
import { useClickThrough } from "../hooks/useClickThrough";
import { useAppStore } from "../stores/useAppStore";
import { applyTheme, readActiveTheme } from "../theme/theme";

export function App() {
  useClickThrough();

  const {
    activePanel,
    bootstrap,
    character,
    characters,
    closePanel,
    createCharacter,
    currentState,
    dataPaths,
    deleteChatHistoryDay,
    deleteCharacter,
    deleteFrameAsset,
    endDrag,
    frameAssets,
    loading,
    messages,
    openDataDir,
    openPanel: openStorePanel,
    sessions,
    startDrag,
    tickAnimation,
    tickMovement,
    updateDrag,
    updatePhysics,
    switchCharacter,
    updateCharacter,
    previewCharacterScale,
    updateCharacterScale,
    chooseAndImportFrameAsset,
    importFrameAssetFromPath,
    body,
    boxOpen,
    toolboxVisible,
    showToolbox,
    lullToSleep,
    initScreenInfo,
  } = useAppStore();

  const [helpTopic, setHelpTopic] = useState<HelpTopic>("start");
  const openPanel = useCallback((panel: ActivePanel) => {
    if (panel === "help") setHelpTopic("start");
    openStorePanel(panel);
  }, [openStorePanel]);

  useEffect(() => {
    if (activePanel !== "help") setHelpTopic("start");
  }, [activePanel]);

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    let refreshing = false;
    let disposed = false;
    const refresh = async () => {
      if (refreshing || disposed || document.hidden) return;
      refreshing = true;
      try {
        await invoke("set_window_to_screen_size");
        if (!disposed) await initScreenInfo();
      } catch { /* A monitor may briefly disappear during display reconfiguration. */ }
      finally { refreshing = false; }
    };
    const timer = window.setInterval(() => void refresh(), 5000);
    window.addEventListener("focus", refresh);
    window.addEventListener("resize", refresh);
    document.addEventListener("visibilitychange", refresh);
    return () => { disposed = true; window.clearInterval(timer); window.removeEventListener("focus", refresh); window.removeEventListener("resize", refresh); document.removeEventListener("visibilitychange", refresh); };
  }, [initScreenInfo]);

  const onboardingChecked = useRef(false);
  useEffect(() => {
    if (loading || !character || onboardingChecked.current) return;
    onboardingChecked.current = true;
    if (!hasCompletedOnboarding()) openPanel("welcome");
  }, [loading, character, openPanel]);

  function finishWelcome(destination?: "settings" | "help"): void {
    completeOnboarding();
    if (destination) openPanel(destination);
    else closePanel();
  }

  useEffect(() => {
    applyTheme(readActiveTheme());
  }, []);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;

    const appWindow = getCurrentWindow();
    let disposed = false;
    let unlisten: (() => void) | null = null;

    void appWindow.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        void appWindow.setSkipTaskbar(true).catch(() => {});
      }
    }).then((handler) => {
      if (disposed) {
        return handler();
      } else {
        unlisten = handler;
      }
    }).catch(() => {});

    return () => {
      disposed = true;
      void Promise.resolve().then(() => unlisten?.()).catch(() => {});
    };
  }, []);

  // Listen for system tray events
  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;

    const unlisten = listen("tray-show-toolbox", () => {
      showToolbox();
    }).catch(() => () => {});
    const unlistenSleep = listen("tray-lull-to-sleep", () => {
      lullToSleep();
    }).catch(() => () => {});

    return () => {
      void unlisten.then((fn) => fn()).catch(() => {});
      void unlistenSleep.then((fn) => fn()).catch(() => {});
    };
  }, [lullToSleep, showToolbox]);

  useEffect(() => {
    const timer = window.setInterval(tickAnimation, 250);
    return () => window.clearInterval(timer);
  }, [tickAnimation]);

  const updatePhysicsRef = useRef(updatePhysics);
  updatePhysicsRef.current = updatePhysics;

  useEffect(() => {
    let lastTime = performance.now();
    let animFrameId: number;

    const loop = (time: number) => {
      const dt = Math.min((time - lastTime) / 1000, 0.05);
      lastTime = time;
      updatePhysicsRef.current(dt);
      animFrameId = requestAnimationFrame(loop);
    };

    animFrameId = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(animFrameId);
  }, []);

  useEffect(() => {
    const timer = window.setInterval(tickMovement, 111);
    return () => window.clearInterval(timer);
  }, [tickMovement]);

  const handleDragStart = useCallback((offsetX: number, offsetY: number) => {
    startDrag(offsetX, offsetY);
  }, [startDrag]);

  const handleDrag = useCallback((x: number, y: number) => {
    updateDrag(x, y);
  }, [updateDrag]);

  const handleDragEnd = useCallback((velocityX: number, velocityY: number) => {
    endDrag(velocityX, velocityY);
  }, [endDrag]);

  return (
    <main className="pointer-events-none relative h-screen w-screen overflow-hidden bg-transparent text-slate-800">
      <PetSprite
        currentState={currentState}
        skinId={character?.skinId ?? "rick_default"}
        frameAssetsPath={character?.frameAssetsPath ?? null}
        displayScale={character?.displayScale ?? 1}
        size={body.width}
        position={{ x: body.x, y: body.y }}
        onDragStart={handleDragStart}
        onDrag={handleDrag}
        onDragEnd={handleDragEnd}
      />

      {toolboxVisible ? <BoxButton /> : null}

      {toolboxVisible && boxOpen && !activePanel ? <ToolboxPopover activePanel={activePanel} onAction={openPanel} /> : null}

      {loading ? (
        <div
          className="glass-panel no-drag pointer-events-auto absolute right-4 top-[200px] rounded-[8px] px-3 py-2 text-[12px] font-semibold text-sky-800"
          data-interactive="true"
        >
          初始化本地数据...
        </div>
      ) : null}

      {activePanel === "chat" ? <ChatPanel character={character} messages={messages} onClose={closePanel} /> : null}
      {activePanel === "skin" ? (
        <SkinPanel
          character={character}
          characters={characters}
          frameAssets={frameAssets}
          onChooseAndImportFrameAsset={chooseAndImportFrameAsset}
          onImportFrameAssetFromPath={importFrameAssetFromPath}
          onDeleteFrameAsset={deleteFrameAsset}
          onCreateCharacter={createCharacter}
          onSwitchCharacter={switchCharacter}
          onUpdateCharacter={updateCharacter}
          onPreviewCharacterScale={previewCharacterScale}
          onUpdateCharacterScale={updateCharacterScale}
          onDeleteCharacter={deleteCharacter}
          onClose={closePanel}
        />
      ) : null}
      {activePanel === "help" ? <HelpPanel key={helpTopic} initialTopic={helpTopic} onClose={closePanel} onSettings={() => openPanel("settings")} onWelcome={() => openPanel("welcome")} /> : null}
      {activePanel === "welcome" ? <WelcomePanel onFinish={() => finishWelcome()} onConfigure={() => finishWelcome("settings")} onGuide={() => finishWelcome("help")} /> : null}
      {activePanel === "settings" || activePanel === "help" ? (
        <div hidden={activePanel !== "settings"}>
        <SettingsPanel
          dataPaths={dataPaths}
          onOpenDataDir={openDataDir}
          onClose={closePanel}
          onOpenGuide={() => { setHelpTopic("deepseek"); openStorePanel("help"); }}
        />
        </div>
      ) : null}
      {activePanel === "theme" ? <ThemePanel onClose={closePanel} /> : null}
      {activePanel === "memory" ? (
        <MemoryPanel
          character={character}
          characters={characters}
          sessions={sessions}
          messages={messages}
          onDeleteHistoryDay={deleteChatHistoryDay}
          onSwitchCharacter={switchCharacter}
          onManageCharacters={() => openPanel("skin")}
          onClose={closePanel}
        />
      ) : null}
    </main>
  );
}
