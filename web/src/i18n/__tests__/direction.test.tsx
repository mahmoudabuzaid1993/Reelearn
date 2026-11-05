import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { directionFor, isRtl, otherLocale } from "../direction";
import { PhonePreview } from "@/features/studio/phone-preview";
import { NextIntlClientProvider } from "next-intl";
import en from "../../../messages/en.json";

describe("direction", () => {
  it("maps locales and learner languages to a text direction", () => {
    expect(directionFor("ar")).toBe("rtl");
    expect(directionFor("ur")).toBe("rtl");
    expect(directionFor("ar-SA")).toBe("rtl");
    expect(directionFor("en")).toBe("ltr");
    expect(directionFor("bn")).toBe("ltr");
    expect(isRtl("ar")).toBe(true);
    expect(otherLocale("ar")).toBe("en");
    expect(otherLocale("en")).toBe("ar");
  });

  it("switches the phone preview dir per reel language inside any console locale", () => {
    const reel = { title: "t", script: "First line. Second.", duration_ms: 30000, question: null, presenter: {} as never, thumbnail_url: null };
    const { rerender } = render(
      <NextIntlClientProvider locale="en" messages={en}>
        <PhonePreview reel={{ ...reel, language: "ar" }} />
      </NextIntlClientProvider>,
    );
    expect(screen.getByRole("img")).toHaveAttribute("dir", "rtl");
    rerender(
      <NextIntlClientProvider locale="en" messages={en}>
        <PhonePreview reel={{ ...reel, language: "en" }} />
      </NextIntlClientProvider>,
    );
    expect(screen.getByRole("img")).toHaveAttribute("dir", "ltr");
  });
});
