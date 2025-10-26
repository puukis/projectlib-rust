import "@testing-library/jest-dom";
import { render } from "@testing-library/svelte/svelte5";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import RunButton from "../RunButton.svelte";

describe("RunButton", () => {
  it("renders run state", () => {
    const { getByText } = render(RunButton, { projectName: "Example", status: "idle" });
    expect(getByText("Run")).toBeInTheDocument();
  });

  it("renders starting state", () => {
    const { getByText } = render(RunButton, { projectName: "Example", status: "starting" });
    expect(getByText("Starting…")).toBeInTheDocument();
  });

  it("renders stop state and reacts to click", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(RunButton, { projectName: "Example", status: "running" });
    const button = getByRole("button", { name: "Stop Example" });
    await user.click(button);
    expect(button).toHaveTextContent("Stop");
  });
});
