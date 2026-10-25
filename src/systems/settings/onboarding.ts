const ONBOARDING_KEY = "bytepet:onboarding:v1:completed";

export function hasCompletedOnboarding(storage?: Pick<Storage, "getItem">): boolean {
  try {
    const source = storage ?? (typeof localStorage === "undefined" ? undefined : localStorage);
    return source?.getItem(ONBOARDING_KEY) === "true";
  } catch { return false; }
}

export function completeOnboarding(storage?: Pick<Storage, "setItem">): void {
  try {
    const target = storage ?? (typeof localStorage === "undefined" ? undefined : localStorage);
    target?.setItem(ONBOARDING_KEY, "true");
  } catch { /* A blocked preference store must not prevent starting the app. */ }
}
