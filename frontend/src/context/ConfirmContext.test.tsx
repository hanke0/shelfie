import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { ConfirmProvider, useConfirmTwice } from "./ConfirmContext";

function Trigger({ onResult }: { onResult: (v: boolean) => void }) {
  const confirmTwice = useConfirmTwice();
  return (
    <button type="button" onClick={() => confirmTwice("删除这本书？").then(onResult)}>
      删除
    </button>
  );
}

function setup() {
  const results: boolean[] = [];
  render(
    <ConfirmProvider>
      <Trigger onResult={(v) => results.push(v)} />
    </ConfirmProvider>,
  );
  return results;
}

const click = (name: string) => userEvent.click(screen.getByRole("button", { name }));

describe("ConfirmProvider", () => {
  it("resolves true after two confirmations", async () => {
    const results = setup();
    await click("删除");
    expect(screen.getByRole("dialog", { name: "确认操作" })).toHaveTextContent("删除这本书？");

    await click("继续");
    expect(screen.getByRole("dialog", { name: "再次确认" })).toHaveTextContent("此操作无法撤销");

    await click("确定");
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(results).toEqual([true]);
  });

  it("resolves false when cancelled at step 1", async () => {
    const results = setup();
    await click("删除");
    await click("取消");
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(results).toEqual([false]);
  });

  it("goes back from step 2 to step 1", async () => {
    const results = setup();
    await click("删除");
    await click("继续");
    await click("返回上一步");
    expect(screen.getByRole("dialog", { name: "确认操作" })).toBeInTheDocument();
    expect(results).toEqual([]);
  });

  it("resolves false when closed via ×", async () => {
    const results = setup();
    await click("删除");
    await click("继续");
    await click("关闭");
    expect(results).toEqual([false]);
  });
});
