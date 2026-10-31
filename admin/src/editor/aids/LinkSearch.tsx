import { type KeyboardEvent as KeyEvent, useEffect, useRef, useState } from "react";
import { usePostSearch } from "../../api/postSearch";
import { isPage, type PostRow } from "../../api/rows";
import { en } from "../../strings/en";
import { AidMenu } from "./AidMenu";
import { type Chosen, isAddress } from "./linkTo";
import { emptyText } from "./PostMenu";
import { menuKey } from "./useMenu";

type Props = {
  chosen: Chosen;
  onPost: (post: PostRow) => void;
  onAddress: (address: string) => void;
  onCancel: () => void;
};

/** Links the selected words to a post found by its title, or to a web address. */
export function LinkSearch({ chosen, onPost, onAddress, onCancel }: Props) {
  const [words, setWords] = useState("");
  const [lit, setLit] = useState(0);
  const address = isAddress(words);
  const found = usePostSearch(address ? "" : words);
  const posts = !address && isPage(found) ? found.rows : [];
  const active = posts.length === 0 ? 0 : Math.min(lit, posts.length - 1);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    input.current?.focus();
  }, []);
  const pick = (index: number) => {
    const post = posts[index];
    if (post !== undefined) {
      onPost(post);
    }
  };
  const onKeyDown = (event: KeyEvent<HTMLInputElement>) => {
    const key = menuKey(event.nativeEvent);
    if (key === "close") {
      event.preventDefault();
      onCancel();
    } else if ((key === "up" || key === "down") && posts.length > 0) {
      event.preventDefault();
      setLit((active + (key === "down" ? 1 : posts.length - 1)) % posts.length);
    }
  };
  const lines = posts.map((post) => ({ id: post.id, name: post.title, hint: post.slug }));
  return (
    <form
      className="focus link-search"
      aria-labelledby="link-search-title"
      onSubmit={(event) => {
        event.preventDefault();
        if (address) {
          onAddress(words);
        } else {
          pick(active);
        }
      }}
    >
      <h2 id="link-search-title">{en.link.title(chosen.words)}</h2>
      <label htmlFor="link-search-words">{en.link.find}</label>
      <span className="hint" id="link-search-hint">
        {address ? en.link.address(words.trim()) : en.link.hint}
      </span>
      <input
        ref={input}
        id="link-search-words"
        aria-describedby="link-search-hint"
        value={words}
        onChange={(event) => setWords(event.target.value)}
        onKeyDown={onKeyDown}
      />
      {address ? null : (
        <AidMenu
          label={en.link.found}
          lines={lines}
          active={active}
          empty={emptyText(found)}
          onPick={pick}
        />
      )}
      <button type="button" className="btn" onClick={onCancel}>
        {en.link.cancel}
      </button>
    </form>
  );
}
