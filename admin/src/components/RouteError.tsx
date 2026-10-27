import { en } from "../strings/en";

/** What every route shows when it throws, instead of a blank screen. */
export function RouteError() {
  const { what, why, fix } = en.problems.page;
  return (
    <main id="main" className="page">
      <h1>{what}</h1>
      <p>{why}</p>
      <p>{fix}</p>
    </main>
  );
}
