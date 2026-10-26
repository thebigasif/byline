// Polish on top of pages that read completely without it (RULES.md BL-WEB-2):
// copy a heading's address, open an enlarged image in a dialog, open search
// with / or Ctrl+K, and mark the section being read in the contents.
// The words it shows come from the script tag's data attributes.
(() => {
  "use strict";
  const words = document.currentScript ? document.currentScript.dataset : {};

  // Says something to screen readers and shows it briefly.
  const status = document.createElement("p");
  status.className = "reader-status";
  status.setAttribute("role", "status");
  document.body.append(status);
  const say = (text) => {
    status.textContent = text;
    setTimeout(() => {
      status.textContent = "";
    }, 3000);
  };

  // A heading's # link copies the section's address and says so.
  document.addEventListener("click", (event) => {
    const link = event.target.closest(".heading-link");
    if (!link || !navigator.clipboard) return;
    event.preventDefault();
    const address = new URL(link.getAttribute("href"), location.href).href;
    navigator.clipboard.writeText(address).then(() => {
      history.replaceState(null, "", link.getAttribute("href"));
      say(words.copied || "");
    });
  });

  // A reduced image opens at full size in a dialog that Escape closes.
  // The browser returns focus to the image's link when it closes.
  document.addEventListener("click", (event) => {
    const link = event.target.closest("a.enlarge");
    if (!link || typeof HTMLDialogElement !== "function") return;
    event.preventDefault();
    const dialog = document.createElement("dialog");
    dialog.className = "lightbox";
    const figure = document.createElement("figure");
    const image = document.createElement("img");
    image.src = link.href;
    image.alt = link.querySelector("img")?.alt || "";
    figure.append(image);
    const caption = link.closest("figure")?.querySelector("figcaption");
    if (caption) {
      const text = document.createElement("figcaption");
      text.textContent = caption.textContent.trim();
      figure.append(text);
    }
    const close = document.createElement("button");
    close.type = "button";
    close.textContent = words.close || "";
    close.addEventListener("click", () => dialog.close());
    dialog.append(close, figure);
    dialog.addEventListener("click", (click) => {
      if (click.target === dialog) dialog.close();
    });
    dialog.addEventListener("close", () => dialog.remove());
    document.body.append(dialog);
    dialog.showModal();
  });

  // / or Ctrl+K or Cmd+K opens search from any page, unless the reader is typing.
  document.addEventListener("keydown", (event) => {
    const typing = event.target.closest("input, textarea, select, [contenteditable]");
    const slash = event.key === "/" && !typing && !event.ctrlKey && !event.metaKey;
    const command = event.key.toLowerCase() === "k" && (event.ctrlKey || event.metaKey);
    if (!slash && !command) return;
    const box = document.querySelector("input[type=search]");
    const page = document.querySelector("a.search-link");
    if (!box && !page) return;
    event.preventDefault();
    if (box) box.focus();
    else location.href = page.href;
  });

  // The contents column marks the section on screen.
  const side = document.querySelector(".toc-side");
  if (side && "IntersectionObserver" in window) {
    const links = new Map(
      [...side.querySelectorAll("a[href^='#']")].map((a) => [a.getAttribute("href").slice(1), a]),
    );
    const seen = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          for (const link of links.values()) link.removeAttribute("aria-current");
          links.get(entry.target.id)?.setAttribute("aria-current", "location");
        }
      },
      { rootMargin: "0px 0px -70% 0px" },
    );
    for (const id of links.keys()) {
      const heading = document.getElementById(id);
      if (heading) seen.observe(heading);
    }
  }
})();
