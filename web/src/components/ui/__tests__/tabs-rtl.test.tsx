import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { Tabs } from "../tabs";

function Harness() {
  const [value, setValue] = useState<"a" | "b" | "c">("a");
  return <Tabs ariaLabel="demo" value={value} onChange={setValue} items={[{ value: "a", label: "A" }, { value: "b", label: "B" }, { value: "c", label: "C" }]} />;
}

describe("Tabs keyboard navigation follows the document direction", () => {
  afterEach(() => {
    document.dir = "";
  });

  it("ArrowRight moves forward in LTR", () => {
    render(<Harness />);
    fireEvent.keyDown(screen.getByRole("tab", { name: "A" }), { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: "B" })).toHaveAttribute("aria-selected", "true");
  });

  it("ArrowRight moves backward in RTL", () => {
    document.dir = "rtl";
    render(<Harness />);
    fireEvent.keyDown(screen.getByRole("tab", { name: "A" }), { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: "C" })).toHaveAttribute("aria-selected", "true");
  });
});
