import { en } from "./en";

const NAMES = new Intl.DisplayNames(en.locale, { type: "language" });

/** A language's name in this language, such as "German" for `de`; the code if unknown. */
export function languageName(code: string): string {
  try {
    return NAMES.of(code) ?? code;
  } catch {
    return code;
  }
}
