import { Link, Outlet } from "@tanstack/react-router";
import { en } from "../strings/en";

/** The header and side navigation around every admin screen. */
export function Shell() {
  return (
    <>
      <a className="skip" href="#main">
        {en.skip}
      </a>
      <header className="top">
        <div className="brand">{en.brand}</div>
        <a href="/">{en.viewBlog}</a>
      </header>
      <div className="shell">
        <nav className="side" aria-label={en.nav.label}>
          <ul>
            <li>
              <Link to="/" activeOptions={{ exact: true }}>
                {en.nav.home}
              </Link>
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
