import { act, render } from "@testing-library/react";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { nodesFixture } from "@/mocks/fixtures";
import { NodesRoute } from "@/routes/NodesRoute";

const { mockApi, mockOutletContext, mockToast, mockUseMutation, mockUseQuery, mockUseQueryClient } =
  vi.hoisted(() => ({
    mockApi: {
      exportNodes: vi.fn(),
      openNodeSessions: vi.fn(),
      queryNodes: vi.fn(),
    },
    mockOutletContext: vi.fn(),
    mockToast: {
      error: vi.fn(),
      success: vi.fn(),
    },
    mockUseMutation: vi.fn(),
    mockUseQuery: vi.fn(),
    mockUseQueryClient: vi.fn(),
  }));

let latestNodesPageProps: ComponentProps<typeof import("@/pages/NodesPage").NodesPage> | null =
  null;

vi.mock("@tanstack/react-query", () => ({
  useMutation: (options: unknown) => mockUseMutation(options),
  useQuery: () => mockUseQuery(),
  useQueryClient: () => mockUseQueryClient(),
}));

vi.mock("react-router-dom", () => ({
  useOutletContext: () => mockOutletContext(),
}));

vi.mock("sonner", () => ({
  toast: mockToast,
}));

vi.mock("@/lib/api", () => ({
  api: mockApi,
}));

vi.mock("@/pages/NodesPage", () => ({
  NodesPage: (props: ComponentProps<typeof import("@/pages/NodesPage").NodesPage>) => {
    latestNodesPageProps = props;
    return null;
  },
}));

describe("NodesRoute", () => {
  beforeEach(() => {
    latestNodesPageProps = null;
    mockApi.exportNodes.mockReset();
    mockApi.openNodeSessions.mockReset();
    mockApi.queryNodes.mockReset();
    mockOutletContext.mockReset();
    mockToast.error.mockReset();
    mockToast.success.mockReset();
    mockUseMutation.mockReset();
    mockUseQuery.mockReset();
    mockUseQueryClient.mockReset();

    mockOutletContext.mockReturnValue({ profileId: "default" });
    mockUseQueryClient.mockReturnValue({
      invalidateQueries: vi.fn().mockResolvedValue(undefined),
    });
    mockUseQuery.mockReturnValue({
      data: nodesFixture,
      error: null,
      isError: false,
      isFetching: false,
      isLoading: false,
    });
    mockUseMutation.mockImplementation(
      (options: {
        mutationFn: (...args: unknown[]) => Promise<unknown>;
        onSuccess?: (data: unknown) => unknown;
        onError?: (error: unknown) => unknown;
      }) => ({
        isPending: false,
        mutateAsync: async (...args: unknown[]) => {
          try {
            const result = await options.mutationFn(...args);
            await options.onSuccess?.(result);
            return result;
          } catch (error) {
            options.onError?.(error);
            throw error;
          }
        },
      }),
    );
  });

  it("revokes export blob URLs asynchronously after triggering the download", async () => {
    vi.useFakeTimers();
    const createObjectURL = vi.fn(() => "blob:test-nodes");
    const revokeObjectURL = vi.fn();
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});

    mockApi.exportNodes.mockResolvedValue("node-link");
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: createObjectURL,
    });
    Object.defineProperty(URL, "revokeObjectURL", {
      configurable: true,
      value: revokeObjectURL,
    });

    try {
      render(<NodesRoute />);

      await act(async () => {
        await latestNodesPageProps?.onExport("link_lines");
      });

      expect(mockApi.exportNodes).toHaveBeenCalled();
      expect(createObjectURL).toHaveBeenCalled();
      expect(clickSpy).toHaveBeenCalled();
      expect(revokeObjectURL).not.toHaveBeenCalled();

      await act(async () => {
        vi.runAllTimers();
      });

      expect(revokeObjectURL).toHaveBeenCalledWith("blob:test-nodes");
    } finally {
      clickSpy.mockRestore();
      vi.useRealTimers();
    }
  });

  it("clears explicit node selection when semantic filters change", async () => {
    render(<NodesRoute />);

    await act(async () => {
      latestNodesPageProps?.onToggleSelect("JP-Tokyo-Entry", true);
    });
    expect(latestNodesPageProps?.selectedIds).toEqual(["JP-Tokyo-Entry"]);

    await act(async () => {
      latestNodesPageProps?.onFilterChange({ query: "tokyo", page: 1 });
    });
    expect(latestNodesPageProps?.selectedIds).toEqual([]);
  });

  it("preserves explicit node selection for pure pagination changes", async () => {
    render(<NodesRoute />);

    await act(async () => {
      latestNodesPageProps?.onToggleSelect("JP-Tokyo-Entry", true);
    });
    expect(latestNodesPageProps?.selectedIds).toEqual(["JP-Tokyo-Entry"]);

    await act(async () => {
      latestNodesPageProps?.onFilterChange({ page: 2 });
    });
    expect(latestNodesPageProps?.selectedIds).toEqual(["JP-Tokyo-Entry"]);
  });
});
