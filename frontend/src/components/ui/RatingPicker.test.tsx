import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { RatingPicker } from "./RatingPicker";

describe("RatingPicker", () => {
  it("shows current score", () => {
    render(<RatingPicker value={3} onChange={() => {}} />);
    expect(screen.getByRole("button", { name: "3 分" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("3 / 5")).toBeInTheDocument();
  });

  it("ignores out-of-range values", () => {
    render(<RatingPicker value={7} onChange={() => {}} />);
    expect(screen.queryByText(/\/ 5/)).toBeNull();
  });

  it("selects a star and toggles off the same star", async () => {
    const onChange = vi.fn();
    const { rerender } = render(<RatingPicker onChange={onChange} />);
    await userEvent.click(screen.getByRole("button", { name: "4 分" }));
    expect(onChange).toHaveBeenLastCalledWith(4);

    rerender(<RatingPicker value={4} onChange={onChange} />);
    await userEvent.click(screen.getByRole("button", { name: "4 分" }));
    expect(onChange).toHaveBeenLastCalledWith(undefined);
  });

  it("disables all stars", () => {
    render(<RatingPicker disabled onChange={() => {}} />);
    for (const btn of screen.getAllByRole("button")) {
      expect(btn).toBeDisabled();
    }
  });
});
