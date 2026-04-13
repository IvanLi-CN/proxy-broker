export const GLOBAL_PROFILE_ID = "__global__";
export const DEFAULT_PROFILE_ID = "default";

export function isGlobalProfileId(value: string) {
  return value === GLOBAL_PROFILE_ID;
}
