import type { SkinValidationIssue, SkinValidationReport } from "../../types/domain";

export interface SkinManifestState {
  key: string;
  frames: number;
  fps: number;
}

export interface SkinManifest {
  skinId: string;
  name: string;
  frameWidth: number;
  frameHeight: number;
  states: SkinManifestState[];
}

export function validateSkinManifest(manifest: Partial<SkinManifest> | null): SkinValidationReport {
  const issues: SkinValidationIssue[] = [];

  if (!manifest) {
    return {
      valid: false,
      issues: [{ severity: "P0", code: "ASSET_MANIFEST_MISSING", message: "缺少 manifest.json。" }]
    };
  }

  if (!manifest.skinId || !manifest.name) {
    issues.push({ severity: "P0", code: "ASSET_MANIFEST_INVALID", message: "manifest 缺少 skinId 或 name。" });
  }

  if (!Number.isFinite(manifest.frameWidth) || !Number.isFinite(manifest.frameHeight)) {
    issues.push({ severity: "P0", code: "ASSET_SIZE_MISSING", message: "manifest 必须声明 frameWidth 与 frameHeight。" });
  }

  const requiredStates = [
    "calm",
    "sleeping",
    "wake_up",
    "yawn",
    "sit",
    "sit_down",
    "happy",
    "cheer_up",
    "sad",
    "angry",
    "comfort",
    "thinking",
    "run_left",
    "run_right",
    "fly_up",
    "fall_down",
    "dizzy",
    "error",
  ];
  const providedKeys = new Set((manifest.states ?? []).map((state) => state.key));

  for (const stateKey of requiredStates) {
    if (!providedKeys.has(stateKey)) {
      issues.push({
        severity: "P0",
        code: "ASSET_DIR_MISSING",
        message: `缺少必要状态 ${stateKey}。`,
        stateKey
      });
    }
  }

  if (!(manifest.states ?? []).some((state) => /^action\d+$/i.test(state.key))) {
    issues.push({
      severity: "P0",
      code: "ASSET_ACTION_MISSING",
      message: "至少需要一个短动作状态，例如 action1、action2 或 action3。",
    });
  }

  return {
    valid: !issues.some((issue) => issue.severity === "P0"),
    issues,
    frameWidth: manifest.frameWidth,
    frameHeight: manifest.frameHeight
  };
}
