// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, test, vi, type Mock } from "vitest";
import { onDismiss } from "./dismiss";

describe("onDismiss", () => {
  let panel: HTMLDivElement;
  let input: HTMLInputElement;
  let row: HTMLButtonElement;
  let close: Mock<() => void>;
  let stop: () => void;

  const fire = (el: EventTarget, type: string, init: MouseEventInit & KeyboardEventInit = {}) =>
    el.dispatchEvent(type === "keydown" ? new KeyboardEvent(type, { bubbles: true, ...init }) : new MouseEvent(type, { bubbles: true, cancelable: true, ...init }));

  beforeEach(() => {
    document.body.innerHTML = "";
    panel = document.createElement("div");
    input = document.createElement("input");
    panel.append(input);
    row = document.createElement("button");
    document.body.append(panel, row);
    close = vi.fn<() => void>();
    stop = onDismiss(() => panel, close);
  });

  afterEach(() => stop());

  test("a click outside closes and does not also activate what was clicked", () => {
    close.mockImplementation(() => stop()); // closing unmounts the panel, which removes its listeners
    const opened = vi.fn();
    row.addEventListener("click", opened);
    fire(row, "mousedown");
    fire(row, "mouseup");
    fire(row, "click");
    expect(close).toHaveBeenCalledTimes(1);
    expect(opened).not.toHaveBeenCalled();
  });

  test("only the dismissing click is swallowed", async () => {
    const opened = vi.fn();
    row.addEventListener("click", opened);
    fire(row, "mousedown");
    fire(row, "mouseup");
    await new Promise((r) => setTimeout(r, 0)); // e.g. a drag that ended without a click
    fire(row, "click");
    expect(opened).toHaveBeenCalledTimes(1);
  });

  test("clicks inside the panel neither close it nor get swallowed", () => {
    const pressed = vi.fn();
    input.addEventListener("click", pressed);
    fire(input, "mousedown");
    fire(input, "mouseup");
    fire(input, "click");
    expect(close).not.toHaveBeenCalled();
    expect(pressed).toHaveBeenCalled();
  });

  test("scroll events (a long name scrolling its input, the page shrinking) don't close it", () => {
    fire(input, "scroll");
    fire(document, "scroll");
    expect(close).not.toHaveBeenCalled();
  });

  test("scrolling the wheel outside closes; inside (a long group list) does not", () => {
    fire(input, "wheel");
    expect(close).not.toHaveBeenCalled();
    fire(row, "wheel");
    expect(close).toHaveBeenCalledTimes(1);
  });

  test("Escape closes", () => {
    fire(document, "keydown", { key: "Escape" });
    fire(document, "keydown", { key: "a" });
    expect(close).toHaveBeenCalledTimes(1);
  });

  test("stopping removes every listener", () => {
    stop();
    fire(row, "mousedown");
    fire(document, "keydown", { key: "Escape" });
    fire(row, "wheel");
    expect(close).not.toHaveBeenCalled();
  });
});
