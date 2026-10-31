import { en } from "./en";

const DAY = new Intl.DateTimeFormat(en.locale, { day: "numeric", month: "long", year: "numeric" });

const MOMENT = new Intl.DateTimeFormat(en.locale, { dateStyle: "long", timeStyle: "short" });

/** A moment as the day a reader would say it, such as "12 October 2026", in their time zone. */
export function day(iso: string): string {
  return DAY.format(new Date(iso));
}

/** A moment to the minute, such as "12 October 2026 at 09:12", in the reader's time zone. */
export function moment(iso: string): string {
  return MOMENT.format(new Date(iso));
}
