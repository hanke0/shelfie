import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useSetKoreaderLink } from "@/api/generated/koreader/koreader";
import { useSearch } from "@/api/generated/search/search";
import { Modal } from "@/components/ui/Modal";
import { FieldLabel } from "@/components/ui/FieldLabel";
import formStyles from "@/components/ui/Form.module.css";
import { useApiAction } from "@/hooks/useApiAction";
import adminStyles from "@/pages/AdminPage.module.css";

interface KoreaderLinkModalProps {
  open: boolean;
  onClose: () => void;
  libraryFilter: string;
}

export function KoreaderLinkModal({ open, onClose, libraryFilter }: KoreaderLinkModalProps) {
  const qc = useQueryClient();
  const setLink = useSetKoreaderLink();
  const run = useApiAction();
  const [documentId, setDocumentId] = useState("");
  const [bookSearch, setBookSearch] = useState("");
  const [selectedBookId, setSelectedBookId] = useState("");
  const [error, setError] = useState<string | null>(null);

  const { data: searchResults } = useSearch(
    { q: bookSearch, library_id: libraryFilter || undefined, limit: 20 },
    { query: { enabled: open && bookSearch.trim().length >= 1 } },
  );

  const reset = () => {
    setDocumentId("");
    setBookSearch("");
    setSelectedBookId("");
    setError(null);
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!documentId.trim() || !selectedBookId) {
      setError("请填写 document 并选择图书");
      return;
    }
    setError(null);
    const ok = await run(
      async () => {
        await setLink.mutateAsync({
          data: { document: documentId.trim(), book_id: selectedBookId },
        });
        await qc.invalidateQueries({ queryKey: ["/koreader/links"] });
        await qc.invalidateQueries({ queryKey: ["/koreader/progress"] });
      },
      { successMessage: "匹配已保存", errorMessage: "保存失败" },
    );
    if (ok) handleClose();
  };

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title="手动匹配 document → 图书"
      wide
      footer={
        <>
          <button type="button" className="btn btn-ghost" onClick={handleClose}>
            取消
          </button>
          <button
            type="submit"
            form="koreader-link-form"
            className="btn"
            disabled={!selectedBookId || setLink.isPending}
          >
            {setLink.isPending ? "保存中…" : "保存匹配"}
          </button>
        </>
      }
    >
      <form id="koreader-link-form" className={formStyles.form} onSubmit={(e) => void handleSubmit(e)}>
        {error && <p className={formStyles.error}>{error}</p>}
        <label className={formStyles.field}>
          <FieldLabel required>KOReader document（32 位 MD5）</FieldLabel>
          <input
            value={documentId}
            onChange={(e) => setDocumentId(e.target.value)}
            placeholder="41cce710f34e5ec21315e19c99821415"
            required
          />
        </label>
        <label className={formStyles.field}>
          <FieldLabel>搜索图书</FieldLabel>
          <input
            type="search"
            value={bookSearch}
            onChange={(e) => {
              setBookSearch(e.target.value);
              setSelectedBookId("");
            }}
            placeholder="书名或作者"
          />
        </label>
        {searchResults && searchResults.length > 0 && (
          <ul className={adminStyles.list}>
            {searchResults.map((b) => (
              <li key={b.id}>
                <button
                  type="button"
                  className={selectedBookId === b.id ? adminStyles.active : ""}
                  onClick={() => setSelectedBookId(b.id)}
                >
                  {b.title} — {b.author || "未知作者"}
                </button>
              </li>
            ))}
          </ul>
        )}
        {selectedBookId && (
          <p className={formStyles.hint}>已选图书 ID: {selectedBookId}</p>
        )}
      </form>
    </Modal>
  );
}
