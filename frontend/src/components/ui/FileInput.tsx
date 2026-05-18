import { useEffect, useId, useRef, useState } from "react";
import styles from "./FileInput.module.css";

function fileMatchesAccept(file: File, accept: string): boolean {
  const tokens = accept
    .split(",")
    .map((s) => s.trim().toLowerCase())
    .filter(Boolean);
  if (tokens.length === 0) return true;

  const name = file.name.toLowerCase();
  const type = file.type.toLowerCase();

  return tokens.some((token) => {
    if (token.startsWith(".")) return name.endsWith(token);
    if (token.endsWith("/*")) return type.startsWith(token.slice(0, -1));
    return type === token;
  });
}

export interface FileInputProps {
  accept?: string;
  value?: File | null;
  onChange: (file: File | null) => void;
  required?: boolean;
  id?: string;
  name?: string;
  placeholder?: string;
  className?: string;
  variant?: "dropzone" | "link";
}

export function FileInput({
  accept,
  value = null,
  onChange,
  required,
  id: idProp,
  name,
  placeholder = "点击选择或拖拽文件到此处",
  className,
  variant = "dropzone",
}: FileInputProps) {
  const autoId = useId();
  const inputId = idProp ?? autoId;
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);
  const [rejectHint, setRejectHint] = useState<string | null>(null);

  useEffect(() => {
    const input = inputRef.current;
    if (!input) return;
    if (value) {
      const dt = new DataTransfer();
      dt.items.add(value);
      input.files = dt.files;
    } else {
      input.value = "";
    }
  }, [value]);

  const applyFile = (file: File | undefined) => {
    setRejectHint(null);
    if (!file) {
      onChange(null);
      return;
    }
    if (accept && !fileMatchesAccept(file, accept)) {
      setRejectHint("文件类型不符合要求");
      return;
    }
    onChange(file);
  };

  const onInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    applyFile(e.target.files?.[0]);
  };

  const onDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragging(true);
  };

  const onDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
    setDragging(true);
  };

  const onDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const next = e.relatedTarget as Node | null;
    if (next && e.currentTarget.contains(next)) return;
    setDragging(false);
  };

  const onDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragging(false);
    applyFile(e.dataTransfer.files?.[0]);
  };

  const zoneClass = [
    variant === "link" ? styles.link : styles.dropzone,
    dragging ? styles.dragging : "",
    value ? styles.hasFile : "",
    className ?? "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={styles.wrap}>
      <input
        ref={inputRef}
        id={inputId}
        name={name}
        type="file"
        className={styles.native}
        accept={accept}
        required={required}
        onChange={onInputChange}
        tabIndex={-1}
        aria-hidden={variant === "dropzone"}
      />
      <div
        role="button"
        tabIndex={0}
        className={zoneClass}
        aria-label={placeholder}
        onClick={() => inputRef.current?.click()}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            inputRef.current?.click();
          }
        }}
        onDragEnter={onDragEnter}
        onDragOver={onDragOver}
        onDragLeave={onDragLeave}
        onDrop={onDrop}
      >
        {value ? (
          <span className={styles.fileName}>{value.name}</span>
        ) : (
          <span className={styles.placeholder}>{placeholder}</span>
        )}
      </div>
      {rejectHint ? <span className={styles.rejectHint}>{rejectHint}</span> : null}
    </div>
  );
}
