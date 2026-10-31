import { type FollowUps, openSearches } from "../api/searchConsole";
import { calendarDay } from "../strings/dates";
import { searchConsoleWords } from "../strings/searchConsoleWords";

type Props = { found: FollowUps };

/** The focus of the screen: how many long searches no heading answers yet, and what to do. */
export function OpenSearches({ found }: Props) {
  const words = searchConsoleWords.open;
  const { open, all } = openSearches(found);
  const [first, last] = [calendarDay(found.first), calendarDay(found.last)];
  let count = <p>{words.none(found.words, first, last)}</p>;
  if (open > 0) {
    count = (
      <ul className="stats">
        <li>
          <strong>{words.count(open, all)}</strong> {words.sentence(first, last)}
        </li>
      </ul>
    );
  } else if (all > 0) {
    count = <p>{words.allAnswered(all)}</p>;
  }
  return (
    <section className="focus" aria-labelledby="open-title">
      <h2 id="open-title">{words.title}</h2>
      {count}
      <p className="muted">{words.longMeans(found.words)}</p>
      <p>{words.fix}</p>
    </section>
  );
}
