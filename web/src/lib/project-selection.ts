export const GLOBAL_PROJECT_ID = "__global__";
export const DEFAULT_PROJECT_ID = "default";

export function isGlobalProjectId(value: string) {
  return value === GLOBAL_PROJECT_ID;
}
