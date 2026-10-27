import { en } from "../strings/en";

/** The first screen after signing in. */
export function Home() {
  return (
    <>
      <h1>{en.home.title}</h1>
      <p className="lede">{en.home.lede}</p>
    </>
  );
}
