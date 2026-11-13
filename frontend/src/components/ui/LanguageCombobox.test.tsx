import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { formatLanguageLabel, getLanguageOption } from "@/data/iso639-1";
import { LanguageCombobox } from "./LanguageCombobox";

const label = (code: string) => formatLanguageLabel(getLanguageOption(code)!);

function Harness({ initial = "", onChange }: { initial?: string; onChange?: (c: string) => void }) {
  const [value, setValue] = useState(initial);
  return (
    <>
      <LanguageCombobox
        value={value}
        onChange={(code) => {
          setValue(code);
          onChange?.(code);
        }}
      />
      <button type="button">outside</button>
    </>
  );
}

describe("LanguageCombobox", () => {
  it("shows label for the current value", () => {
    render(<Harness initial="en" />);
    expect(screen.getByRole("combobox")).toHaveValue(label("en"));
  });

  it("filters options and picks with keyboard", async () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);
    const input = screen.getByRole("combobox");

    await userEvent.type(input, "english");
    expect(input).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("option", { name: label("en") })).toBeInTheDocument();

    await userEvent.keyboard("{Enter}");
    expect(onChange).toHaveBeenCalledWith("en");
    expect(input).toHaveValue(label("en"));
  });

  it("picks an option with the mouse", async () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);
    await userEvent.type(screen.getByRole("combobox"), "fr");
    await userEvent.click(screen.getByRole("option", { name: label("fr") }));
    expect(onChange).toHaveBeenCalledWith("fr");
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("shows empty state for no matches", async () => {
    render(<Harness />);
    await userEvent.type(screen.getByRole("combobox"), "zzzz");
    expect(screen.getByText("无匹配语言")).toBeInTheDocument();
  });

  it("Escape restores the committed value", async () => {
    render(<Harness initial="en" />);
    const input = screen.getByRole("combobox");
    await userEvent.clear(input);
    await userEvent.type(input, "xyz{Escape}");
    expect(input).toHaveValue(label("en"));
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("commits typed code on outside click and reverts unknown text", async () => {
    const onChange = vi.fn();
    render(<Harness initial="en" onChange={onChange} />);
    const input = screen.getByRole("combobox");

    await userEvent.clear(input);
    await userEvent.type(input, "de");
    await userEvent.click(screen.getByRole("button", { name: "outside" }));
    expect(onChange).toHaveBeenLastCalledWith("de");
    expect(input).toHaveValue(label("de"));

    await userEvent.clear(input);
    await userEvent.type(input, "klingon");
    await userEvent.click(screen.getByRole("button", { name: "outside" }));
    expect(new Set(onChange.mock.calls.flat())).toEqual(new Set(["de"]));
    expect(input).toHaveValue(label("de"));
  });
});
