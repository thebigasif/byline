import { CsvRedirects } from "../redirects/CsvRedirects";
import { KeptRedirects } from "../redirects/KeptRedirects";
import { MissLog } from "../redirects/MissLog";
import { SendToPost } from "../redirects/SendToPost";
import { en } from "../strings/en";

/** Where old addresses lead: the redirects Khalid keeps, and how to add more. */
export function Redirects() {
  return (
    <>
      <h1>{en.redirects.title}</h1>
      <p className="lede">{en.redirects.lede}</p>
      <SendToPost />
      <MissLog />
      <KeptRedirects />
      <CsvRedirects />
    </>
  );
}
