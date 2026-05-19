import { useMemo } from "react";
import { useListCategories } from "@/api/generated/categories/categories";
import { Dropdown } from "@/components/ui/Dropdown";

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
  includeAllOption?: boolean;
}

export function CategorySelect({
  libraryId,
  value,
  onChange,
  required,
  includeAllOption,
}: CategorySelectProps) {
  const { data: categories, isLoading } = useListCategories(libraryId ?? "", {
    query: { enabled: !!libraryId },
  });

  const options = useMemo(() => {
    const names = new Set(categories?.map((c) => c.name) ?? []);
    if (value.trim()) names.add(value.trim());
    const sorted = sortCategories([...names]);
    const items = sorted.map((name) => ({ value: name, label: name }));
    if (includeAllOption) {
      return [{ value: "", label: "全部分类" }, ...items];
    }
    return items;
  }, [categories, value, includeAllOption]);

  if (!libraryId) {
    return <p className="muted-hint">请先在顶栏选择图书馆</p>;
  }

  return (
    <>
      <Dropdown
        value={value}
        onChange={onChange}
        options={options}
        placeholder={isLoading ? "加载分类…" : includeAllOption ? "全部分类" : "选择分类"}
        disabled={isLoading || (!includeAllOption && options.length === 0)}
        required={required}
        aria-label="分类"
      />
      {!isLoading && !includeAllOption && options.length === 0 ? (
        <span className="muted-hint">请先在图书馆管理中添加分类</span>
      ) : null}
    </>
  );
}
