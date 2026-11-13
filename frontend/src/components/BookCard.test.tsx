import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { BookCard as BookCardType } from "@/api/generated/models";
import { fetchCoverBlob } from "@/lib/custom-fetch";
import { BookCard } from "./BookCard";

vi.mock("@/lib/custom-fetch", () => ({ fetchCoverBlob: vi.fn() }));
vi.mock("@/lib/title-cover", () => ({
  generateTitleCoverBlobUrl: vi.fn(() => Promise.reject(new Error("no canvas"))),
}));

const fetchCover = vi.mocked(fetchCoverBlob);

const book = {
  id: "b1",
  title: "三体",
  author: "刘慈欣",
  reading_percent: 0,
} as BookCardType;

function renderCard(props: Partial<Parameters<typeof BookCard>[0]> = {}) {
  return render(
    <MemoryRouter>
      <BookCard book={book} {...props} />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  fetchCover.mockReset();
});

describe("BookCard", () => {
  it("links to detail page and shows server cover", async () => {
    fetchCover.mockResolvedValue("blob:cover");
    renderCard();

    expect(screen.getByRole("link")).toHaveAttribute("href", "/books/b1");
    expect(screen.getByRole("heading", { name: "三体" })).toBeInTheDocument();
    expect(screen.getByText("刘慈欣")).toBeInTheDocument();
    expect(await screen.findByRole("img", { name: "三体" })).toHaveAttribute("src", "blob:cover");
    expect(fetchCover).toHaveBeenCalledWith("b1", { thumbnail: true });
  });

  it("falls back to generated cover and unknown author", async () => {
    fetchCover.mockRejectedValue(new Error("404"));
    renderCard({ book: { ...book, author: "" } });

    expect(screen.getByText("未知作者")).toBeInTheDocument();
    expect(await screen.findByText("三")).toBeInTheDocument();
    expect(screen.queryByRole("img")).toBeNull();
  });

  it("renders progress bar capped at 100%", () => {
    fetchCover.mockReturnValue(new Promise(() => {}));
    const { container } = renderCard({ book: { ...book, reading_percent: 150 } });
    const bar = container.querySelector(".progress > div") as HTMLElement;
    expect(bar.style.width).toBe("100%");
  });

  it("acts as a toggle button in selectable mode", async () => {
    fetchCover.mockReturnValue(new Promise(() => {}));
    const onSelectToggle = vi.fn();
    renderCard({ selectable: true, selected: true, onSelectToggle });

    expect(screen.queryByRole("link")).toBeNull();
    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("checkbox")).toHaveAttribute("aria-checked", "true");

    await userEvent.click(button);
    expect(onSelectToggle).toHaveBeenCalledOnce();
  });
});
