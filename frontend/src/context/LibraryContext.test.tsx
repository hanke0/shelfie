import { act, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { useListLibraries } from "@/api/generated/libraries/libraries";
import { setAuth } from "@/lib/auth";
import { LibraryProvider, useLibrary } from "./LibraryContext";

vi.mock("@/api/generated/libraries/libraries", () => ({ useListLibraries: vi.fn() }));

const mockedUseListLibraries = vi.mocked(useListLibraries);

function LibraryNames() {
  const { libraries } = useLibrary();
  return <p>{libraries.map((l) => l.name).join(",") || "无"}</p>;
}

describe("LibraryProvider", () => {
  it("starts fetching libraries once the user logs in, without a reload", () => {
    mockedUseListLibraries.mockImplementation((options) => {
      const enabled = options?.query?.enabled;
      return {
        data: enabled ? [{ id: "lib-1", name: "默认" }] : undefined,
        isLoading: false,
      } as unknown as ReturnType<typeof useListLibraries>;
    });

    render(
      <QueryClientProvider client={new QueryClient()}>
        <LibraryProvider>
          <LibraryNames />
        </LibraryProvider>
      </QueryClientProvider>,
    );
    expect(screen.getByText("无")).toBeInTheDocument();

    act(() => setAuth("tok", { id: "1", username: "a", role: "user" }));

    expect(screen.getByText("默认")).toBeInTheDocument();
  });
});
