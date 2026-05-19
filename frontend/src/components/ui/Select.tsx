import { Dropdown, type DropdownOption, type DropdownProps } from "./Dropdown";

/** @deprecated Prefer `Dropdown` with `options` prop. Kept for gradual migration. */
export type SelectProps = Omit<DropdownProps, "options"> & {
  options: DropdownOption[];
  onChange: (e: { target: { value: string } }) => void;
};

export function Select({ onChange, ...props }: SelectProps) {
  return (
    <Dropdown
      {...props}
      onChange={(value) => onChange({ target: { value } })}
    />
  );
}

export { Dropdown, type DropdownOption, type DropdownProps };
