import { describe, expect, test } from "bun:test";
import { stripFrontmatterFields } from "./stripFrontmatterFields.ts";

describe("stripFrontmatterFields", () => {
  test("removes only the named fields, keeping others and the body untouched", () => {
    const raw = `---
map: vilheim-crace
map-type: ruin
summary: A small chapel.
---

Body text stays exactly as it was.
`;
    const { content, removed } = stripFrontmatterFields(raw, ["map", "map-type", "map-x", "map-y"]);
    expect(removed.sort()).toEqual(["map", "map-type"]);
    expect(content).toBe(`---
summary: A small chapel.
---

Body text stays exactly as it was.
`);
  });

  test("leaves a note with no matching fields completely unchanged", () => {
    const raw = `---\ntitle: Untouched\n---\nBody.\n`;
    const { content, removed } = stripFrontmatterFields(raw, ["map", "map-x"]);
    expect(removed).toEqual([]);
    expect(content).toBe(raw);
  });

  test("leaves a note with no frontmatter block at all unchanged", () => {
    const raw = "Just a body, no frontmatter.\n";
    const { content, removed } = stripFrontmatterFields(raw, ["map"]);
    expect(removed).toEqual([]);
    expect(content).toBe(raw);
  });

  test("produces an empty frontmatter block if every field is removed", () => {
    const raw = `---\nmap: x\nmap-x: 1\n---\nBody.\n`;
    const { content } = stripFrontmatterFields(raw, ["map", "map-x"]);
    expect(content).toBe(`---\n---\nBody.\n`);
  });

  test("preserves quoting and value formatting on kept lines", () => {
    const raw = `---\nmap-color: "#f66151"\nsummary: ""\n---\nBody.\n`;
    const { content } = stripFrontmatterFields(raw, ["map-color"]);
    expect(content).toBe(`---\nsummary: ""\n---\nBody.\n`);
  });
});
