import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useLibrary } from "@/context/LibraryContext";
import { setAuth } from "@/lib/auth";
import { ProtectedRoute } from "./ProtectedRoute";

vi.mock("@/context/LibraryContext", () => ({ useLibrary: vi.fn() }));

const mockedUseLibrary = vi.mocked(useLibrary);

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/login" element={<p>登录页</p>} />
        <Route
          path="/"
          element={
            <ProtectedRoute>
              <p>首页内容</p>
            </ProtectedRoute>
          }
        />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  mockedUseLibrary.mockReturnValue({ isReady: true } as ReturnType<typeof useLibrary>);
});

describe("ProtectedRoute", () => {
  it("redirects to /login without token", () => {
    renderAt("/");
    expect(screen.getByText("登录页")).toBeInTheDocument();
    expect(screen.queryByText("首页内容")).toBeNull();
  });

  it("shows loading until library is ready", () => {
    setAuth("tok", { id: "1", username: "a", role: "user" });
    mockedUseLibrary.mockReturnValue({ isReady: false } as ReturnType<typeof useLibrary>);
    renderAt("/");
    expect(screen.getByText("正在加载图书馆…")).toBeInTheDocument();
    expect(screen.queryByText("首页内容")).toBeNull();
  });

  it("renders children when authenticated and ready", () => {
    setAuth("tok", { id: "1", username: "a", role: "user" });
    renderAt("/");
    expect(screen.getByText("首页内容")).toBeInTheDocument();
  });
});
