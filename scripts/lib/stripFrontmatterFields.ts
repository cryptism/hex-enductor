const FRONTMATTER_BLOCK = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/;

export interface StripResult {
  content: string;
  removed: string[];
}

/**
 * Removes specific top-level keys from a note's frontmatter block by
 * filtering lines, not by parsing-and-re-serializing the YAML — so a
 * note's field order, quoting style, and comments survive untouched
 * for every field that wasn't removed. Only handles simple `key:
 * value` scalar lines (everything migrated into .hexen.yml is one),
 * not multi-line values.
 */
export function stripFrontmatterFields(raw: string, fieldsToRemove: readonly string[]): StripResult {
  const match = raw.match(FRONTMATTER_BLOCK);
  if (!match) return { content: raw, removed: [] };

  const remove = new Set(fieldsToRemove);
  const removed: string[] = [];
  const lines = match[1]!.split(/\r?\n/);
  const kept = lines.filter((line) => {
    const key = /^([A-Za-z0-9_-]+):/.exec(line)?.[1];
    if (key && remove.has(key)) {
      removed.push(key);
      return false;
    }
    return true;
  });

  const newBlock = `---\n${kept.join("\n")}${kept.length > 0 ? "\n" : ""}---\n`;
  const content = raw.slice(0, match.index) + newBlock + raw.slice(match.index! + match[0].length);
  return { content, removed };
}
