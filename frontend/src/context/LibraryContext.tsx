import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { useListLibraries } from "@/api/generated/libraries/libraries";
import { isAuthenticated } from "@/lib/auth";
import type { LibraryDto } from "@/api/generated/models";

const STORAGE_KEY = "shelfie_current_library_id";

interface LibraryContextValue {
  libraries: LibraryDto[];
  libraryId: string | null;
  library: LibraryDto | null;
  setLibraryId: (id: string | null) => void;
  isLoading: boolean;
}

const LibraryContext = createContext<LibraryContextValue | null>(null);

export function LibraryProvider({ children }: { children: ReactNode }) {
  const { data: libraries = [], isLoading } = useListLibraries({
    query: { enabled: isAuthenticated() },
  });
  const [libraryId, setLibraryIdState] = useState<string | null>(() =>
    localStorage.getItem(STORAGE_KEY),
  );

  const setLibraryId = useCallback((id: string | null) => {
    setLibraryIdState(id);
    if (id) {
      localStorage.setItem(STORAGE_KEY, id);
    } else {
      localStorage.removeItem(STORAGE_KEY);
    }
  }, []);

  useEffect(() => {
    if (isLoading || libraries.length === 0) return;

    const exists = libraryId && libraries.some((l) => l.id === libraryId);
    if (!exists) {
      setLibraryId(libraries[0].id);
    }
  }, [isLoading, libraries, libraryId, setLibraryId]);

  const library = useMemo(
    () => libraries.find((l) => l.id === libraryId) ?? null,
    [libraries, libraryId],
  );

  const value = useMemo(
    () => ({
      libraries,
      libraryId,
      library,
      setLibraryId,
      isLoading,
    }),
    [libraries, libraryId, library, setLibraryId, isLoading],
  );

  return <LibraryContext.Provider value={value}>{children}</LibraryContext.Provider>;
}

export function useLibrary() {
  const ctx = useContext(LibraryContext);
  if (!ctx) {
    throw new Error("useLibrary must be used within LibraryProvider");
  }
  return ctx;
}

/** 需要已选图书馆时使用；未选则返回 null */
export function useRequiredLibraryId(): string | null {
  const { libraryId, isLoading } = useLibrary();
  if (isLoading) return null;
  return libraryId;
}
