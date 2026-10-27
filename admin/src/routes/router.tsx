import {
  createBrowserHistory,
  createRootRoute,
  createRoute,
  createRouter,
  type RouterHistory,
} from "@tanstack/react-router";
import { RouteError } from "../components/RouteError";
import { Shell } from "../components/Shell";
import { Home } from "./Home";

const root = createRootRoute({ errorComponent: RouteError });

// Every screen sits inside the shell.
const signedIn = createRoute({
  getParentRoute: () => root,
  id: "signed-in",
  component: Shell,
  errorComponent: RouteError,
});

const home = createRoute({ getParentRoute: () => signedIn, path: "/", component: Home });

const routeTree = root.addChildren([signedIn.addChildren([home])]);

/** Makes the admin's router; a test passes a memory history. */
export function makeRouter(history: RouterHistory = createBrowserHistory()) {
  return createRouter({ routeTree, history, basepath: "/admin" });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof makeRouter>;
  }
}
