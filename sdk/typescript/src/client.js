// @ts-check
import { operations } from "./operations.js";

/**
 * @typedef {import("./index.js").ClientOptions} ClientOptions
 * @typedef {import("./index.js").Client} Client
 * @typedef {import("./index.js").OperationId} OperationId
 * @typedef {import("./schema.js").Problem} Problem
 * @typedef {{
 *   path?: Readonly<Record<string, string | number>>;
 *   query?: Readonly<Record<string, string | number | boolean | null | undefined>>;
 *   headers?: Readonly<Record<string, string | undefined>>;
 *   body?: unknown;
 * }} Values
 */

/**
 * Makes a client for the site at `options.baseUrl`.
 *
 * @param {ClientOptions} options
 * @returns {Client}
 */
export function createClient(options) {
  const send = options.fetch ?? globalThis.fetch;
  const base = options.baseUrl.replace(/\/+$/, "");
  return {
    /**
     * @template {OperationId} K
     * @param {K} operation
     * @param {import("./index.js").Input<K>} input
     * @returns {Promise<import("./index.js").Result<import("./index.js").Output<K>>>}
     */
    async request(operation, input) {
      /** @type {Values} */
      const values = input;
      const route = operations[operation];
      const url = new URL(`${base}${fill(route.path, values.path ?? {})}`);
      for (const [name, value] of Object.entries(values.query ?? {})) {
        if (value !== undefined && value !== null) url.searchParams.set(name, String(value));
      }
      const headers = new Headers({ Accept: "application/json, application/problem+json" });
      for (const [name, value] of Object.entries(values.headers ?? {})) {
        if (value !== undefined) headers.set(name, value);
      }
      if (route.key === "admin" && options.adminKey !== undefined) {
        headers.set("Authorization", `Khalid ${options.adminKey}`);
      } else if (route.key === "content" && options.contentKey !== undefined) {
        headers.set("Authorization", `Khalid-Content ${options.contentKey}`);
      }
      /** @type {RequestInit} */
      const init = { method: route.method, headers };
      if (route.body !== null && values.body !== undefined) {
        headers.set("Content-Type", route.body);
        init.body =
          route.body === "application/json" ? JSON.stringify(values.body) : body(values.body);
      }
      const answer = await send(url, init);
      const type = answer.headers.get("Content-Type") ?? "";
      if (!answer.ok) {
        return { ok: false, status: answer.status, problem: await problem(answer, type) };
      }
      // The answer's shape is the document's promise; the server is tested against it.
      const data = /** @type {import("./index.js").Output<K>} */ (await read(answer, type));
      return { ok: true, status: answer.status, data };
    },
  };
}

/**
 * The path template with each `{name}` replaced by its encoded value.
 *
 * @param {string} template
 * @param {Readonly<Record<string, string | number>>} values
 * @returns {string}
 */
function fill(template, values) {
  return template.replace(/\{([^}]+)\}/g, (_, name) => {
    const value = values[name];
    if (value === undefined) throw new TypeError(`The path needs ${name}. Pass it in input.path.`);
    return encodeURIComponent(String(value));
  });
}

/**
 * A body that is not JSON, sent as given.
 *
 * @param {unknown} value
 * @returns {BodyInit}
 */
function body(value) {
  if (typeof value === "string" || value instanceof Blob) return value;
  throw new TypeError("This operation takes text or a Blob as its body.");
}

/**
 * The success answer: parsed JSON, text, bytes as a Blob, or null when empty.
 *
 * @param {Response} answer
 * @param {string} type
 * @returns {Promise<unknown>}
 */
async function read(answer, type) {
  if (answer.status === 204) return null;
  if (type.includes("json")) return answer.json();
  if (type.startsWith("text/")) return answer.text();
  return answer.blob();
}

/**
 * The problem the server described, or one made from the status when the
 * answer was not a problem document.
 *
 * @param {Response} answer
 * @param {string} type
 * @returns {Promise<Problem>}
 */
async function problem(answer, type) {
  if (type.includes("json")) return /** @type {Problem} */ (await answer.json());
  const text = await answer.text();
  return {
    type: "about:blank",
    title: answer.statusText || `HTTP ${answer.status}`,
    status: answer.status,
    detail: text,
    fix: "Check the request against openapi.json, then try again.",
  };
}
