import {
  createBrowserHistory,
  createRootRoute,
  createRoute,
  createRouter,
  type RouterHistory,
  redirect,
} from "@tanstack/react-router";
import { readKey } from "../auth/session";
import { RouteError } from "../components/RouteError";
import { Shell } from "../components/Shell";
import { Home } from "./Home";
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

const home = createRoute({ getParentRoute: () => signedIn, path: "/", component: Home });

const routeTree = root.addChildren([signIn, signedIn.addChildren([home])]);

/** Makes the admin's router; a test passes a memory history. */
export function makeRouter(history: RouterHistory = createBrowserHistory()) {
  return createRouter({ routeTree, history, basepath: "/admin" });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof makeRouter>;
  }
}
