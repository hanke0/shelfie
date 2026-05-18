import { useEffect, useId, useMemo, useRef, useState } from "react";
import {
  filterLanguageOptions,
  formatLanguageLabel,
  getLanguageOption,
  resolveLanguageCode,
} from "@/data/iso639-1";
import styles from "./LanguageCombobox.module.css";

interface LanguageComboboxProps {
  value: string;
  onChange: (code: string) => void;
  id?: string;
  placeholder?: string;
}

export function LanguageCombobox({
  value,
  onChange,
  id,
  placeholder = "搜索 ISO 639-1，如 zh、English、中文",
}: LanguageComboboxProps) {
  const listId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const inputValueRef = useRef("");
  const [open, setOpen] = useState(false);
  const [inputValue, setInputValue] = useState("");
  const [highlight, setHighlight] = useState(0);

  const displayForValue = (code: string) => {
    const opt = getLanguageOption(code);
    return opt ? formatLanguageLabel(opt) : code;
  };

  const setInput = (text: string) => {
    inputValueRef.current = text;
    setInputValue(text);
  };

  useEffect(() => {
    const label = value ? displayForValue(value) : "";
    setInput(label);
  }, [value]);

  const options = useMemo(
    () => (open ? filterLanguageOptions(inputValue) : []),
    [open, inputValue],
  );

  useEffect(() => {
    setHighlight(0);
  }, [inputValue, open]);

  const commitInput = (text: string) => {
    const resolved = resolveLanguageCode(text);
    if (resolved !== null) {
      onChange(resolved);
      setInput(resolved ? displayForValue(resolved) : "");
      return;
    }
    setInput(value ? displayForValue(value) : text);
  };

  const pick = (code: string) => {
    const label = displayForValue(code);
    onChange(code);
    setInput(label);
    setOpen(false);
  };

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) {
        commitInput(inputValueRef.current);
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open, value, onChange]);

  return (
    <div ref={rootRef} className={styles.root}>
      <input
        ref={inputRef}
        id={id}
        type="text"
        className={styles.input}
        role="combobox"
        aria-expanded={open}
        aria-controls={listId}
        aria-autocomplete="list"
        value={inputValue}
        placeholder={placeholder}
        onChange={(e) => {
          setInput(e.target.value);
          setOpen(true);
        }}
        onFocus={() => {
          setOpen(true);
          window.requestAnimationFrame(() => inputRef.current?.select());
        }}
        onBlur={() => {
          window.setTimeout(() => {
            if (!rootRef.current?.contains(document.activeElement)) {
              commitInput(inputValueRef.current);
              setOpen(false);
            }
          }, 0);
        }}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            setOpen(false);
            setInput(value ? displayForValue(value) : "");
            return;
          }
          if (!open) {
            if (e.key === "ArrowDown" || e.key === "Enter") setOpen(true);
            return;
          }
          if (e.key === "ArrowDown") {
            e.preventDefault();
            setHighlight((i) => Math.min(i + 1, Math.max(0, options.length - 1)));
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            setHighlight((i) => Math.max(i - 1, 0));
          } else if (e.key === "Enter" && options[highlight]) {
            e.preventDefault();
            pick(options[highlight].code);
            inputRef.current?.blur();
          }
        }}
      />
      {open && (
        <ul id={listId} role="listbox" className={styles.list}>
          {options.length === 0 ? (
            <li className={styles.empty}>无匹配语言</li>
          ) : (
            options.map((opt, index) => (
              <li
                key={opt.code}
                role="option"
                aria-selected={opt.code === value}
                className={[
                  styles.option,
                  index === highlight ? styles.optionHighlighted : "",
                  opt.code === value ? styles.optionSelected : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
                onMouseDown={(e) => {
                  e.preventDefault();
                  pick(opt.code);
                }}
                onMouseEnter={() => setHighlight(index)}
              >
                {formatLanguageLabel(opt)}
              </li>
            ))
          )}
        </ul>
      )}
    </div>
  );
}
