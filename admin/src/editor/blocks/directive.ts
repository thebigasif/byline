// The directive grammar of ADR 0012 and ADR 0018, read and written the way
// `khalid-content` reads it, so a block typed in a file and a block made in
// the editor are the same text.

/** A directive's attributes in the order they were written. */
export type Attributes = [string, string][];

/** The first line of a directive: its run of colons, name and attributes. */
export type Opening = {
  fence: string;
  name: string;
  attributes: Attributes;
};

/**
 * Reads a `::name{...}` or `:::name{...}` line, or `null` when the line is
 * not one or its attributes cannot be read.
 */
export function readOpening(line: string): Opening | null {
  const found = /^(:{2,})([a-z][a-z-]*)(?:\{(.*)\})?[ \t]*$/.exec(line);
  if (found === null) {
    return null;
  }
  const [, fence = "", name = "", inside = ""] = found;
  const attributes = readAttributes(inside);
  return attributes === null ? null : { fence, name, attributes };
}

/** Reads `name=value name="a value"` pairs, or `null` when they cannot be read. */
export function readAttributes(text: string): Attributes | null {
  const found: Attributes = [];
  let rest = text.trim();
  while (rest !== "") {
    const equals = rest.indexOf("=");
    const key = rest.slice(0, equals).trim();
    if (equals < 0 || !/^[A-Za-z0-9-]+$/.test(key)) {
      return null;
    }
    let after = rest.slice(equals + 1);
    let value: string;
    if (after.startsWith('"')) {
      const close = after.indexOf('"', 1);
      if (close < 0) {
        return null;
      }
      value = after.slice(1, close);
      after = after.slice(close + 1);
      if (!(after === "" || after.startsWith(" "))) {
        return null;
      }
    } else {
      const space = after.indexOf(" ");
      value = space < 0 ? after : after.slice(0, space);
      after = space < 0 ? "" : after.slice(space);
    }
    if (/["{}]/.test(value)) {
      return null;
    }
    found.push([key, value]);
    rest = after.trimStart();
  }
  return found;
}

/**
 * Writes a directive's name and attributes in canonical form: the
 * attributes in `order`, each value in double quotes unless it is one word.
 */
export function printOpening(
  name: string,
  order: readonly string[],
  values: Record<string, string | null>,
): string {
  const pairs = order.flatMap((key) => {
    const value = values[key];
    return value === null || value === undefined ? [] : [`${key}=${quoted(value)}`];
  });
  return pairs.length === 0 ? name : `${name}{${pairs.join(" ")}}`;
}

function quoted(value: string): string {
  return /^[A-Za-z0-9-]+$/.test(value) ? value : `"${value}"`;
}

/** Whether two attribute lists hold the same values, in any order. */
export function sameValues(
  attributes: Attributes,
  order: readonly string[],
  values: Record<string, string | null>,
): boolean {
  const written = new Map(attributes);
  return (
    written.size === attributes.length &&
    attributes.every(([key]) => order.includes(key)) &&
    order.every((key) => (written.get(key) ?? null) === (values[key] ?? null))
  );
}
