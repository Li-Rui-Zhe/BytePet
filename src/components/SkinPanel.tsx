import { CheckCircle2, FolderOpen, Pencil, Trash2, UploadCloud, UserRoundPlus } from "lucide-react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useCallback, useEffect, useRef, useState } from "react";
import type {
  CharacterProfile,
  CreateCharacterRequest,
  FrameAssetOption,
  UpdateCharacterRequest,
} from "../types/domain";
import { createScaleSaver } from "../systems/physics/scaleSaver";
import { PanelShell } from "./PanelShell";
import { RangeSlider } from "./RangeSlider";

interface SkinPanelProps {
  character: CharacterProfile | null;
  characters: CharacterProfile[];
  frameAssets: FrameAssetOption[];
  onChooseAndImportFrameAsset: () => Promise<FrameAssetOption | null>;
  onImportFrameAssetFromPath: (path: string) => Promise<FrameAssetOption>;
  onDeleteFrameAsset: (assetId: string) => Promise<void>;
  onCreateCharacter: (request: CreateCharacterRequest) => Promise<void>;
  onSwitchCharacter: (characterId: string) => Promise<void>;
  onUpdateCharacter: (request: UpdateCharacterRequest) => Promise<void>;
  onPreviewCharacterScale: (characterId: string, displayScale: number) => void;
  onUpdateCharacterScale: (characterId: string, displayScale: number) => Promise<void>;
  onDeleteCharacter: (characterId: string) => Promise<void>;
  onClose: () => void;
}

type Mode = "create" | "edit";
const MIN_CHARACTER_SCALE = 0.75;
const MAX_CHARACTER_SCALE = 3;
const CHARACTER_SCALE_STEP = 0.05;

function errorMessage(err: unknown, fallback: string): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  return fallback;
}

export function SkinPanel({
  character,
  characters,
  frameAssets,
  onChooseAndImportFrameAsset,
  onImportFrameAssetFromPath,
  onDeleteFrameAsset,
  onCreateCharacter,
  onSwitchCharacter,
  onUpdateCharacter,
  onPreviewCharacterScale,
  onUpdateCharacterScale,
  onDeleteCharacter,
  onClose,
}: SkinPanelProps) {
  const [mode, setMode] = useState<Mode>("create");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [prompt, setPrompt] = useState("");
  const [selectedAssetId, setSelectedAssetId] = useState("rick_default");
  const [saving, setSaving] = useState(false);
  const [importing, setImporting] = useState(false);
  const [deletingAsset, setDeletingAsset] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);
  const [pendingDeleteAssetId, setPendingDeleteAssetId] = useState<string | null>(null);
  const dropZoneRef = useRef<HTMLDivElement>(null);
  const mountedRef = useRef(true);
  const saveScaleRef = useRef(onUpdateCharacterScale);
  saveScaleRef.current = onUpdateCharacterScale;
  const scaleSaverRef = useRef<ReturnType<typeof createScaleSaver> | null>(null);
  if (!scaleSaverRef.current) {
    scaleSaverRef.current = createScaleSaver((id, value) => saveScaleRef.current(id, value), (err) => {
      if (mountedRef.current) setError(errorMessage(err, "角色大小保存失败"));
    });
  }
  const selectedAsset = frameAssets.find((asset) => asset.id === selectedAssetId) ?? frameAssets[0];
  const selectedAssetUsers = selectedAsset
    ? characters.filter((profile) => profile.skinId === selectedAsset.id)
    : [];
  const canDeleteSelectedAsset =
    Boolean(selectedAsset) &&
    !selectedAsset?.builtIn &&
    frameAssets.length > 1 &&
    selectedAssetUsers.length === 0;

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; scaleSaverRef.current?.flush(); };
  }, []);

  const updateScale = useCallback(
    (profile: CharacterProfile, rawScale: number) => {
      if (!profile.isActive) return;
      const displayScale = Math.min(MAX_CHARACTER_SCALE, Math.max(MIN_CHARACTER_SCALE, rawScale));
      onPreviewCharacterScale(profile.id, displayScale);
      scaleSaverRef.current?.schedule(profile.id, displayScale);
    },
    [onPreviewCharacterScale]
  );

  function resetCreateForm(): void {
    setMode("create");
    setEditingId(null);
    setName("");
    setPrompt("");
    setSelectedAssetId("rick_default");
    setError(null);
    setPendingDeleteId(null);
    setPendingDeleteAssetId(null);
  }

  function editProfile(profile: CharacterProfile): void {
    setName(profile.name);
    setPrompt(profile.prompt || profile.description);
    const asset = frameAssets.find(item => item.id === profile.skinId || Boolean(item.path && item.path === profile.frameAssetsPath));
    setSelectedAssetId(asset?.id ?? "rick_default");
    setError(null);
    setMode("edit");
    setEditingId(profile.id);
    setPendingDeleteId(null);
  }

  const isInsideDropZone = useCallback((x: number, y: number): boolean => {
    const rect = dropZoneRef.current?.getBoundingClientRect();
    if (!rect) return false;
    const scale = window.devicePixelRatio || 1;
    const cssX = x / scale;
    const cssY = y / scale;
    return cssX >= rect.left && cssX <= rect.right && cssY >= rect.top && cssY <= rect.bottom;
  }, []);

  const importPath = useCallback(
    async (path: string): Promise<void> => {
      if (!path || importing) return;
      setImporting(true);
      setError(null);
      try {
        const asset = await onImportFrameAssetFromPath(path);
        setSelectedAssetId(asset.id);
        setPendingDeleteAssetId(null);
      } catch (err) {
        setError(errorMessage(err, "导入失败"));
      } finally {
        setImporting(false);
        setDragActive(false);
      }
    },
    [importing, onImportFrameAssetFromPath]
  );

  useEffect(() => {
    // getCurrentWebview reads native metadata synchronously; a Promise catch
    // alone cannot protect the browser preview from that initial failure.
    if (!window.__TAURI_INTERNALS__) return;
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          setDragActive(isInsideDropZone(payload.position.x, payload.position.y));
          return;
        }
        if (payload.type === "leave") {
          setDragActive(false);
          return;
        }
        if (payload.type === "drop") {
          const inside = isInsideDropZone(payload.position.x, payload.position.y);
          setDragActive(false);
          if (inside && payload.paths[0]) {
            void importPath(payload.paths[0]);
          }
        }
      })
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch(() => {
        // Browser preview does not expose the Tauri drag-drop event API.
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [importPath, isInsideDropZone]);

  async function importFromFolderPicker(): Promise<void> {
    setImporting(true);
    setError(null);
    try {
      const asset = await onChooseAndImportFrameAsset();
      if (asset) setSelectedAssetId(asset.id);
      setPendingDeleteAssetId(null);
    } catch (err) {
      setError(errorMessage(err, "导入失败"));
    } finally {
      setImporting(false);
    }
  }

  async function importDroppedPath(event: React.DragEvent<HTMLDivElement>): Promise<void> {
    event.preventDefault();
    setDragActive(false);
    const file = event.dataTransfer.files.item(0) as (File & { path?: string }) | null;
    const path = file?.path || event.dataTransfer.getData("text/plain");
    if (!path) {
      setError("无法读取拖入文件夹路径。请直接拖入文件夹，或使用“选择文件夹并导入”。");
      return;
    }
    await importPath(path);
  }

  async function removeSelectedAsset(): Promise<void> {
    if (!selectedAsset) return;
    if (selectedAsset.builtIn) {
      setError("系统默认素材包不能删除。");
      return;
    }
    if (frameAssets.length <= 1) {
      setError("至少需要保留一个素材包。");
      return;
    }
    if (selectedAssetUsers.length > 0) {
      setError(`该素材包正在被 ${selectedAssetUsers.map((item) => item.name).join("、")} 使用，请先给这些人物换一个素材包。`);
      return;
    }
    if (pendingDeleteAssetId !== selectedAsset.id) {
      setPendingDeleteAssetId(selectedAsset.id);
      setError(null);
      return;
    }

    setDeletingAsset(true);
    setError(null);
    try {
      await onDeleteFrameAsset(selectedAsset.id);
      setSelectedAssetId("rick_default");
      setPendingDeleteAssetId(null);
    } catch (err) {
      setError(errorMessage(err, "删除素材包失败"));
    } finally {
      setDeletingAsset(false);
    }
  }

  async function save(): Promise<void> {
    const trimmedName = name.trim();
    const trimmedPrompt = prompt.trim();
    if (!trimmedName) {
      setError("请输入人物名字");
      return;
    }
    if (trimmedName.length > 30) {
      setError("人物名字最多 30 个字符");
      return;
    }
    if (!trimmedPrompt) {
      setError("请输入人物 prompt");
      return;
    }
    if (trimmedPrompt.length > 1000) {
      setError("人物 prompt 最多 1000 个字符");
      return;
    }

    setSaving(true);
    setError(null);
    try {
      const frameSourcePath = selectedAsset?.id ?? "rick_default";
      if (mode === "edit" && editingId) {
        await onUpdateCharacter({ characterId: editingId, name: trimmedName, prompt: trimmedPrompt, frameSourcePath });
      } else {
        await onCreateCharacter({ name: trimmedName, prompt: trimmedPrompt, frameSourcePath });
      }
      resetCreateForm();
    } catch (err) {
      setError(errorMessage(err, "保存失败"));
    } finally {
      setSaving(false);
    }
  }

  async function remove(profile: CharacterProfile): Promise<void> {
    if (pendingDeleteId !== profile.id) {
      setPendingDeleteId(profile.id);
      setError(null);
      return;
    }
    try {
      await onDeleteCharacter(profile.id);
      if (editingId === profile.id) resetCreateForm();
      setPendingDeleteId(null);
    } catch (err) {
      setError(errorMessage(err, "删除失败"));
    }
  }

  return (
    <PanelShell title="人物" subtitle={character ? `当前人物：${character.name}` : "新建和管理人物"} preferredSize={{ width: 380, height: 580 }} className="characters-shell" onClose={onClose}>
      <div className="panel-scroll ui-scroll">
        <div className="flex min-h-full flex-col gap-3 pb-2">
          <div className="ui-card">
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="flex items-center gap-2 text-sky-700">
                <UserRoundPlus size={17} />
                <span className="text-[12px] font-semibold">{mode === "edit" ? "编辑人物" : "新建人物"}</span>
              </div>
              {mode === "edit" ? (
                <button className="ui-link" type="button" onClick={resetCreateForm}>
                  新建
                </button>
              ) : null}
            </div>

            <div className="space-y-2">
              <label className="ui-field">
                人物名字
                <input
                  className="ui-input"
                  value={name}
                  maxLength={30}
                  placeholder="最多 30 个字符"
                  onChange={(event) => setName(event.target.value)}
                />
              </label>

              <label className="ui-field">
                人物 prompt
                <textarea
                  className="ui-input ui-textarea"
                  value={prompt}
                  maxLength={1000}
                  placeholder="描述人物性格、说话方式和行为边界"
                  onChange={(event) => setPrompt(event.target.value)}
                />
              </label>

              <label className="ui-field">
                PNG 序列帧资源
                <select
                  className="ui-input"
                  value={selectedAssetId}
                  onChange={(event) => {
                    setSelectedAssetId(event.target.value);
                    setPendingDeleteAssetId(null);
                  }}
                >
                  {frameAssets.map((asset) => (
                    <option key={asset.id} value={asset.id}>
                      {asset.name}{asset.builtIn ? "（系统自带）" : ""}{asset.shortActionKeys.length ? ` · ${asset.shortActionKeys.length} 个短动作` : ""}
                    </option>
                  ))}
                </select>
              </label>

              <div
                ref={dropZoneRef}
                className="ui-dropzone"
                data-drag-active={dragActive}
                onDragOver={(event) => {
                  event.preventDefault();
                  setDragActive(true);
                }}
                onDragLeave={() => setDragActive(false)}
                onDrop={(event) => void importDroppedPath(event)}
              >
                <UploadCloud className="mx-auto mb-1" size={18} />
                拖入序列帧文件夹，或点击下方按钮选择下载好的文件夹
              </div>

              <button
                className="ui-button ui-button-secondary w-full"
                type="button"
                disabled={importing}
                onClick={() => void importFromFolderPicker()}
              >
                <FolderOpen size={14} />
                {importing ? "导入中..." : "选择文件夹并导入"}
              </button>

              {selectedAsset ? (
                <div className="ui-notice space-y-2">
                  <div className="flex items-center gap-1.5">
                    <CheckCircle2 size={13} />
                    当前资源：{selectedAsset.name} · 短动作 {selectedAsset.shortActionKeys.length} 个
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      className="ui-button ui-button-secondary ui-button-small ui-button-danger"
                      type="button"
                      disabled={!canDeleteSelectedAsset || deletingAsset}
                      title={
                        selectedAsset.builtIn
                          ? "系统默认素材包不能删除"
                          : selectedAssetUsers.length
                            ? "该素材包正在被人物使用"
                            : "完全删除该素材包文件夹"
                      }
                      onClick={() => void removeSelectedAsset()}
                    >
                      <Trash2 size={12} className="mr-1 inline" />
                      {deletingAsset ? "删除中..." : "删除素材包"}
                    </button>
                    {!canDeleteSelectedAsset ? (
                      <span className="text-[10px] text-slate-500">
                        {selectedAsset.builtIn
                          ? "系统素材保留"
                          : selectedAssetUsers.length
                            ? "被人物使用中"
                            : "至少保留一个素材"}
                      </span>
                    ) : null}
                  </div>
                  {pendingDeleteAssetId === selectedAsset.id ? (
                    <div className="ui-notice ui-notice-danger">
                      <p>确认彻底删除「{selectedAsset.name}」素材包？会删除 public/assets/skins 下对应文件夹。</p>
                      <div className="mt-2 grid grid-cols-2 gap-2">
                        <button
                          className="ui-button ui-button-secondary ui-button-danger"
                          type="button"
                          onClick={() => void removeSelectedAsset()}
                        >
                          确认删除
                        </button>
                        <button
                          className="ui-button ui-button-secondary"
                          type="button"
                          onClick={() => setPendingDeleteAssetId(null)}
                        >
                          取消
                        </button>
                      </div>
                    </div>
                  ) : null}
                </div>
              ) : null}

              {error ? <div role="alert" className="ui-notice ui-notice-danger">{error}</div> : null}

              <button
                className="ui-button ui-button-primary w-full"
                type="button"
                disabled={saving}
                onClick={() => void save()}
              >
                {saving ? "保存中..." : mode === "edit" ? "保存修改" : "创建人物"}
              </button>
            </div>
          </div>

          <details className="ui-card ui-details character-requirements">
            <summary>序列帧资源要求</summary>
            <div className="ui-details-body ui-hint">
            <p className="mt-1">
              必须包含 calm、sleeping、wake_up、yawn、sit、sit_down、happy、cheer_up、sad、angry、comfort、thinking、run_left、run_right、fly_up、fall_down、dizzy、error，以及工具箱按钮 box。
            </p>
            <p className="mt-1">
              直接拖入或选择角色资源文件夹即可，例如 rick_default。导入后会使用文件夹名作为资源目录名，只复制配置需要的 PNG 帧。
            </p>
            <p className="mt-1">
              还需要至少一个短动作目录，命名为 action1、action2、action3 等。每个 action 目录固定 6 张 PNG：action1_0001.png 到 action1_0006.png，超过 6 张会导入失败。
            </p>
            <p className="mt-1">
              其他动作按配置帧数放图：大多数为 6 张，run_left / run_right 为 4 张，sit_down 为 4 张，fall_down 为 4 张，dizzy 为 2 张，box 为 1 张 box_0001.png。所有 PNG 尺寸需一致，并带透明通道。
            </p>
            </div>
          </details>

          <div className="space-y-2">
            {characters.map((profile) => (
              <div key={profile.id} className="ui-card character-card">
                <div className="character-card-heading">
                  <div className="min-w-0">
                    <div className="truncate text-[12px] font-semibold text-slate-800">
                      {profile.name}
                      {profile.isActive ? <span className="ui-status ml-2">当前</span> : null}
                    </div>
                    <p className="mt-1 line-clamp-2 text-[11px] leading-relaxed text-slate-500">
                      {profile.prompt || profile.description || "暂无 prompt"}
                    </p>
                  </div>
                  <div className="flex shrink-0 gap-1">
                    <button className="ui-button ui-button-secondary ui-button-small" type="button" disabled={profile.isActive} onClick={() => void onSwitchCharacter(profile.id)}>
                      切换
                    </button>
                    <button className="ui-icon-button ui-icon-button-small" type="button" title="编辑" aria-label={`编辑 ${profile.name}`} onClick={() => editProfile(profile)}>
                      <Pencil size={13} />
                    </button>
                    <button className="ui-icon-button ui-icon-button-small ui-button-danger" type="button" title="删除" aria-label={`删除 ${profile.name}`} disabled={characters.length <= 1} onClick={() => void remove(profile)}>
                      <Trash2 size={13} />
                    </button>
                  </div>
                </div>
                <div className="character-scale">
                  <div className="mb-1 flex items-center justify-between text-[11px]">
                    <span className="font-semibold text-slate-600">角色大小</span>
                    <span className="ui-range-value">
                      {profile.isActive ? `${Math.round((profile.displayScale ?? 1) * 100)}%` : "切换后可调整"}
                    </span>
                  </div>
                  <RangeSlider
                    label={`${profile.name} 的角色大小`}
                    aria-valuetext={`${Math.round((profile.displayScale ?? 1) * 100)}%`}
                    min={MIN_CHARACTER_SCALE}
                    max={MAX_CHARACTER_SCALE}
                    step={CHARACTER_SCALE_STEP}
                    value={profile.displayScale ?? 1}
                    disabled={!profile.isActive}
                    onChange={(event) => updateScale(profile, Number(event.target.value))}
                    onPointerUp={() => scaleSaverRef.current?.flush(profile.id)}
                    onKeyUp={() => scaleSaverRef.current?.flush(profile.id)}
                    onBlur={() => scaleSaverRef.current?.flush(profile.id)}
                  />
                </div>
                {pendingDeleteId === profile.id ? (
                  <div className="ui-notice ui-notice-danger mt-2">
                    <p className="text-[11px] leading-relaxed text-rose-600">
                      确认删除「{profile.name}」？聊天记录和记忆也会一起删除。
                    </p>
                    <div className="mt-2 grid grid-cols-2 gap-2">
                      <button
                        className="ui-button ui-button-secondary ui-button-danger"
                        type="button"
                        onClick={() => void remove(profile)}
                      >
                        确认删除
                      </button>
                      <button
                        className="ui-button ui-button-secondary"
                        type="button"
                        onClick={() => setPendingDeleteId(null)}
                      >
                        取消
                      </button>
                    </div>
                  </div>
                ) : null}
              </div>
            ))}
          </div>
        </div>
      </div>
    </PanelShell>
  );
}
