import type { AdminAuthor } from "../api/authors";
import { en } from "../strings/en";
import { languageName } from "../strings/languages";

type Props = { authors: AdminAuthor[] };

/** Every author, one row per language. */
export function AuthorTable({ authors }: Props) {
  const words = en.authors.list;
  return (
    <div className="scroll">
      <table aria-labelledby="authors-title">
        <thead>
          <tr>
            <th scope="col">{words.name}</th>
            <th scope="col">{words.slug}</th>
            <th scope="col">{words.language}</th>
            <th scope="col">{words.role}</th>
          </tr>
        </thead>
        <tbody>
          {authors.map((author) => (
            <tr key={author.id}>
              <td>{author.name}</td>
              <td data-label={words.slug}>
                <code>{author.slug}</code>
              </td>
              <td data-label={words.language}>{languageName(author.language)}</td>
              <td data-label={words.role}>{author.role ?? words.noRole}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
