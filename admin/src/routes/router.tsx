import {
  createBrowserHistory,
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
  type RouterHistory,
  redirect,
} from "@tanstack/react-router";
import { readKey } from "../auth/session";
import { RouteError } from "../components/RouteError";
import { Shell } from "../components/Shell";
import { postSearch } from "../posts/filters";
import { Dashboard } from "./Dashboard";
import { LinkSignIn } from "./LinkSignIn";
import { NewPost } from "./NewPost";
import { Posts } from "./Posts";
import { Redirects } from "./Redirects";
import { SignIn } from "./SignIn";
import { Sites } from "./Sites";

const root = createRootRoute({ errorComponent: RouteError });

const signIn = createRoute({ getParentRoute: () => root, path: "/sign-in", component: SignIn });

// A person's own sign-in, by an emailed link, and the one page it opens:
// the super-admin panel needs a session, never a key, so both sit outside the shell.
const linkSignIn = createRoute({
  getParentRoute: () => root,
  path: "/sign-in/link",
  component: LinkSignIn,
});

const sites = createRoute({ getParentRoute: () => root, path: "/sites", component: Sites });

// Every screen but sign-in sits inside the shell and needs a key.
const signedIn = createRoute({
  getParentRoute: () => root,
  id: "signed-in",
  beforeLoad: () => {
    if (readKey() === null) {
      throw redirect({ to: "/sign-in" });
    }
  },
  component: Shell,
  errorComponent: RouteError,
});

const dashboard = createRoute({
  getParentRoute: () => signedIn,
  path: "/",
  component: Dashboard,
});

// The filters live in the address, so a filtered list can be bookmarked and shared.
const posts = createRoute({
  getParentRoute: () => signedIn,
  path: "/posts",
  validateSearch: postSearch,
  component: Posts,
});

const newPost = createRoute({ getParentRoute: () => signedIn, path: "/new", component: NewPost });

// The editor carries Tiptap, so it loads only when a post is opened.
const editor = createRoute({
  getParentRoute: () => signedIn,
  path: "/posts/$id",
  component: lazyRouteComponent(() => import("./Editor"), "Editor"),
});

const redirects = createRoute({
  getParentRoute: () => signedIn,
  path: "/redirects",
  component: Redirects,
});

const routeTree = root.addChildren([
  signIn,
  linkSignIn,
  sites,
  signedIn.addChildren([dashboard, posts, newPost, editor, redirects]),
]);

/** Makes the admin's router; a test passes a memory history. */
export function makeRouter(history: RouterHistory = createBrowserHistory()) {
  return createRouter({ routeTree, history, basepath: "/admin" });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof makeRouter>;
  }
}
