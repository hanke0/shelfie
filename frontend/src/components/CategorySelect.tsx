import { useMemo } from "react";
import { useListCategories } from "@/api/generated/categories/categories";
import { Select } from "@/components/ui/Select";

const DEFAULT_CATEGORY = "未分类";

function sortCategories(names: string[]): string[] {
  return [...names].sort((a, b) => {
    if (a === DEFAULT_CATEGORY) return -1;
    if (b === DEFAULT_CATEGORY) return 1;
    return a.localeCompare(b, "zh");
  });
}

interface CategorySelectProps {
  libraryId: string | null | undefined;
  value: string;
  onChange: (name: string) => void;
  required?: boolean;
}

export function CategorySelect({
  libraryId,
  value,
  onChange,
  required,
}: CategorySelectProps) {
  const { data: categories, isLoading } = useListCategories(libraryId ?? "", {
    query: { enabled: !!libraryId },
  });

  const options = useMemo(() => {
    const names = new Set(categories?.map((c) => c.name) ?? []);
    if (value.trim()) names.add(value.trim());
    return sortCategories([...names]);
  }, [categories, value]);

  if (!libraryId) {
    return <p className="muted-hint">请先在顶栏选择图书馆</p>;
  }

  return (
    <>
      <Select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        required={required}
        disabled={isLoading || options.length === 0}
      >
        <option value="">{isLoading ? "加载分类…" : "选择分类"}</option>
        {options.map((name) => (
          <option key={name} value={name}>
            {name}
          </option>
        ))}
      </Select>
      {!isLoading && options.length === 0 ? (
        <span className="muted-hint">请先在图书馆管理中添加分类</span>
      ) : null}
    </>
  );
}
