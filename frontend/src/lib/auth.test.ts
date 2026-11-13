import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { clearAuth, getToken, getUser, isAuthenticated, setAuth, useStoredUser } from "./auth";

const user = { id: "u1", username: "alice", role: "admin" };

describe("auth storage", () => {
  it("stores and clears token and user", () => {
    expect(isAuthenticated()).toBe(false);

    setAuth("tok", user);
    expect(getToken()).toBe("tok");
    expect(getUser()).toEqual(user);
    expect(isAuthenticated()).toBe(true);

    clearAuth();
    expect(getToken()).toBeNull();
    expect(getUser()).toBeNull();
  });

  it("returns null for corrupted user JSON", () => {
    localStorage.setItem("shelfie_user", "{oops");
    expect(getUser()).toBeNull();
  });

  it("dispatches auth-changed events", () => {
    const listener = vi.fn();
    window.addEventListener("shelfie-auth-changed", listener);
    setAuth("tok", user);
    clearAuth();
    window.removeEventListener("shelfie-auth-changed", listener);
    expect(listener).toHaveBeenCalledTimes(2);
  });
});

describe("useStoredUser", () => {
  it("re-renders on login and logout", () => {
    const { result } = renderHook(() => useStoredUser());
    expect(result.current).toBeNull();

    act(() => setAuth("tok", user));
    expect(result.current).toEqual(user);

    act(() => clearAuth());
    expect(result.current).toBeNull();
  });
});
