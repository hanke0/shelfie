import { act, render, renderHook, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { useApiAction } from "@/hooks/useApiAction";
import { ToastProvider, useToast } from "./ToastContext";

const wrapper = ({ children }: { children: ReactNode }) => <ToastProvider>{children}</ToastProvider>;

describe("ToastProvider", () => {
  it("shows error as alert and success as status", () => {
    const { result } = renderHook(() => useToast(), { wrapper });
    act(() => {
      result.current.error("出错了");
      result.current.success("已保存");
    });
    expect(screen.getByRole("alert")).toHaveTextContent("出错了");
    expect(screen.getByRole("status")).toHaveTextContent("已保存");
  });

  it("ignores blank messages", () => {
    const { result } = renderHook(() => useToast(), { wrapper });
    act(() => result.current.error("   "));
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("auto-dismisses after timeout", () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useToast(), { wrapper });
    act(() => result.current.success("稍后消失"));
    expect(screen.getByText("稍后消失")).toBeInTheDocument();
    act(() => vi.advanceTimersByTime(4500));
    expect(screen.queryByText("稍后消失")).toBeNull();
  });

  it("throws outside provider", () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => render(<Consumer />)).toThrow("useToast must be used within ToastProvider");
  });
});

function Consumer() {
  useToast();
  return null;
}

describe("useApiAction", () => {
  it("returns true and shows success message", async () => {
    const { result } = renderHook(() => useApiAction(), { wrapper });
    let ok = false;
    await act(async () => {
      ok = await result.current(async () => "x", { successMessage: "完成" });
    });
    expect(ok).toBe(true);
    expect(screen.getByRole("status")).toHaveTextContent("完成");
  });

  it("returns false and shows formatted error", async () => {
    const { result } = renderHook(() => useApiAction(), { wrapper });
    let ok = true;
    await act(async () => {
      ok = await result.current(() => Promise.reject(new Error("网络错误")));
    });
    expect(ok).toBe(false);
    expect(screen.getByRole("alert")).toHaveTextContent("网络错误");
  });

  it("uses errorMessage as fallback", async () => {
    const { result } = renderHook(() => useApiAction(), { wrapper });
    await act(async () => {
      await result.current(() => Promise.reject(null), { errorMessage: "删除失败" });
    });
    expect(screen.getByRole("alert")).toHaveTextContent("删除失败");
  });
});
