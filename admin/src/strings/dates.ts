import { en } from "./en";

const DAY = new Intl.DateTimeFormat(en.locale, { day: "numeric", month: "long", year: "numeric" });

/** A moment as the day a reader would say it, such as "12 October 2026", in their time zone. */
export function day(iso: string): string {
  return DAY.format(new Date(iso));
}
