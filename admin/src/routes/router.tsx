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
import { mediaSearch } from "../media/search";
import { postSearch } from "../posts/filters";
import { Analytics } from "./Analytics";
import { AuditLog } from "./AuditLog";
import { Dashboard } from "./Dashboard";
import { Media } from "./Media";
import { MediaDetails } from "./MediaDetails";
import { NewPost } from "./NewPost";
import { Posts } from "./Posts";
import { Redirects } from "./Redirects";
import { SchemaTemplates } from "./SchemaTemplates";
import { SignIn } from "./SignIn";
import { Trash } from "./Trash";

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

// Which files to show lives in the address, like the posts filters.
const media = createRoute({
  getParentRoute: () => signedIn,
  path: "/media",
  validateSearch: mediaSearch,
  component: Media,
});

const mediaItem = createRoute({
  getParentRoute: () => signedIn,
  path: "/media/$id",
  component: MediaDetails,
});

const trash = createRoute({ getParentRoute: () => signedIn, path: "/trash", component: Trash });

const audit = createRoute({ getParentRoute: () => signedIn, path: "/audit", component: AuditLog });

const redirects = createRoute({
  getParentRoute: () => signedIn,
  path: "/redirects",
  component: Redirects,
});

const templates = createRoute({
  getParentRoute: () => signedIn,
  path: "/schema-templates",
  component: SchemaTemplates,
});

const analytics = createRoute({
  getParentRoute: () => signedIn,
  path: "/analytics",
  component: Analytics,
});

const routeTree = root.addChildren([
  signIn,
  signedIn.addChildren([
    dashboard,
    posts,
    newPost,
    editor,
    media,
    mediaItem,
    trash,
    audit,
    redirects,
    templates,
    analytics,
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
