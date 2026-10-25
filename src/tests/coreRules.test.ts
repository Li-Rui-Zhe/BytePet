import { describe, expect, it } from "vitest";
import { validateSkinManifest } from "../systems/skin/skinValidator";
import { createRuntimeState, resolveExpiredState, transitionTo } from "../systems/stateMachine/runtime";
import { getAnimationState } from "../systems/stateMachine/stateDefinitions";
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import animationStates from "../config/animationStates.generated.json";

describe("state machine rules", () => {
  it("lets higher priority states interrupt lower priority states", () => {
    const calm = createRuntimeState("calm", 1000);
    const next = transitionTo(calm, "happy", 1100);
    expect(next.definition.key).toBe("happy");
  });

  it("keeps locked thinking from being interrupted by happy", () => {
    const thinking = createRuntimeState("thinking", 1000);
    const next = transitionTo(thinking, "happy", 1200);
    expect(next.definition.key).toBe("thinking");
  });

  it("returns one-shot animation to fallback after it expires", () => {
    const happy = createRuntimeState(getAnimationState("happy"), 1000);
    const resolved = resolveExpiredState(happy, "calm", 8000);
    expect(resolved.definition.key).toBe("calm");
  });
});

describe("skin rules", () => {
  it("ships exactly the configured frames without obsolete default assets", () => {
    const root = fileURLToPath(new URL("../../public/assets/skins/rick_default/", import.meta.url));
    const expected = new Map(animationStates.states.map(state => [state.key, state.frames]));
    expected.set("box", 1);
    const directories = readdirSync(root, { withFileTypes: true }).filter(entry => entry.isDirectory());
    const actions = directories.filter(entry => /^action\d+$/i.test(entry.name));
    expect(actions.length).toBeGreaterThan(0);
    for (const action of actions) expected.set(action.name, 6);
    expect(directories.map(entry => entry.name).sort()).toEqual([...expected.keys()].sort());
    for (const [key, count] of expected) {
      const filenames = Array.from({ length: count }, (_, i) => `${key}_${String(i + 1).padStart(4, "0")}.png`);
      expect(readdirSync(join(root, key)).sort(), key).toEqual(filenames);
    }
  });

  it("blocks skin import when manifest is missing", () => {
    const report = validateSkinManifest(null);
    expect(report.valid).toBe(false);
    expect(report.issues[0]?.code).toBe("ASSET_MANIFEST_MISSING");
  });

});
