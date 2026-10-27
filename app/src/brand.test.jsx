import { describe, expect, it } from "vitest";
import React from "react";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { Brand } from "./brand";

const asset = (path) =>
  readFileSync(new URL(`../public/${path}`, import.meta.url));

describe("Clawberry identity", () => {
  it("keeps one mark across logo, appearance variants, favicon and app icon", () => {
    const mark = asset("brand/mark.svg").toString();
    const geometry = mark.match(/d="(M12[^"]+)"/)?.[1];
    expect(geometry).toBeTruthy();
    for (const name of [
      "brand/logo.svg",
      "brand/mark-light.svg",
      "brand/icon.svg",
      "favicon.svg",
    ])
      expect(asset(name).toString()).toContain(`d="${geometry}"`);
    expect(asset("brand/icon.svg").toString()).not.toContain("rx=");
  });

  it("ships valid PNG exports at the advertised dimensions", () => {
    for (const size of [16, 32, 180, 192, 512, 1024]) {
      const png = asset(
        size === 180 ? "apple-touch-icon.png" : `brand/icon-${size}.png`,
      );
      expect(png.subarray(0, 8).toString("hex")).toBe("89504e470d0a1a0a");
      expect(png.readUInt32BE(16)).toBe(size);
      expect(png.readUInt32BE(20)).toBe(size);
    }
  });

  it("labels the identity once, without duplicate decorative image announcements", () => {
    const html = renderToStaticMarkup(<Brand />);
    expect(html).toContain("Clawberry");
    expect(html).toContain('alt=""');
    expect(html).not.toContain("TinyClaw");
  });
});
