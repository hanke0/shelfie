import styles from "./RatingPicker.module.css";

interface RatingPickerProps {
  value?: number | null;
  onChange: (rating: number | undefined) => void;
  disabled?: boolean;
}

export function RatingPicker({ value, onChange, disabled }: RatingPickerProps) {
  const score = value && value >= 1 && value <= 5 ? value : undefined;

  return (
    <div className={styles.root} role="group" aria-label="评分 1 至 5 分">
      {[1, 2, 3, 4, 5].map((n) => {
        const filled = score !== undefined && n <= score;
        return (
          <button
            key={n}
            type="button"
            className={[styles.star, filled ? styles.starFilled : ""].filter(Boolean).join(" ")}
            disabled={disabled}
            aria-label={`${n} 分`}
            aria-pressed={score === n}
            onClick={() => onChange(score === n ? undefined : n)}
          >
            ★
          </button>
        );
      })}
      {score !== undefined && (
        <span className={styles.label}>{score} / 5</span>
      )}
    </div>
  );
}
