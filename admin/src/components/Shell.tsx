import { Link, Outlet, useNavigate } from "@tanstack/react-router";
import { endSession } from "../api/account";
import { forgetKey, readCsrf } from "../auth/session";
import { en } from "../strings/en";
import { searchConsoleWords } from "../strings/searchConsoleWords";

/** The header and side navigation around every signed-in screen. */
export function Shell() {
  const navigate = useNavigate();
  const signOut = async () => {
    // An email sign-in also ends on the server, so its cookie stops working.
    if (readCsrf() !== null) {
      await endSession();
    }
    forgetKey();
    await navigate({ to: "/sign-in" });
  };
  return (
    <>
      <a className="skip" href="#main">
        {en.skip}
      </a>
      <header className="top">
        <div className="brand">{en.brand}</div>
        <div className="flex flex-wrap items-center gap-3">
          <a href="/">{en.viewBlog}</a>
          <Link to="/account">{en.account.nav}</Link>
          <button type="button" className="btn" onClick={signOut}>
            {en.signOut}
          </button>
        </div>
      </header>
      <div className="shell">
        <nav className="side" aria-label={en.nav.label}>
          <ul>
            <li>
              <Link to="/" activeOptions={{ exact: true }}>
                {en.nav.dashboard}
              </Link>
            </li>
            <li>
              <Link to="/posts" search={{}} activeOptions={{ includeSearch: false }}>
                {en.nav.posts}
              </Link>
            </li>
            <li>
              <Link to="/media" search={{}} activeOptions={{ includeSearch: false }}>
                {en.nav.media}
              </Link>
            </li>
            <li>
              <Link to="/comments">{en.nav.comments}</Link>
            </li>
            <li>
              <Link to="/authors">{en.nav.authors}</Link>
            </li>
            <li>
              <Link to="/trash">{en.nav.trash}</Link>
            </li>
            <li>
              <Link to="/audit">{en.nav.audit}</Link>
            </li>
            <li>
              <Link to="/redirects">{en.nav.redirects}</Link>
            </li>
            <li>
              <Link to="/schema-templates">{en.nav.templates}</Link>
            </li>
            <li>
              <Link to="/passkeys">{en.nav.passkeys}</Link>
            </li>
            <li>
              <Link to="/analytics">{en.nav.analytics}</Link>
            </li>
            <li>
              <Link to="/search-console">{searchConsoleWords.title}</Link>
            </li>
            <li>
              <Link to="/keys">{en.nav.keys}</Link>
            </li>
            <li>
              <Link to="/webhooks">{en.nav.webhooks}</Link>
            </li>
            <li>
              <Link to="/team">{en.nav.team}</Link>
            </li>
            <li>
              <Link to="/settings">{en.nav.settings}</Link>
            </li>
          </ul>
        </nav>
        <main id="main" className="page">
          <Outlet />
        </main>
      </div>
    </>
  );
}
