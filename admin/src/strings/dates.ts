import { en } from "./en";

const DAY = new Intl.DateTimeFormat(en.locale, { day: "numeric", month: "long", year: "numeric" });

/** A moment as the day a reader would say it, such as "12 October 2026", in their time zone. */
export function day(iso: string): string {
  return DAY.format(new Date(iso));
}

const MOMENT = new Intl.DateTimeFormat(en.locale, { dateStyle: "long", timeStyle: "short" });

/** A moment with its time, such as "31 October 2026 at 09:30", in the reader's time zone. */
export function moment(iso: string): string {
  return MOMENT.format(new Date(iso));
}
