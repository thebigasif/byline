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
import { mediaSearch } from "../media/search";
import { postSearch } from "../posts/filters";
import { Account } from "./Account";
import { Analytics } from "./Analytics";
import { AuditLog } from "./AuditLog";
import { AuthorPage } from "./AuthorPage";
import { Authors } from "./Authors";
import { Comments } from "./Comments";
import { Dashboard } from "./Dashboard";
import { HandOver } from "./HandOver";
import { Keys } from "./Keys";
import { LinkSignIn } from "./LinkSignIn";
import { Media } from "./Media";
import { MediaDetails } from "./MediaDetails";
import { NewPost } from "./NewPost";
import { Passkeys } from "./Passkeys";
import { Posts } from "./Posts";
import { Redirects } from "./Redirects";
import { SchemaTemplates } from "./SchemaTemplates";
import { SearchConsole } from "./SearchConsole";
import { SignIn } from "./SignIn";
import { Site } from "./Site";
import { Sites } from "./Sites";
import { Team } from "./Team";
import { Trash } from "./Trash";
import { WebhookActivity } from "./WebhookActivity";
import { Webhooks } from "./Webhooks";

const root = createRootRoute({ errorComponent: RouteError });

const signIn = createRoute({ getParentRoute: () => root, path: "/sign-in", component: SignIn });

// Where the link from `khalid staff`, `khalid sites admin` or an invitation lands.
const linkSignIn = createRoute({
  getParentRoute: () => root,
  path: "/sign-in/link",
  component: LinkSignIn,
});

// The super-admin panel needs a session, never a key, so it sits outside the shell.
const sites = createRoute({ getParentRoute: () => root, path: "/sites", component: Sites });

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

const passkeys = createRoute({
  getParentRoute: () => signedIn,
  path: "/passkeys",
  component: Passkeys,
});

const analytics = createRoute({
  getParentRoute: () => signedIn,
  path: "/analytics",
  component: Analytics,
});

const searchConsole = createRoute({
  getParentRoute: () => signedIn,
  path: "/search-console",
  component: SearchConsole,
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

const webhooks = createRoute({
  getParentRoute: () => signedIn,
  path: "/webhooks",
  component: Webhooks,
});

const webhookActivity = createRoute({
  getParentRoute: () => signedIn,
  path: "/webhooks/$id",
  component: WebhookActivity,
});

const team = createRoute({ getParentRoute: () => signedIn, path: "/team", component: Team });

// Its own screen, so the consequence is read before anything is asked.
const handOver = createRoute({
  getParentRoute: () => signedIn,
  path: "/team/owner",
  component: HandOver,
});

// The whole blog's settings; post settings live in the editor.
const site = createRoute({ getParentRoute: () => signedIn, path: "/settings", component: Site });

const routeTree = root.addChildren([
  signIn,
  linkSignIn,
  sites,
  signedIn.addChildren([
    dashboard,
    posts,
    newPost,
    editor,
    media,
    mediaItem,
    comments,
    authors,
    authorPage,
    trash,
    audit,
    redirects,
    templates,
    passkeys,
    analytics,
    searchConsole,
    keys,
    webhooks,
    webhookActivity,
    team,
    handOver,
    site,
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
