// A post's head as Khalid's theme writes it. React puts the title, meta and
// link tags in the document's <head>, wherever they are rendered; the
// JSON-LD script stays where it is, which search engines read as well.
import { createElement } from "react";
import { type Head, headElements } from "../lib/khalid.js";

export function KhalidHead({ head }: { head: Head }) {
  return (
    <>
      {headElements(head).map((element, index) =>
        createElement(element.type, { key: index, ...element.props }),
      )}
    </>
  );
}
