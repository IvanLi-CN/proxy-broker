import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { PropsWithChildren } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useTaskEvents } from "@/hooks/use-task-events";

const mockGetTaskEventsUrl = vi.fn((_: unknown) => "/api/v1/tasks/events");

vi.mock("@/lib/api", () => ({
  api: {
    getTaskEventsUrl: (query: unknown) => mockGetTaskEventsUrl(query),
  },
}));

class MockEventSource {
  static instances: MockEventSource[] = [];

  private listeners = new Map<string, Set<EventListener>>();
  readonly url: string;

  constructor(url: string) {
    this.url = url;
    MockEventSource.instances.push(this);
  }

  addEventListener(type: string, listener: EventListener) {
    const listeners = this.listeners.get(type) ?? new Set<EventListener>();
    listeners.add(listener);
    this.listeners.set(type, listeners);
  }

  removeEventListener(type: string, listener: EventListener) {
    this.listeners.get(type)?.delete(listener);
  }

  close() {}

  emit(type: string, data: string) {
    for (const listener of this.listeners.get(type) ?? []) {
      listener({ data } as MessageEvent<string>);
    }
  }
}

describe("useTaskEvents", () => {
  const originalEventSource = globalThis.EventSource;

  beforeEach(() => {
    MockEventSource.instances = [];
    mockGetTaskEventsUrl.mockClear();
    globalThis.EventSource = MockEventSource as unknown as typeof EventSource;
  });

  afterEach(() => {
    globalThis.EventSource = originalEventSource;
  });

  it("refetches active task detail queries after a snapshot arrives", async () => {
    const queryClient = new QueryClient();
    const refetchSpy = vi.spyOn(queryClient, "refetchQueries").mockResolvedValue(undefined);
    const wrapper = ({ children }: PropsWithChildren) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    renderHook(
      () =>
        useTaskEvents({
          query: {
            profile_id: "default",
            limit: 40,
          },
        }),
      { wrapper },
    );

    act(() => {
      MockEventSource.instances[0]?.emit(
        "snapshot",
        JSON.stringify({
          type: "snapshot",
          data: {
            summary: {
              total_runs: 1,
              queued_runs: 0,
              running_runs: 1,
              failed_runs: 0,
              succeeded_runs: 0,
              skipped_runs: 0,
              last_run_at: 1,
            },
            runs: [],
            next_cursor: null,
          },
        }),
      );
    });

    await waitFor(() =>
      expect(refetchSpy).toHaveBeenCalledWith({
        queryKey: ["task-run"],
        type: "active",
      }),
    );
  });
});
