import { describe, expect, it } from "vitest";
import { readStoredPreference, resolveTheme } from "./theme";

describe("resolveTheme", () => {
  it("returns dark when preference is system and OS prefers dark", () => {
    expect(resolveTheme("system", true)).toBe("dark");
  });

  it("returns light when preference is system and OS prefers light", () => {
    expect(resolveTheme("system", false)).toBe("light");
  });

  it("returns the explicit preference regardless of OS setting", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });
});

describe("readStoredPreference", () => {
  it("returns the stored value when it is a valid explicit preference", () => {
    expect(readStoredPreference(() => "light")).toBe("light");
    expect(readStoredPreference(() => "dark")).toBe("dark");
  });

  it("falls back to system when nothing is stored", () => {
    expect(readStoredPreference(() => null)).toBe("system");
  });

  it("falls back to system when the stored value is not a valid theme", () => {
    expect(readStoredPreference(() => "purple")).toBe("system");
  });
});
