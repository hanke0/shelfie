import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import styles from "./Dropdown.module.css";

export interface DropdownOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface DropdownProps {
  value: string;
  onChange: (value: string) => void;
  options: DropdownOption[];
  placeholder?: string;
  disabled?: boolean;
  required?: boolean;
  id?: string;
  className?: string;
  variant?: "form" | "header";
  icon?: ReactNode;
  "aria-label"?: string;
}

export function Dropdown({
  value,
  onChange,
  options,
  placeholder = "请选择",
  disabled = false,
  required = false,
  id: idProp,
  className,
  variant = "form",
  icon,
  "aria-label": ariaLabel,
}: DropdownProps) {
  const autoId = useId();
  const listId = useId();
  const triggerId = idProp ?? autoId;
  const rootRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [highlight, setHighlight] = useState(0);

  const enabledOptions = options.filter((o) => !o.disabled);
  const selectedIndex = Math.max(
    0,
    enabledOptions.findIndex((o) => o.value === value),
  );
  const selected = options.find((o) => o.value === value);
  const displayLabel = selected?.label ?? (value ? value : null);

  useEffect(() => {
    if (!open) return;
    setHighlight(selectedIndex >= 0 ? selectedIndex : 0);
  }, [open, selectedIndex]);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  const pick = (next: string) => {
    onChange(next);
    setOpen(false);
  };

  const toggle = () => {
    if (disabled || enabledOptions.length === 0) return;
    setOpen((v) => !v);
  };

  const rootClass = [
    styles.root,
    variant === "header" ? styles.rootHeader : "",
    className ?? "",
  ]
    .filter(Boolean)
    .join(" ");

  const shellClass = [
    styles.shell,
    variant === "header" ? styles.shellHeader : styles.shellForm,
    open ? styles.shellOpen : "",
    disabled ? styles.shellDisabled : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div ref={rootRef} className={rootClass}>
      {required ? (
        <input
          tabIndex={-1}
          aria-hidden
          className={styles.visuallyHidden}
          value={value}
          required
          readOnly
          onChange={() => {}}
        />
      ) : null}
      <div className={shellClass}>
        <button
          id={triggerId}
          type="button"
          className={styles.trigger}
          disabled={disabled || enabledOptions.length === 0}
          aria-haspopup="listbox"
          aria-expanded={open}
          aria-controls={listId}
          aria-label={ariaLabel}
          onClick={toggle}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              setOpen(false);
              return;
            }
            if (!open) {
              if (e.key === "ArrowDown" || e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                if (!disabled && enabledOptions.length > 0) setOpen(true);
              }
              return;
            }
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setHighlight((i) => Math.min(i + 1, enabledOptions.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setHighlight((i) => Math.max(i - 1, 0));
            } else if (e.key === "Enter" && enabledOptions[highlight]) {
              e.preventDefault();
              pick(enabledOptions[highlight].value);
            }
          }}
        >
          {icon ? <span className={styles.icon}>{icon}</span> : null}
          <span
            className={[styles.label, !displayLabel ? styles.placeholder : ""]
              .filter(Boolean)
              .join(" ")}
          >
            {displayLabel ?? placeholder}
          </span>
          <span
            className={[styles.chevron, open ? styles.chevronOpen : ""].filter(Boolean).join(" ")}
            aria-hidden
          >
            ▾
          </span>
        </button>
      </div>

      {open && enabledOptions.length > 0 ? (
        <ul id={listId} role="listbox" className={styles.menu} aria-labelledby={triggerId}>
          {options.map((opt) => {
            if (opt.disabled) {
              return (
                <li
                  key={opt.value || opt.label}
                  role="option"
                  aria-disabled
                  className={[styles.option, styles.optionDisabled].join(" ")}
                >
                  <span className={styles.optionLabel}>{opt.label}</span>
                </li>
              );
            }
            const enabledIndex = enabledOptions.findIndex((o) => o.value === opt.value);
            const isSelected = opt.value === value;
            return (
              <li
                key={opt.value}
                role="option"
                aria-selected={isSelected}
                className={[
                  styles.option,
                  enabledIndex === highlight ? styles.optionHighlighted : "",
                  isSelected ? styles.optionSelected : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
                onMouseDown={(e) => {
                  e.preventDefault();
                  pick(opt.value);
                }}
                onMouseEnter={() => {
                  if (enabledIndex >= 0) setHighlight(enabledIndex);
                }}
              >
                <span className={styles.optionLabel}>{opt.label}</span>
                {isSelected ? (
                  <span className={styles.check} aria-hidden>
                    ✓
                  </span>
                ) : null}
              </li>
            );
          })}
        </ul>
      ) : null}
    </div>
  );
}
