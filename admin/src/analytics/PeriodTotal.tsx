import { type ChangeEvent, type FormEvent, useState } from "react";
import { type Days, PERIODS, type Visitors } from "../api/analytics";
import { count } from "../strings/dates";
import { en } from "../strings/en";

type Props = { counts: Visitors; days: Days; onDays: (days: Days) => void };

/** The days the select names, or the screen's default when it names none. */
function picked(value: string): Days {
  return PERIODS.find((days) => String(days) === value) ?? 28;
}

/** How the period compares with the one before, in one sentence. */
function change({ views, before, days }: Visitors): string {
  const words = en.analytics;
  if (before === 0) {
    return views === 0 ? words.level(days) : words.fresh(days);
  }
  const percent = Math.round((Math.abs(views - before) / before) * 100);
  if (percent === 0) {
    return words.level(days);
  }
  const say = views > before ? words.rise : words.fall;
  return say(percent, days, count(before));
}

/** The focus of the screen: the page views of the period, and the choice of period. */
export function PeriodTotal({ counts, days, onDays }: Props) {
  const words = en.analytics;
  const [choice, setChoice] = useState<Days>(days);
  const pick = (event: ChangeEvent<HTMLSelectElement>) => setChoice(picked(event.target.value));
  const submit = (event: FormEvent) => {
    event.preventDefault();
    onDays(choice);
  };
  return (
    <section className="focus" aria-labelledby="total-title">
      <h2 id="total-title">{words.total(counts.days)}</h2>
      <p className="big">{count(counts.views)}</p>
      <p>{change(counts)}</p>
      <p className="muted">{words.viewMeans}</p>
      <form className="period" onSubmit={submit}>
        <div className="field">
          <label htmlFor="days">{words.days}</label>
          <select id="days" value={choice} onChange={pick}>
            {PERIODS.map((option) => (
              <option key={option} value={option}>
                {words.period(option)}
              </option>
            ))}
          </select>
        </div>
        <button type="submit" className="btn primary">
          {words.show}
        </button>
      </form>
    </section>
  );
}
