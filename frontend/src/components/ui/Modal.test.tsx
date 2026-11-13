import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Modal } from "./Modal";

describe("Modal", () => {
  it("renders nothing when closed", () => {
    render(
      <Modal open={false} onClose={() => {}} title="标题">
        内容
      </Modal>,
    );
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("renders title, body and footer when open", () => {
    render(
      <Modal open onClose={() => {}} title="标题" footer={<button>保存</button>}>
        内容
      </Modal>,
    );
    const dialog = screen.getByRole("dialog", { name: "标题" });
    expect(dialog).toHaveTextContent("内容");
    expect(screen.getByRole("button", { name: "保存" })).toBeInTheDocument();
  });

  it("calls onClose from the close button", async () => {
    const onClose = vi.fn();
    render(
      <Modal open onClose={onClose} title="标题">
        内容
      </Modal>,
    );
    await userEvent.click(screen.getByRole("button", { name: "关闭" }));
    expect(onClose).toHaveBeenCalledOnce();
  });
});
