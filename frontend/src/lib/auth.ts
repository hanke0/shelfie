import { useEffect, useState } from "react";

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

export function useStoredUser(): StoredUser | null {
  const [user, setUser] = useState<StoredUser | null>(() => getUser());
  useEffect(() => {
    const sync = () => setUser(getUser());
    window.addEventListener(AUTH_CHANGED, sync);
    return () => window.removeEventListener(AUTH_CHANGED, sync);
  }, []);
  return user;
}

export function isAuthenticated(): boolean {
  return !!getToken();
}
