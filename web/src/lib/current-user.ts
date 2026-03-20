import { ApiError } from "@/lib/api";
import type { AuthMeResponse, CurrentUserState } from "@/lib/types";

export function resolveCurrentUserState({
  identity,
  isLoading = false,
  error = null,
}: {
  identity?: AuthMeResponse | null;
  isLoading?: boolean;
  error?: unknown;
}): CurrentUserState {
  if (identity?.authenticated) {
    return {
      status: "resolved",
      identity,
    };
  }

  if (isLoading) {
    return { status: "loading" };
  }

  if (error instanceof ApiError && error.status === 401) {
    return { status: "anonymous" };
  }

  if (error instanceof ApiError) {
    return {
      status: "error",
      message: `${error.code}: ${error.message}`,
    };
  }

  if (error instanceof Error) {
    return {
      status: "error",
      message: error.message,
    };
  }

  return { status: "anonymous" };
}
