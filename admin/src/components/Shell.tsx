import { Link, Outlet, useNavigate } from "@tanstack/react-router";
import { forgetKey } from "../auth/session";
import { en } from "../strings/en";

/** The header and side navigation around every signed-in screen. */
export function Shell() {
  const navigate = useNavigate();
  const signOut = () => {
    forgetKey();
    void navigate({ to: "/sign-in" });
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
              <Link to="/redirects">{en.nav.redirects}</Link>
            </li>
            <li>
              <Link to="/serpdad">{en.nav.serpdad}</Link>
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
