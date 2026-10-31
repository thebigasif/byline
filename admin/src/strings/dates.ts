import { en } from "./en";

const DAY = new Intl.DateTimeFormat(en.locale, { day: "numeric", month: "long", year: "numeric" });

const MOMENT = new Intl.DateTimeFormat(en.locale, { dateStyle: "long", timeStyle: "short" });

/** A moment as the day a reader would say it, such as "12 October 2026", in their time zone. */
export function day(iso: string): string {
  return DAY.format(new Date(iso));
}

/** A moment with its time, such as "31 October 2026 at 09:30", in the reader's time zone. */
export function moment(iso: string): string {
  return MOMENT.format(new Date(iso));
}

const CALENDAR = new Intl.DateTimeFormat(en.locale, {
  day: "numeric",
  month: "long",
  year: "numeric",
  timeZone: "UTC",
});

/** A calendar day the API sends as `2026-10-31`, written out, such as "31 October 2026". */
export function calendarDay(date: string): string {
  return CALENDAR.format(new Date(`${date}T00:00:00Z`));
}

const COUNT = new Intl.NumberFormat(en.locale);

/** A count written with the separators of this language, such as "1,284". */
export function count(value: number): string {
  return COUNT.format(value);
}
