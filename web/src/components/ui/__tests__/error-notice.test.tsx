import { render, screen } from "@testing-library/react";
import { NextIntlClientProvider } from "next-intl";
import { describe, expect, it } from "vitest";
import { ApiError } from "@/lib/api/errors";
import { ErrorNotice } from "../error-notice";
import en from "../../../../messages/en.json";

describe("ErrorNotice", () => {
  it("shows the problem detail and the request id", () => {
    const error = new ApiError(409, { type: "x", title: "Approvals missing", status: 409, detail: "Both must approve.", request_id: "req_123" });
    render(
      <NextIntlClientProvider locale="en" messages={en}>
        <ErrorNotice error={error} />
      </NextIntlClientProvider>,
    );
    expect(screen.getByRole("alert")).toHaveTextContent("Both must approve.");
    expect(screen.getByText("req_123")).toBeInTheDocument();
    expect(error.fieldErrors).toEqual({});
  });
});
