import { useSyncExternalStore } from "react";

const TOKEN_KEY = "shelfie_token";
const USER_KEY = "shelfie_user";
const AUTH_CHANGED = "shelfie-auth-changed";

function notifyAuthChanged() {
  window.dispatchEvent(new Event(AUTH_CHANGED));
}

export interface StoredUser {
  id: string;
  username: string;
  role: string;
}

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function setAuth(token: string, user: StoredUser) {
  localStorage.setItem(TOKEN_KEY, token);
  localStorage.setItem(USER_KEY, JSON.stringify(user));
  notifyAuthChanged();
}

export function getUser(): StoredUser | null {
  const raw = localStorage.getItem(USER_KEY);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as StoredUser;
  } catch {
    return null;
  }
}

export function clearAuth() {
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(USER_KEY);
  notifyAuthChanged();
}

/** 同页登录/登出走自定义事件；其他标签页改动 localStorage 时触发 storage 事件 */
function subscribeAuth(onChange: () => void) {
  window.addEventListener(AUTH_CHANGED, onChange);
  window.addEventListener("storage", onChange);
  return () => {
    window.removeEventListener(AUTH_CHANGED, onChange);
    window.removeEventListener("storage", onChange);
  };
}

// useSyncExternalStore 要求快照在数据未变时引用稳定，按原始字符串缓存解析结果
let cachedUserRaw: string | null = null;
let cachedUser: StoredUser | null = null;

function getUserSnapshot(): StoredUser | null {
  const raw = localStorage.getItem(USER_KEY);
  if (raw !== cachedUserRaw) {
    cachedUserRaw = raw;
    cachedUser = getUser();
  }
  return cachedUser;
}

export function useStoredUser(): StoredUser | null {
  return useSyncExternalStore(subscribeAuth, getUserSnapshot);
}

export function isAuthenticated(): boolean {
  return !!getToken();
}

export function useIsAuthenticated(): boolean {
  return useSyncExternalStore(subscribeAuth, isAuthenticated);
}
