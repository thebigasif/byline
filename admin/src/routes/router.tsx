import {
  createBrowserHistory,
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
  type RouterHistory,
  redirect,
} from "@tanstack/react-router";
import { isSignedIn } from "../auth/session";
import { RouteError } from "../components/RouteError";
import { Shell } from "../components/Shell";
import { postSearch } from "../posts/filters";
import { Account } from "./Account";
import { AuthorPage } from "./AuthorPage";
import { Authors } from "./Authors";
import { Comments } from "./Comments";
import { Dashboard } from "./Dashboard";
import { Keys } from "./Keys";
import { LinkSignIn } from "./LinkSignIn";
import { NewPost } from "./NewPost";
import { Posts } from "./Posts";
import { Redirects } from "./Redirects";
import { SignIn } from "./SignIn";

const root = createRootRoute({ errorComponent: RouteError });

const signIn = createRoute({ getParentRoute: () => root, path: "/sign-in", component: SignIn });

// Where the link from `khalid staff` or an invitation lands.
const linkSignIn = createRoute({
  getParentRoute: () => root,
  path: "/sign-in/link",
  component: LinkSignIn,
});

// Every screen but sign-in sits inside the shell and needs a key or an email sign-in.
const signedIn = createRoute({
  getParentRoute: () => root,
  id: "signed-in",
  beforeLoad: () => {
    if (!isSignedIn()) {
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

const comments = createRoute({
  getParentRoute: () => signedIn,
  path: "/comments",
  component: Comments,
});

const account = createRoute({
  getParentRoute: () => signedIn,
  path: "/account",
  component: Account,
});

const authors = createRoute({
  getParentRoute: () => signedIn,
  path: "/authors",
  component: Authors,
});

const authorPage = createRoute({
  getParentRoute: () => signedIn,
  path: "/authors/$id",
  component: AuthorPage,
});

const keys = createRoute({ getParentRoute: () => signedIn, path: "/keys", component: Keys });

const routeTree = root.addChildren([
  signIn,
  linkSignIn,
  signedIn.addChildren([
    dashboard,
    posts,
    newPost,
    editor,
    comments,
    authors,
    authorPage,
    redirects,
    keys,
    account,
  ]),
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
