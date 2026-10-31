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
import { NewPost } from "./NewPost";
import { Posts } from "./Posts";
import { Redirects } from "./Redirects";
import { SearchConsole } from "./SearchConsole";
import { SignIn } from "./SignIn";

const root = createRootRoute({ errorComponent: RouteError });

const signIn = createRoute({ getParentRoute: () => root, path: "/sign-in", component: SignIn });

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

const searchConsole = createRoute({
  getParentRoute: () => signedIn,
  path: "/search-console",
  component: SearchConsole,
});

const routeTree = root.addChildren([
  signIn,
  signedIn.addChildren([dashboard, posts, newPost, editor, redirects, searchConsole]),
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
