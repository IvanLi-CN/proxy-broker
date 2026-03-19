import { render } from "@testing-library/react";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ApiError } from "@/lib/api";
import { RootRoute } from "@/routes/RootRoute";

const { mockToast, mockUseMutation, mockUseProfilePreference, mockUseQuery, mockUseQueryClient } =
  vi.hoisted(() => ({
    mockToast: {
      error: vi.fn(),
      info: vi.fn(),
      success: vi.fn(),
    },
    mockUseMutation: vi.fn(),
    mockUseProfilePreference: vi.fn(),
    mockUseQuery: vi.fn(),
    mockUseQueryClient: vi.fn(),
  }));

let latestAppShellProps: ComponentProps<typeof import("@/components/AppShell").AppShell> | null =
  null;

vi.mock("@tanstack/react-query", () => ({
  useMutation: () => mockUseMutation(),
  useQuery: () => mockUseQuery(),
  useQueryClient: () => mockUseQueryClient(),
}));

vi.mock("sonner", () => ({
  toast: mockToast,
}));

vi.mock("@/hooks/use-profile-preference", () => ({
  useProfilePreference: () => mockUseProfilePreference(),
}));

vi.mock("@/components/AppShell", () => ({
  AppShell: (props: ComponentProps<typeof import("@/components/AppShell").AppShell>) => {
    latestAppShellProps = props;
    return null;
  },
}));

vi.mock("react-router-dom", () => ({
  Outlet: () => null,
}));

describe("RootRoute", () => {
  beforeEach(() => {
    latestAppShellProps = null;
    mockUseMutation.mockReset();
    mockUseProfilePreference.mockReset();
    mockUseQuery.mockReset();
    mockUseQueryClient.mockReset();
    mockToast.error.mockReset();
    mockToast.info.mockReset();
    mockToast.success.mockReset();

    mockUseProfilePreference.mockReturnValue(["default", vi.fn()]);
    mockUseQueryClient.mockReturnValue({
      invalidateQueries: vi.fn().mockResolvedValue(undefined),
    });
    mockUseMutation.mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    });
  });

  it("keeps cached profiles visible when a background refetch fails", () => {
    mockUseQuery
      .mockReturnValueOnce({
        data: { status: "healthy" },
      })
      .mockReturnValueOnce({
        data: { profiles: ["default", "edge-jp"] },
        error: new ApiError(500, {
          code: "http_500",
          message: "Profiles temporarily unavailable",
        }),
        isError: true,
        isLoading: false,
        refetch: vi.fn(),
      });

    render(<RootRoute />);

    expect(latestAppShellProps?.profiles).toEqual(["default", "edge-jp"]);
    expect(latestAppShellProps?.profilesError).toBeNull();
  });

  it("keeps the active profile unchanged when create returns profile_exists", async () => {
    const setProfileId = vi.fn();
    const invalidateQueries = vi.fn().mockResolvedValue(undefined);
    const duplicateError = new ApiError(409, {
      code: "profile_exists",
      message: "Profile already exists",
    });

    mockUseProfilePreference.mockReturnValue(["default", setProfileId]);
    mockUseQueryClient.mockReturnValue({ invalidateQueries });
    mockUseQuery
      .mockReturnValueOnce({
        data: { status: "healthy" },
      })
      .mockReturnValueOnce({
        data: { profiles: ["default", "edge-jp"] },
        isError: false,
        isLoading: false,
        refetch: vi.fn(),
      });
    mockUseMutation.mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn().mockRejectedValue(duplicateError),
    });

    render(<RootRoute />);

    await expect(latestAppShellProps?.onCreateProfile("  edge-jp  ")).rejects.toBe(duplicateError);
    expect(setProfileId).not.toHaveBeenCalled();
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["profiles"] });
    expect(mockToast.info).toHaveBeenCalledWith(
      "Profile edge-jp already exists. Refreshing catalog.",
    );
    expect(mockToast.error).toHaveBeenCalledWith("profile_exists: Profile already exists");
  });
});
